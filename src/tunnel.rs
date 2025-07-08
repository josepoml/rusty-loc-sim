use std::{
    io::Write,
    net::{IpAddr, Ipv6Addr},
    os::windows::process::CommandExt,
    path::PathBuf,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
    sync::mpsc::UnboundedReceiver,
};

use log::error;
use std::process::Command;
use tokio_rustls::client::TlsStream;
use wintun::{Session, Wintun};

use crate::usbmux::UsbMuxClient;

const IPV6_HEADER_SIZE: usize = 40;
const LOOPBACK_HEADER: [u8; 4] = [0x00, 0x00, 0x86, 0xdd];

pub struct Tunnel {
    pub wintun: Arc<Session>,
    termination_token: Arc<RwLock<bool>>,
}

impl Tunnel {
    pub fn new(ipv6: String, mtu: u32, wintun_path: PathBuf) -> Tunnel {
        let tun = unsafe { wintun::load_from_path(wintun_path) }.unwrap();
        let adapter = wintun::Adapter::create(&tun, "wintun", "smt", None).unwrap();
        let session = Arc::new(adapter.start_session(wintun::MAX_RING_CAPACITY).unwrap());
        Command::new("netsh")
            .arg("interface")
            .arg("ipv6")
            .arg("set")
            .arg("address")
            .arg("interface=\"wintun\"")
            .arg(format!("address={}/64", ipv6))
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        Command::new("netsh")
            .arg("interface")
            .arg("ipv6")
            .arg("set")
            .arg("subinterface")
            .arg("interface=\"wintun\"")
            .arg(format!("mtu={}", mtu))
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        thread::sleep(Duration::from_millis(2000));
        Tunnel {
            wintun: session,
            termination_token: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn on(
        &mut self,
        mut reader: ReadHalf<TlsStream<TcpStream>>,
        mut writer: WriteHalf<TlsStream<TcpStream>>,
    ) -> (
        tokio::task::JoinHandle<()>,
        tokio::task::JoinHandle<()>,
        UnboundedReceiver<String>,
    ) {
        let mut guard = self.termination_token.write().unwrap();
        *guard = false;

        let read_session = Arc::clone(&self.wintun);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let tx2 = tx.clone();

        // Channel for passing packets from blocking to async writer
        let (packet_tx, mut packet_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
        let tt1 = self.termination_token.clone();
        let tx_blocking = tx.clone();
        // Blocking task: read from wintun, send to async writer
        let tun_read_handle = tokio::task::spawn_blocking(move || {
            loop {
                if *(tt1.read().unwrap()) {
                    break;
                }
                match read_session.receive_blocking() {
                    Ok(mut packet) => {
                        let bytes = packet.bytes_mut();
                        let ip_version = (bytes[0] >> 4) & 0x0f;
                        if ip_version == 6 {
                            // Send bytes to async writer
                            if packet_tx.send(bytes.to_vec()).is_err() {
                                break;
                            }
                        }
                    }
                    Err(_error) => {
                        let _ = tx_blocking.send("Reader session error".to_string());
                        break;
                    }
                }
            }
        });

        // Async task: receive bytes and write to writer
        let tt1_writer = self.termination_token.clone();
        let tx_writer = tx.clone();
        let writer_handle = tokio::spawn(async move {
            while let Some(bytes) = packet_rx.recv().await {
                if *(tt1_writer.read().unwrap()) {
                    break;
                }
                if let Err(_error) = writer.write_all(&bytes).await {
                    let _ = tx_writer.send("Writer error".to_string());
                    let mut termination_token = tt1_writer.write().unwrap();
                    *termination_token = true;
                    break;
                }
            }
        });

        let tt2 = self.termination_token.clone();

        // Handle network -> tunnel
        let write_session = Arc::clone(&self.wintun);
        let sock_read_handle = tokio::task::spawn(async move {
            let mut ipv6_header = [0u8; IPV6_HEADER_SIZE];
            loop {
                if *(tt2.read().unwrap()) {
                    break;
                }
                let mut is_error = false;
                tokio::select! {
                _ = async {
                 match reader.read_exact(&mut ipv6_header).await {
                    Ok(_) => {

                    },
                    Err(error) => {
                        tx2.send("Reader error".to_string());
                        is_error = true;
                        let mut termination_token = tt2.write().unwrap();
                                        *termination_token = true;
                        return ;
                    }
                 }
                let ipv6_length = u16::from_be_bytes([ipv6_header[4], ipv6_header[5]]) as usize;

                let mut ipv6_body = vec![0u8; ipv6_length];
                match reader.read_exact(&mut ipv6_body).await {
                    Ok(_) => {

                    },
                    Err(error) => {
                        tx2.send("Reader error".to_string());
                        is_error = true;
                        let mut termination_token = tt2.write().unwrap();
                                        *termination_token = true;
                        return
                    }
                }

                let full_packet = [&ipv6_header[..], &ipv6_body[..]].concat();

                // Allocate and send packet correctly
                let mut send_pkt = write_session
                    .allocate_send_packet(full_packet.len() as u16)
                    .expect("Packet allocation failed");
                send_pkt.bytes_mut().copy_from_slice(&full_packet);
                write_session.send_packet(send_pkt);

                } => {},
                _ = tokio::time::sleep(Duration::from_secs(2)) => {
                      if *(tt2.read().unwrap()) {
                    break;
                }
                }
                        }
                if (is_error) {
                    break;
                }
            }
        });

        (tun_read_handle, sock_read_handle, rx)
    }

    pub fn terminate(&mut self) {
        let mut guard = self.termination_token.write().unwrap();
        *guard = true;
    }
}
