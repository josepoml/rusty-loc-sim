pub mod error;

use std::path::PathBuf;

use error::DeviceError;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::dtservice::DtServiceHandler;
use crate::tunnel::Tunnel;
use crate::usbmux::UsbMuxClient;
use crate::xpc::XpcHandler;

pub struct Device {
    tunnel: Option<Tunnel>,
    device_addr: Option<String>,
    device_port: Option<u16>,
    connection: Option<DtServiceHandler>,
}

impl Device {
    pub fn new() -> Self {
        Device {
            tunnel: None,
            device_addr: None,
            device_port: None,
            connection: None,
        }
    }

    pub async fn connect(
        &mut self,
        wintun_path: PathBuf,
    ) -> Result<
        (
            tokio::task::JoinHandle<()>,
            tokio::task::JoinHandle<()>,
            tokio::task::JoinHandle<()>,
        ),
        DeviceError,
    > {
        let mut usbmux_client = UsbMuxClient::new().await?;
        usbmux_client.get_device_pair_record().await?;
        usbmux_client.connect_to_lockdown().await?;
        usbmux_client.start_lockdown_session().await?;
        usbmux_client.conncet_to_cdp().await?;
        let (mut addr, mut mtu, mut server_addr, mut server_port) =
            usbmux_client.try_cdp_handshake().await?;

        self.device_addr = Some(server_addr);
        self.device_port = Some(server_port);

        self.tunnel = Some(Tunnel::new(addr, mtu, wintun_path));

        let (mut reader, mut writer) = tokio::io::split(
            usbmux_client
                .ssl_sock
                .take()
                .ok_or_else(|| DeviceError::Error("No ssl sock in usbmux client"))?,
        );

        let (sock_read_handle, tun_read_handle, writer_handle) =
            self.tunnel.as_mut().unwrap().on(reader, writer).await;

        Ok((sock_read_handle, tun_read_handle, writer_handle))
    }

    async fn get_dt_service_port(&self) -> Result<u16, DeviceError> {
        let (addr, port) = (
            self.device_addr
                .as_ref()
                .ok_or(DeviceError::Error("Missing device addr"))?,
            self.device_port
                .as_ref()
                .ok_or(DeviceError::Error("Missing device port"))?,
        );
        let mut xpc_handler = XpcHandler::new(&addr, &port).await;
        xpc_handler.do_handshake().await?;

        let dt_port = xpc_handler
            .dtport
            .ok_or(DeviceError::Error("Missing dt port"))?;
        Ok(dt_port)
    }

    pub async fn simulate_location(&mut self, lat: f64, lng: f64) -> Result<(), DeviceError> {
        let dt_addr = self
            .device_addr
            .as_ref()
            .ok_or(DeviceError::Error("No addr"))?;
        let dt_port = self.get_dt_service_port().await?;
        let mut dt_service_handler = DtServiceHandler::new(dt_addr, &dt_port).await?;

        dt_service_handler.start_channel(String::from("")).await?;

        dt_service_handler.simulate_location(lat, lng).await?;

        self.connection = Some(dt_service_handler);

        Ok(())
    }

    pub async fn reveal_developer_mode(&mut self) -> Result<(), DeviceError> {
        let mut usbmux_client = UsbMuxClient::new().await?;
        usbmux_client.get_device_pair_record().await?;
        usbmux_client.connect_to_lockdown().await?;
        usbmux_client.start_lockdown_session().await?;
        usbmux_client.connect_to_amfi().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_device() {
        let mut device = Device::new();
    }
}
