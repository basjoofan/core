use tokio::fs::File;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;
use tokio::io::copy;

pub enum Content {
    Empty,
    Bytes(Vec<u8>),
    Parts(Vec<Part>),
}

pub enum Part {
    Bytes(Vec<u8>),
    File(File),
}

impl Content {
    pub async fn write<W: AsyncWrite + Unpin>(self, mut writer: W) -> Result<(), std::io::Error> {
        match self {
            Content::Empty => Ok(()),
            Content::Bytes(bytes) => writer.write_all(&bytes).await,
            Content::Parts(parts) => {
                for part in parts {
                    match part {
                        Part::Bytes(bytes) => writer.write_all(&bytes).await?,
                        Part::File(mut file) => {
                            let _ = copy(&mut file, &mut writer).await?;
                        }
                    }
                }
                Ok(())
            }
        }
    }
}
