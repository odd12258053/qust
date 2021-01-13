use crate::command::Command;
use mio::Token;

pub(crate) const TERMINATION: u8 = b'\n';

#[derive(Debug)]
pub struct Request {
    pub token: Token,
    pub cmd: Command,
    pub arg: Vec<u8>,
}

#[derive(Debug)]
pub struct Reply {
    pub token: Token,
    pub status: i8,
    pub data: Vec<u8>,
}

impl Reply {
    pub fn message(&self) -> Vec<u8> {
        [
            format!("{} ", self.status).as_bytes(),
            self.data.as_slice(),
            &[TERMINATION],
        ]
        .concat()
    }
    pub fn error(token: Token) -> Reply {
        Reply {
            token,
            status: -1,
            data: b"Error".to_vec(),
        }
    }
    pub fn empty(token: Token) -> Reply {
        Reply {
            token,
            status: 0,
            data: vec![0; 0],
        }
    }
}
