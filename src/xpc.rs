pub mod errors;

use errors::{HandshakeError, ParseError, ReceiveFrameError, SendFrameError, XpcError};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub struct XpcHandler {
    pub sock: TcpStream,
    pub dtport: Option<u16>,
}

impl XpcHandler {
    pub async fn new(server_addr: &String, server_port: &u16) -> Self {
        let sock = TcpStream::connect(format!("[{}]:{}", server_addr, server_port))
            .await
            .unwrap();
        XpcHandler {
            sock: sock,
            dtport: None,
        }
    }

    pub async fn do_handshake(&mut self) -> Result<(), XpcError> {
        self.send_frames().await;
        let response_frame = self.receive_frames().await?;
        self.dtport = Some(self.get_dvt_port(response_frame)?);
        Ok(())
    }

    async fn send_frames(&mut self) -> Result<(), SendFrameError> {
        self.sock
            .write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n")
            .await?;

        // Settings frame (non-ACK)
        let settings_frame = [
            0x00, 0x00, 0x0C, // Length = 12
            0x04, // Type = SETTINGS
            0x00, // Flags = 0
            0x00, 0x00, 0x00, 0x00, // conn ID = 0
            0x00, 0x03, // MAX_CONCURRENT_connS
            0x00, 0x00, 0x00, 0x64, // Value = 100
            0x00, 0x04, // INITIAL_WINDOW_SIZE
            0x00, 0x10, 0x00, 0x00, // Value = 1048576
        ];
        self.sock.write_all(&settings_frame).await?;

        // WindowUpdate frame
        let window_update = [
            0x00, 0x00, 0x04, // Length = 4
            0x08, // Type = WINDOW_UPDATE
            0x00, // Flags = 0
            0x00, 0x00, 0x00, 0x00, // conn ID = 0
            0x00, 0x0F, 0x00, 0x01, // Increment = 983041
        ];
        self.sock.write_all(&window_update).await?;

        // Headers frame (ROOT_CHANNEL)
        let root_headers = [
            0x00, 0x00, 0x00, // Length = 0
            0x01, // Type = HEADERS
            0x04, // Flags = END_HEADERS
            0x00, 0x00, 0x00, 0x01, // conn ID = 1
        ];
        self.sock.write_all(&root_headers).await?;

        // Data frame (ROOT_CHANNEL)
        let mut data_frame_1 = vec![
            0x00, 0x00, 0x2C, // Length = 44
            0x00, // Type = DATA
            0x00, // Flags = 0
            0x00, 0x00, 0x00, 0x01, // conn ID = 1
        ];
        let data_payload = hex::decode("920bb0290100000014000000000000000000000000000000423713420500000000f000000400000000000000")?;
        data_frame_1.extend(data_payload);

        self.sock.write_all(&data_frame_1).await?;

        let mut data_frame_2 = vec![
            0x00, 0x00, 0x18, // Length = 24
            0x00, // Type = DATA
            0x00, // Flags = 0
            0x00, 0x00, 0x00, 0x01, // conn ID = 1
        ];

        let data_frame_payload_2 = hex::decode("920bb0290102000000000000000000000000000000000000")?;
        data_frame_2.extend(data_frame_payload_2);

        self.sock.write_all(&data_frame_2).await?;

        // Headers frame (REPLY_CHANNEL)
        let reply_headers = [
            0x00, 0x00, 0x00, // Length = 0
            0x01, // Type = HEADERS
            0x04, // Flags = END_HEADERS
            0x00, 0x00, 0x00, 0x03, // conn ID = 3
        ];
        self.sock.write_all(&reply_headers).await?;

        let mut data_frame_3 = vec![
            0x00, 0x00, 0x18, // Length = 24
            0x00, // Type = DATA
            0x00, // Flags = 0
            0x00, 0x00, 0x00, 0x03, // conn ID = 1
        ];

        let data_frame_payload_3 = hex::decode("920bb0290100400000000000000000000000000000000000")?;
        data_frame_3.extend(data_frame_payload_3);

        self.sock.write_all(&data_frame_3).await?;
        Ok(())
    }

    async fn receive_frames(&mut self) -> Result<Vec<u8>, ReceiveFrameError> {
        let mut response_frame: Vec<u8> = vec![];

        while true {
            let mut header = [0u8; 9];

            self.sock.read_exact(&mut header).await?;

            let length_of_payload =
                (header[2] as u32) | ((header[1] as u32) << 8) | ((header[0] as u32) << 16);

            let mut payload = vec![0u8; length_of_payload as usize];
            self.sock.read_exact(&mut payload).await?;

            let hex_arr = hex::encode(&payload);

            if length_of_payload > 8000 {
                response_frame = payload;
                break;
            }
        }

        Ok(response_frame)
    }

    fn get_dvt_port(&mut self, response_frame: Vec<u8>) -> Result<u16, ParseError> {
        let service = b"com.apple.instruments.dtservicehub";

        let service_index = response_frame
            .windows(service.len())
            .position(|window| window == service)
            .ok_or(ParseError::MatchError(
                "services in response frame".to_string(),
            ))?;
        let slice = &response_frame[service_index..];
        let port_index = slice
            .windows(4)
            .position(|window| window == b"Port")
            .ok_or(ParseError::MatchError("port in service".to_string()))?;
        let port = &slice[(port_index + 16)..(port_index + 21)];
        let port_str = std::str::from_utf8(port)?;
        let port_num: u16 = port_str.parse()?;
        Ok(port_num)
    }
}
