use serde::{Deserialize, Serialize};
use std::error;
use std::net::TcpStream;
use std::path::PathBuf;
use std::io::{self, ErrorKind, Read, Write};

pub trait Transfer {

    /// Serialize the contents of this into a string
    fn pack(&self) -> serde_json::Result<String> where Self: Serialize {
        serde_json::to_string(&self)
    }

    /// Create this type from a serialized string
    fn unpack(input: String) -> serde_json::Result<Self> where
        Self: Sized + for<'a> Deserialize<'a> {

        serde_json::from_str::<Self>(&input)
    }

    /// Receive an instance of this type over the tcp stream `stream`
    fn receive(stream: &mut TcpStream)
               -> std::result::Result<Self, Box<dyn error::Error>> where
        Self: Sized + for<'a> Deserialize<'a> {

        let mut pdata = [0u8; 8];
        stream.read(&mut pdata)?;

        let size = usize::from_le_bytes(pdata);

        // Reject if the packet is larger than 250MB
        if size >= 0x1000_0000 {
            Err(io::Error::new(ErrorKind::InvalidData, "Packet too large"))?;
        }

        let mut data = vec![0u8; size];
        stream.read_exact(&mut data)?;

        let data = String::from_utf8(data)?;

        Self::unpack(data).map_err(std::convert::Into::into)

    }

    /// Serialize and send this type over the tcp stream `stream`
    fn send(&self, stream: &mut TcpStream)
            -> std::result::Result<(), Box<dyn error::Error>> where Self: Serialize {

        let data = self.pack()?;
        let data = data.as_bytes();
        let size = data.len();

        stream.write(&size.to_le_bytes())?;
        stream.write(&data)?;

        Ok(())
    }
}

/// This is a request that will me made by the client process and received by
/// the server
#[derive(Serialize, Deserialize)]
pub struct Request {
    pub dbname: String,
    pub needle: String,
}

/// The response that will be sent by the server to the client process
#[derive(Serialize, Deserialize)]
pub struct Response {
    pub error: bool,
    pub message: String,
    pub files: Vec<PathBuf>,
}

impl Transfer for Request {}
impl Transfer for Response {}

impl Response {
    pub fn new(message: String, files: Vec<PathBuf>) -> Self {
        Response {
            error: false,
            message,
            files,
        }
    }

    pub fn err<T: AsRef<str>>(message: T) -> Self {
        Response {
            error: true,
            message: message.as_ref().to_string(),
            files: vec![],
        }
    }
}
