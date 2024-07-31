use super::Error;
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
            Content::Byte(bytes) => Ok(writer.write_all(bytes).map_err(Error::WriteFailed)?),
            Content::Multipart(ref mut prepared) => {
                std::io::copy(prepared, writer).map_err(Error::WriteFailed)?;
                Ok(())
            }
        }
    }
}
