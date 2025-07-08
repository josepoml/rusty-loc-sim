#![allow(warnings)]

pub mod device;
mod dtservice;
mod tunnel;
mod usbmux;
mod xpc;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::device::Device;

    // #[tokio::test]
    // async fn wintun() {
    //     let mut usbmux_client = UsbMuxClient::new().await.unwrap();
    //     usbmux_client.get_device_pair_record().await.unwrap();
    //     usbmux_client.connect_to_lockdown().await.unwrap();
    //     usbmux_client.start_lockdown_session().await.unwrap();
    //     usbmux_client.conncet_to_cdp().await.unwrap();
    //     let (mut addr, mut mtu, mut server_addr, mut server_port) =
    //         usbmux_client.try_cdp_handshake().await.unwrap();

    //     let mut tun = Tunnel::new(addr, mtu);

    //     let (mut reader, mut writer) = tokio::io::split(usbmux_client.ssl_sock.take().unwrap());
    //     println!("{server_addr}:{server_port}");

    //     let (writer_task, tun_read_task, sock_read_task) = tun.on(reader, writer).await;

    //     tokio::time::sleep(Duration::from_secs(10)).await;

    //     let mut xpc_handler = XpcHandler::new(&server_addr, &server_port).await;

    //     xpc_handler.do_handshake().await.unwrap();

    //     tun.terminate();
    //     let mut dth: DtServiceHandler;
    //     if let Some(port) = xpc_handler.dtport {
    //         println!("{}", port);
    //         dth = DtServiceHandler::new(&server_addr, &port).await.unwrap();
    //         dth.do_handshake().await.unwrap();
    //         dth.start_channel("".to_string()).await.unwrap();
    //         dth.simulate_location().await.unwrap();
    //     }

    //     tokio::time::sleep(Duration::from_secs(50)).await;
    // }

    // async fn apptest() {
    //     let mut device = Device::new().await;
    //     device.connect().await;
    //     device.simulate_location(19.25010, -99.57864).await.unwrap();
    //     tokio::time::sleep(Duration::from_secs(50)).await;
    // }
}
