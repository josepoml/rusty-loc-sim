mod dxt;
pub mod errors;

use std::io::Cursor;

use byteorder::LittleEndian;
use dxt::{create_locationsm_message, HANDSHAKE_MESSAGE, SL_CHANNEL_REQUEST};
use errors::DtServiceError;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub struct DtServiceHandler {
    sock: TcpStream,
}

impl DtServiceHandler {
    pub async fn new(server_addr: &String, server_port: &u16) -> Result<Self, DtServiceError> {
        let sock = TcpStream::connect(format!("[{}]:{}", server_addr, server_port)).await?;
        Ok(DtServiceHandler { sock: sock })
    }

    pub async fn do_handshake(&mut self) -> Result<(), DtServiceError> {
        self.sock.write_all(&HANDSHAKE_MESSAGE).await?;
        self.receive_dxt_message().await?;

        Ok(())
    }

    pub async fn start_channel(
        &mut self,
        channel_identifier: String,
    ) -> Result<(), DtServiceError> {
        self.sock.write_all(&SL_CHANNEL_REQUEST).await?;
        self.receive_dxt_message().await?;

        Ok(())
    }

    pub async fn simulate_location(&mut self, lat: f64, lng: f64) -> Result<(), DtServiceError> {
        let sm = create_locationsm_message(lat, lng);
        self.sock.write_all(&sm).await?;
        self.receive_dxt_message().await?;
        Ok(())
    }

    async fn receive_dxt_message(&mut self) -> Result<(), DtServiceError> {
        let mut dxt_msg_header = [0u8; 32];

        self.sock.read_exact(&mut dxt_msg_header).await?;

        let mut cursor = Cursor::new(&dxt_msg_header);

        cursor.set_position(12);

        let mut length = byteorder::ReadBytesExt::read_u32::<LittleEndian>(&mut cursor)?;

        let mut payload = vec![0u8; length as usize];

        self.sock.read_exact(&mut payload).await?;

        Ok(())
    }
}
