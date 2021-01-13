use crate::utils::compare;
use std::str::from_utf8;

#[derive(Debug, Eq, PartialEq)]
pub enum Command {
    TERMINATE,
    QUIT,
    HELLO,
    ADDJOB,
    GETJOB,
    ACKJOB,
    STATQUE,
    DELQUE,
}

const QUIT: &[u8] = b"QUIT";
const HELLO: &[u8] = b"HELLO";
const ADDJOB: &[u8] = b"ADDJOB";
const GETJOB: &[u8] = b"GETJOB";
const ACKJOB: &[u8] = b"ACKJOB";
const STATQUE: &[u8] = b"STATQUE";
const DELQUE: &[u8] = b"DELQUE";

pub const ENABLE_COMMANDS: [Command; 7] = [
    Command::ACKJOB,
    Command::ADDJOB,
    Command::DELQUE,
    Command::GETJOB,
    Command::HELLO,
    Command::QUIT,
    Command::STATQUE,
];

impl Command {
    pub fn from(value: &[u8]) -> Option<Command> {
        if value == ADDJOB {
            Some(Command::ADDJOB)
        } else if value == GETJOB {
            Some(Command::GETJOB)
        } else if value == ACKJOB {
            Some(Command::ACKJOB)
        } else if value == STATQUE {
            Some(Command::STATQUE)
        } else if value == DELQUE {
            Some(Command::DELQUE)
        } else if value == QUIT {
            Some(Command::QUIT)
        } else if value == HELLO {
            Some(Command::HELLO)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &[u8] {
        match self {
            Command::TERMINATE => b"",
            Command::QUIT => QUIT,
            Command::HELLO => HELLO,
            Command::ADDJOB => ADDJOB,
            Command::GETJOB => GETJOB,
            Command::ACKJOB => ACKJOB,
            Command::STATQUE => STATQUE,
            Command::DELQUE => DELQUE,
        }
    }

    pub fn estimate(value: &[u8]) -> Vec<Command> {
        let mut cmds = Vec::new();

        let value = match from_utf8(value) {
            Ok(s) => s.to_uppercase(),
            Err(_) => return cmds,
        };
        let idx = value.len() - 1;

        if Some(idx) == compare(value.as_bytes(), ADDJOB) {
            cmds.push(Command::ADDJOB);
        }
        if Some(idx) == compare(value.as_bytes(), GETJOB) {
            cmds.push(Command::GETJOB);
        }
        if Some(idx) == compare(value.as_bytes(), ACKJOB) {
            cmds.push(Command::ACKJOB);
        }
        if Some(idx) == compare(value.as_bytes(), STATQUE) {
            cmds.push(Command::STATQUE);
        }
        if Some(idx) == compare(value.as_bytes(), DELQUE) {
            cmds.push(Command::DELQUE);
        }
        if Some(idx) == compare(value.as_bytes(), QUIT) {
            cmds.push(Command::QUIT);
        }
        if Some(idx) == compare(value.as_bytes(), HELLO) {
            cmds.push(Command::HELLO);
        }
        cmds
    }
}
