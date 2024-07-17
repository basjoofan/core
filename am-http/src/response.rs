use crate::Body;
use crate::Headers;
use crate::Version;

pub struct Response {
    /// The response's version
    pub version: Version,
    /// The response's status
    pub status: u16,
    /// The response's reason
    pub reason: String,
    /// The response's headers
    pub headers: Headers,
    /// The response's body
    pub body: dyn Body,
}
