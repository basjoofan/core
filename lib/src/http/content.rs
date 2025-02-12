use super::Stream;
// use std::io::Read;
use tokio::io::AsyncWriteExt;
use tokio::io::WriteHalf;

pub enum Content {
    Empty,
    Byte(Vec<u8>),
    // TODO Multipart(multipart::client::lazy::PreparedFields<'a>),
}

impl Content {
    pub async fn write(&mut self, writer: &mut WriteHalf<Stream>) -> Result<(), std::io::Error> {
        match self {
            Content::Empty => Ok(()),
            Content::Byte(bytes) => writer.write_all(bytes).await,
            // Content::Multipart(_) => {
            //     // let mut buffer = [0; 1024];
            //     // while let Ok(read) = prepared.read(&mut buffer) {
            //     //     if read == 0 {
            //     //         break;
            //     //     }
            //     //     writer.write_all(&buffer[0..read]).await?;
            //     // }
            //     Ok(())
            // }
        }
    }
}
