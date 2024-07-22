use crate::Error;
use std::io::Write;

pub enum Content<'a> {
    Byte(Vec<u8>),
    Multipart(multipart::client::lazy::Multipart<'a, 'a>),
}

impl Content<'_> {
    pub fn write<W: Write>(&mut self, writer: &mut W) -> Result<(), Error> {
        match self {
            Content::Byte(bytes) => Ok(writer.write_all(bytes).map_err(|_e| Error::WriteFailed)?),
            Content::Multipart(ref mut parts) => {
                let mut prepared: multipart::client::lazy::PreparedFields =
                    parts.prepare().map_err(|_e| Error::MultipartPrepareFailed)?;
                std::io::copy(&mut prepared, writer).map_err(|_e| Error::WriteFailed)?;
                Ok(())
            }
        }
    }
}
