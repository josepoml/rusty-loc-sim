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

    // Spawns a blocking task to read from Wintun and forward IPv6 packets to the async writer.
    fn spawn_tun_reader(
        &self,
        read_session: Arc<Session>,
        termination_token: Arc<RwLock<bool>>,
        packet_tx: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::task::spawn_blocking(move || {
            loop {
                // Check for termination
                if *(termination_token.read().unwrap()) {
                    break;
                }
                let mut packet = read_session.receive_blocking().unwrap();

                let bytes = packet.bytes_mut();
                let ip_version = (bytes[0] >> 4) & 0x0f;
                // Only forward IPv6 packets
                if ip_version == 6 {
                    if packet_tx.send(bytes.to_vec()).is_err() {
                        break;
                    }
                }
            }
        })
    }

    // Spawns an async task to receive bytes and write to the network writer.
    fn spawn_writer_task(
        &self,
        mut writer: WriteHalf<TlsStream<TcpStream>>,
        termination_token: Arc<RwLock<bool>>,
        mut packet_rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(bytes) = packet_rx.recv().await {
                if *(termination_token.read().unwrap()) {
                    break;
                }
                writer.write_all(&bytes).await.unwrap();
            }
        })
    }

    pub async fn on(
        &mut self,
        mut reader: ReadHalf<TlsStream<TcpStream>>,
        mut writer: WriteHalf<TlsStream<TcpStream>>,
    ) -> (
        tokio::task::JoinHandle<()>,
        tokio::task::JoinHandle<()>,
        tokio::task::JoinHandle<()>,
    ) {
        // Reset termination token
        *self.termination_token.write().unwrap() = false;

        let read_session = Arc::clone(&self.wintun);

        let (packet_tx, packet_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

        // Spawn tasks
        let tun_read_handle =
            self.spawn_tun_reader(read_session, self.termination_token.clone(), packet_tx);

        let writer_handle =
            self.spawn_writer_task(writer, self.termination_token.clone(), packet_rx);

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
                 reader.read_exact(&mut ipv6_header).await.unwrap();
                let ipv6_length = u16::from_be_bytes([ipv6_header[4], ipv6_header[5]]) as usize;

                let mut ipv6_body = vec![0u8; ipv6_length];
                reader.read_exact(&mut ipv6_body).await.unwrap();

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
            }
        });

        (sock_read_handle, tun_read_handle, writer_handle)
    }

    pub fn terminate(&mut self) {
        let mut guard = self.termination_token.write().unwrap();
        *guard = true;
    }
}
