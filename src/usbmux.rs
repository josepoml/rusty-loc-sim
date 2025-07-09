// #fix
pub mod errors;
pub mod message;
mod ssl2;
mod usbmuxsock;
use byteorder::{BigEndian, LittleEndian};
use errors::{MessageOperationError, UsbmuxOperationError};
use message::{
    LockdownMessage, UsbMuxPlist, UsbmuxMessage, UsbmuxMessageData, UsbmuxMessageHeader,
    USBMUX_MSGTYPE, USBMUX_VERSION,
};
use tokio_rustls::client::TlsStream;

use plist::{to_writer_xml, Value};
use serde_json::json;
use ssl2::ssl_wrap_socket;
use std::{
    io::{Cursor, Read, Write},
    result,
};
use usbmuxsock::UsbmuxSock;

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub struct UsbMuxClient {
    pub sock: Option<UsbmuxSock>,
    pub ssl_sock: Option<TlsStream<TcpStream>>,

    pub device_id: Option<u16>,
    pub device_serial: Option<String>,
    pub system_buid: Option<String>,
    pub host_id: Option<String>,
    pub host_cert: Option<Box<[u8]>>,
    pub host_key: Option<Box<[u8]>>,
}

impl UsbMuxClient {
    pub async fn new() -> Result<Self, UsbmuxOperationError> {
        let sock = UsbmuxSock::new().await?;
        Ok(UsbMuxClient {
            sock: Some(sock),
            ssl_sock: None,

            device_id: None,
            device_serial: None,
            system_buid: None,
            host_id: None,
            host_cert: None,
            host_key: None,
        })
    }

    pub async fn send_usbmux_message(
        &mut self,
        msg: &UsbmuxMessage,
    ) -> Result<(), MessageOperationError> {
        let plist_data = Value::Dictionary(msg.data.to_plist());
        let mut plist_msg: Vec<u8> = Vec::new();
        to_writer_xml(&mut plist_msg, &plist_data)?;
        let mut payload = Vec::new();

        byteorder::WriteBytesExt::write_u32::<LittleEndian>(&mut payload, msg.header.version)?;
        byteorder::WriteBytesExt::write_u32::<LittleEndian>(&mut payload, msg.header.message)?;
        byteorder::WriteBytesExt::write_u32::<LittleEndian>(&mut payload, msg.header.tag)?;

        payload.extend_from_slice(&plist_msg);

        let total_length = payload.len() as u32 + 4;
        let mut request = Vec::new();
        byteorder::WriteBytesExt::write_u32::<LittleEndian>(&mut request, total_length)?;
        request.extend_from_slice(&payload);

        self.sock
            .as_mut()
            .ok_or(MessageOperationError::MissingStream)?
            .sock
            .write_all(&request)
            .await?;

        Ok(())
    }

    pub async fn read_usbmux_response(&mut self) -> Result<Value, MessageOperationError> {
        let total_length = self
            .sock
            .as_mut()
            .ok_or(MessageOperationError::MissingStream)?
            .sock
            .read_u32_le()
            .await?;

        if total_length < 4 {
            return Err(MessageOperationError::ResponseError);
        }

        let payload_length = total_length - 4;
        let mut response_payload = vec![0u8; payload_length as usize];
        self.sock
            .as_mut()
            .ok_or(MessageOperationError::MissingStream)?
            .sock
            .read_exact(&mut response_payload)
            .await?;
        let mut cursor = Cursor::new(response_payload);
        let _resp_version = byteorder::ReadBytesExt::read_u32::<LittleEndian>(&mut cursor)?;
        let _resp_message = byteorder::ReadBytesExt::read_u32::<LittleEndian>(&mut cursor)?;
        let _resp_tag = byteorder::ReadBytesExt::read_u32::<LittleEndian>(&mut cursor)?;
        let mut plist_bytes = Vec::new();
        std::io::Read::read_to_end(&mut cursor, &mut plist_bytes)?;
        let plist_val = plist::Value::from_reader_xml(plist_bytes.as_slice())?;

        Ok(plist_val)
    }

    pub async fn send_lockdown_message(
        &mut self,
        msg: &LockdownMessage,
        ssl: bool,
    ) -> Result<(), MessageOperationError> {
        let dict = msg.to_plist();

        let mut plist_msg: Vec<u8> = Vec::new();
        to_writer_xml(&mut plist_msg, &Value::Dictionary(dict))?;

        let total_length = plist_msg.len() as u32;
        let mut request = Vec::new();
        request.extend_from_slice(&total_length.to_be_bytes());
        request.extend_from_slice(&plist_msg);

        if ssl {
            let sock = self
                .ssl_sock
                .as_mut()
                .ok_or(MessageOperationError::MissingStream)?;
            sock.write_all(&request).await?;
        } else {
            self.sock
                .as_mut()
                .ok_or(MessageOperationError::MissingStream)?
                .sock
                .write_all(&request)
                .await?;
        }

        Ok(())
    }

    pub async fn read_lockdown_response(
        &mut self,
        ssl: bool,
    ) -> Result<Value, MessageOperationError> {
        let mut length_data = [0u8; 4];
        let sock: &mut (dyn AsyncRead + Unpin + Send) = if ssl {
            self.ssl_sock
                .as_mut()
                .ok_or(MessageOperationError::MissingStream)?
        } else {
            &mut self
                .sock
                .as_mut()
                .ok_or(MessageOperationError::MissingStream)?
                .sock
        };

        sock.read_exact(&mut length_data).await?;

        let payload_len = u32::from_be_bytes(length_data) as usize;

        let mut payload = vec![0u8; payload_len];

        sock.read_exact(&mut payload).await?;

        let value = Value::from_reader_xml(&payload[..])?;
        Ok(value)
    }

    pub async fn list_devices(&mut self) -> Result<(), UsbmuxOperationError> {
        let msg = UsbmuxMessage {
            header: UsbmuxMessageHeader {
                version: USBMUX_VERSION,
                message: USBMUX_MSGTYPE,
                tag: 1,
            },
            data: UsbmuxMessageData {
                MessageType: "ListDevices".to_string(),
                PairRecordID: None,
                ClientVersionString: "usbmuxd-client".to_string(),
                ProgName: "client".to_string(),
                kLibUSBMuxVersion: 3,
                DeviceID: None,
                PortNumber: None,
            },
        };
        self.send_usbmux_message(&msg).await?;
        let plist_val = self.read_usbmux_response().await?;
        self.device_serial = match &plist_val {
            Value::Dictionary(dict) => dict
                .get("DeviceList")
                .and_then(|dl| dl.as_array())
                .and_then(|arr| arr.first())
                .and_then(|device| device.as_dictionary())
                .and_then(|info| {
                    self.device_id = info
                        .get("DeviceID")
                        .and_then(|val| val.as_unsigned_integer())
                        .and_then(|num| u16::try_from(num).ok());
                    info.get("Properties")
                })
                .and_then(|props| props.as_dictionary())
                .and_then(|props| props.get("SerialNumber"))
                .and_then(|serial| serial.as_string())
                .map(|s| s.to_string()),
            _ => panic!("Unexpected response format"),
        };

        Ok(())
    }

    pub async fn get_device_pair_record(&mut self) -> Result<(), UsbmuxOperationError> {
        self.list_devices().await?;

        let msg = UsbmuxMessage {
            header: UsbmuxMessageHeader {
                version: USBMUX_VERSION,
                message: USBMUX_MSGTYPE,
                tag: 1,
            },
            data: UsbmuxMessageData {
                MessageType: "ReadPairRecord".to_string(),
                PairRecordID: self.device_serial.clone(),
                ClientVersionString: "usbmuxd-client".to_string(),
                ProgName: "client".to_string(),
                kLibUSBMuxVersion: 3,
                DeviceID: None,
                PortNumber: None,
            },
        };
        self.send_usbmux_message(&msg).await?;
        let plist_val = self.read_usbmux_response().await?;

        if let Value::Dictionary(dict) = plist_val {
            if let Some(pair_record_data) = dict.get("PairRecordData").and_then(|v| v.as_data()) {
                let decoded_data = plist::Value::from_reader_xml(pair_record_data)?;
                if let Value::Dictionary(data_dict) = decoded_data {
                    self.system_buid = data_dict
                        .get("SystemBUID")
                        .and_then(|buid| buid.as_string())
                        .map(|s| s.to_string());
                    self.host_id = data_dict
                        .get("HostID")
                        .and_then(|hostid| hostid.as_string())
                        .map(|s| s.to_string());
                    self.host_cert = data_dict
                        .get("HostCertificate")
                        .and_then(|cert| cert.as_data())
                        .map(|d| d.to_owned().into_boxed_slice());
                    self.host_key = data_dict
                        .get("HostPrivateKey")
                        .and_then(|cert| cert.as_data())
                        .map(|d| d.to_owned().into_boxed_slice());
                }
            }
            Ok(())
        } else {
            Err(UsbmuxOperationError::ParseError)
        }
    }

    pub async fn start_lockdown_session(&mut self) -> Result<(), UsbmuxOperationError> {
        let msg = LockdownMessage {
            Label: Some("client".to_string()),
            Request: Some("StartSession".to_string()),
            Service: None,
            HostID: Some(
                self.host_id
                    .clone()
                    .ok_or_else(|| UsbmuxOperationError::MissingArguments("HostID"))?
                    .to_uppercase(),
            ),
            SystemBUID: Some(
                self.system_buid
                    .clone()
                    .ok_or_else(|| UsbmuxOperationError::MissingArguments("SystemBUID"))?,
            ),
            action: None,
            Domain: None,
            Key: None,
        };

        self.send_lockdown_message(&msg, false).await?;
        let response = self.read_lockdown_response(false).await?;
        Ok(())
    }

    pub async fn connect_to_lockdown(&mut self) -> Result<(), UsbmuxOperationError> {
        self.connect_to_service(62078).await?;
        Ok(())
    }

    pub async fn connect_to_service(
        &mut self,
        service_port: u16,
    ) -> Result<(), UsbmuxOperationError> {
        let msg = UsbmuxMessage {
            header: UsbmuxMessageHeader {
                version: USBMUX_VERSION,
                message: USBMUX_MSGTYPE,
                tag: 1,
            },
            data: UsbmuxMessageData {
                MessageType: "Connect".to_string(),
                PairRecordID: None,
                ClientVersionString: "usbmuxd-client".to_string(),
                ProgName: "client".to_string(),
                kLibUSBMuxVersion: 3,
                DeviceID: self.device_id,
                PortNumber: Some(service_port.to_be()),
            },
        };

        self.send_usbmux_message(&msg).await?;
        let response = self.read_usbmux_response().await?;
        Ok(())
    }

    pub async fn ssl_lockdown_request(
        &mut self,
        msg: LockdownMessage,
    ) -> Result<u16, UsbmuxOperationError> {
        self.try_ssl_handshake().await?;

        self.send_lockdown_message(&msg, true).await?;
        let response = self.read_lockdown_response(true).await?;
        let port = response
            .as_dictionary()
            .and_then(|dict| dict.get("Port"))
            .and_then(|port| port.as_unsigned_integer())
            .and_then(|unsigned| Some(unsigned as u16))
            .ok_or_else(|| UsbmuxOperationError::ParseError)?;
        Ok(port)
    }

    pub async fn try_ssl_handshake(&mut self) -> Result<(), UsbmuxOperationError> {
        let cert = self
            .host_cert
            .as_ref()
            .ok_or_else(|| UsbmuxOperationError::MissingArguments("cert"))?;

        let key = self
            .host_key
            .as_ref()
            .ok_or_else(|| UsbmuxOperationError::MissingArguments("key"))?;

        let ssl_sock = ssl_wrap_socket(
            self.sock
                .take()
                .ok_or_else(|| UsbmuxOperationError::Error("missing ssl socket".to_string()))?
                .sock,
            &cert,
            &key,
        )
        .await?;

        self.ssl_sock = Some(ssl_sock);

        Ok(())
    }

    pub async fn conncet_to_cdp(&mut self) -> Result<(), UsbmuxOperationError> {
        let msg = LockdownMessage {
            Label: Some("client".to_string()),
            Request: Some("StartService".to_string()),
            Service: Some("com.apple.internal.devicecompute.CoreDeviceProxy".to_string()),
            HostID: None,
            SystemBUID: None,
            action: None,
            Domain: None,
            Key: None,
        };
        let port = self.ssl_lockdown_request(msg).await?;
        let msg = LockdownMessage {
            Label: None,
            Request: Some("GetValue".to_string()),
            Service: None,
            HostID: None,
            SystemBUID: None,
            action: None,
            Domain: Some("com.apple.security.mac.amfi".to_string()),
            Key: Some("DeveloperModeStatus".to_string()),
        };
        self.send_lockdown_message(&msg, true).await?;
        let response = self.read_lockdown_response(true).await?;
        let dev_status = response
            .as_dictionary()
            .and_then(|dict| dict.get("Value"))
            .and_then(|value| value.as_boolean())
            .ok_or_else(|| UsbmuxOperationError::ParseError)?;
        if (!dev_status) {
            return Err(UsbmuxOperationError::Error(
                "Developer mode disabled".to_string(),
            ));
        }
        self.sock = Some(UsbmuxSock::new().await?);
        self.connect_to_service(port).await?;
        Ok(())
    }

    pub async fn try_cdp_handshake(
        &mut self,
    ) -> Result<(String, u32, String, u16), UsbmuxOperationError> {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, Debug)]
        struct Response {
            clientParameters: CP,
            serverAddress: String,
            serverRSDPort: u32,
            r#type: String,
        }
        #[derive(Serialize, Deserialize, Debug)]
        struct CP {
            address: String,
            mtu: u32,
            netmask: String,
        }

        self.try_ssl_handshake().await?;

        let sock = self
            .ssl_sock
            .as_mut()
            .ok_or(MessageOperationError::MissingStream)?;

        let magic = b"CDTunnel";

        let data = json!({
            "type": "clientHandshakeRequest",
            "mtu": 16000
        });
        let body = serde_json::to_vec(&data)?;

        let body_length = body.len() as u16;

        let mut packet = Vec::new();
        std::io::Write::write_all(&mut packet, magic)?;
        std::io::Write::write_all(&mut packet, &body_length.to_be_bytes())?;
        std::io::Write::write_all(&mut packet, &body)?;

        sock.write_all(&packet).await?;

        let mut header = vec![0u8; magic.len() + 2];
        sock.read_exact(&mut header).await?;

        if &header[..magic.len()] != magic {
            panic!("No magic bytes")
        }

        let mut cursor = Cursor::new(&header[magic.len()..]);
        let payload_length = byteorder::ReadBytesExt::read_u16::<BigEndian>(&mut cursor)?; //cursor.read_u16::<BigEndian>()?;

        let mut payload_bytes = vec![0u8; payload_length as usize];
        sock.read_exact(&mut payload_bytes).await?;

        let parsed: Response = serde_json::from_slice(&payload_bytes)?;

        Ok((
            parsed.clientParameters.address,
            parsed.clientParameters.mtu,
            parsed.serverAddress,
            parsed.serverRSDPort as u16,
        ))
    }

    pub async fn connect_to_amfi(&mut self) -> Result<(), UsbmuxOperationError> {
        let msg = LockdownMessage {
            Label: Some("client".to_string()),
            Request: Some("StartService".to_string()),
            Service: Some("com.apple.amfi.lockdown".to_string()),
            HostID: None,
            SystemBUID: None,
            action: None,
            Domain: None,
            Key: None,
        };
        let port = self.ssl_lockdown_request(msg).await?;
        self.sock = Some(UsbmuxSock::new().await?);
        self.connect_to_service(port).await?;
        self.try_ssl_handshake().await?;
        let msg = LockdownMessage {
            Label: None,
            Request: None,
            Service: None,
            HostID: None,
            SystemBUID: None,
            action: Some(0),
            Domain: None,
            Key: None,
        };
        self.send_lockdown_message(&msg, true).await?;

        Ok(())
    }
}
