use crate::Error;
use std::io::Write;

pub enum Content<'a> {
    Empty,
    Byte(Vec<u8>),
    Multipart(multipart::client::lazy::PreparedFields<'a>),
}

impl Content<'_> {
    pub fn write<W: Write>(&mut self, writer: &mut W) -> Result<(), Error> {
        match self {
            Content::Empty => Ok(()),
            Content::Byte(bytes) => Ok(writer.write_all(bytes).map_err(|e| Error::WriteFailed(e))?),
            Content::Multipart(ref mut prepared) => {
                // let mut prepared: multipart::client::lazy::PreparedFields =
                //     parts.prepare().map_err(|_e| Error::MultipartPrepareFailed)?;
                std::io::copy(prepared, writer).map_err(|e| Error::WriteFailed(e))?;
                Ok(())
            }
        }
    }
}
