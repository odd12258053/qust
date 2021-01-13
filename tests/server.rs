#[cfg(test)]
mod tests {
    use mio::net::TcpStream;
    use mio::{Events, Interest, Poll, Token};
    use std::io;
    use std::io::prelude::*;
    use std::net::Shutdown;

    fn read(stream: &mut TcpStream) -> Vec<u8> {
        let mut ret = vec![0; 0];
        let mut buffer = [0u8; 4096];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(s) => ret.extend(&buffer[0..s]),
                Err(ref err) if err.kind() == io::ErrorKind::Interrupted => continue,
                _ => break,
            }
        }
        ret
    }

    #[test]
    fn job_routine() {
        let max_size = 1 * 1024 * 1024;
        let addr = "127.0.0.1:9000".parse().unwrap();
        let mut stream = TcpStream::connect(addr).unwrap();

        let token = Token(0);
        let mut poll = Poll::new().unwrap();
        let mut events = Events::with_capacity(1);

        poll.registry()
            .register(&mut stream, token, Interest::WRITABLE)
            .unwrap();

        poll.poll(&mut events, None).unwrap();
        for event in events.iter() {
            assert_eq!(event.token().0, token.0);
            assert!(event.is_writable());
            stream.write(b"DELQUE test-que\n").unwrap();
            poll.registry()
                .reregister(&mut stream, token, Interest::READABLE)
                .unwrap();
        }

        let mut ret = vec![0; 0];
        'end_delque1: loop {
            poll.poll(&mut events, None).unwrap();
            for event in events.iter() {
                assert_eq!(event.token().0, token.0);
                assert!(event.is_readable());
                ret.extend(read(&mut stream));
                if ret[ret.len() - 1] == b'\n' {
                    poll.registry()
                        .reregister(&mut stream, token, Interest::WRITABLE)
                        .unwrap();
                    break 'end_delque1;
                } else {
                    poll.registry()
                        .reregister(&mut stream, token, Interest::READABLE)
                        .unwrap();
                }
            }
        }
        assert_eq!(ret.len(), 3);

        let job = vec!['a' as u8; max_size];

        poll.poll(&mut events, None).unwrap();
        for event in events.iter() {
            assert_eq!(event.token().0, token.0);
            assert!(event.is_writable());
            stream
                .write(
                    [b"ADDJOB test-que 300 ", job.as_slice(), b"\n"]
                        .concat()
                        .as_slice(),
                )
                .unwrap();
            poll.registry()
                .reregister(&mut stream, token, Interest::READABLE)
                .unwrap();
        }

        let mut ret = vec![0; 0];
        'end_addjob: loop {
            poll.poll(&mut events, None).unwrap();
            for event in events.iter() {
                assert_eq!(event.token().0, token.0);
                assert!(event.is_readable());
                ret.extend(read(&mut stream));
                if ret[ret.len() - 1] == b'\n' {
                    poll.registry()
                        .reregister(&mut stream, token, Interest::WRITABLE)
                        .unwrap();
                    break 'end_addjob;
                } else {
                    poll.registry()
                        .reregister(&mut stream, token, Interest::READABLE)
                        .unwrap();
                }
            }
        }
        assert_eq!(ret.len(), 35);
        assert_eq!(&ret[0..2], b"1 ");
        assert_eq!(&ret[34..35], b"\n");

        poll.poll(&mut events, None).unwrap();
        for event in events.iter() {
            assert_eq!(event.token().0, token.0);
            assert!(event.is_writable());
            stream.write(b"GETJOB test-que\n").unwrap();
            poll.registry()
                .reregister(&mut stream, token, Interest::READABLE)
                .unwrap();
        }

        let mut ret = vec![0; 0];
        'end_getjob: loop {
            poll.poll(&mut events, None).unwrap();
            for event in events.iter() {
                assert_eq!(event.token().0, token.0);
                assert!(event.is_readable());
                ret.extend(read(&mut stream));
                if ret[ret.len() - 1] == b'\n' {
                    poll.registry()
                        .reregister(&mut stream, token, Interest::WRITABLE)
                        .unwrap();
                    break 'end_getjob;
                } else {
                    poll.registry()
                        .reregister(&mut stream, token, Interest::READABLE)
                        .unwrap();
                }
            }
        }
        assert_eq!(ret.len(), 36 + max_size);
        assert_eq!(&ret[0..2], b"1 ");
        let job_id = &ret[2..34].to_vec();
        assert_eq!(ret[35..ret.len() - 1], job);
        assert_eq!(&ret[ret.len() - 1..ret.len()], b"\n");

        poll.poll(&mut events, None).unwrap();
        for event in events.iter() {
            assert_eq!(event.token().0, token.0);
            assert!(event.is_writable());
            stream
                .write([b"ACKJOB ", job_id.as_slice(), b"\n"].concat().as_slice())
                .unwrap();
            poll.registry()
                .reregister(&mut stream, token, Interest::READABLE)
                .unwrap();
        }

        let mut ret = vec![0; 0];
        'end_ackjob: loop {
            poll.poll(&mut events, None).unwrap();
            for event in events.iter() {
                assert_eq!(event.token().0, token.0);
                assert!(event.is_readable());
                ret.extend(read(&mut stream));
                if ret[ret.len() - 1] == b'\n' {
                    poll.registry()
                        .reregister(&mut stream, token, Interest::WRITABLE)
                        .unwrap();
                    break 'end_ackjob;
                } else {
                    poll.registry()
                        .reregister(&mut stream, token, Interest::READABLE)
                        .unwrap();
                }
            }
        }
        assert_eq!(ret.len(), 3);
        assert_eq!(&ret[0..ret.len()], b"1 \n");

        poll.poll(&mut events, None).unwrap();
        for event in events.iter() {
            assert_eq!(event.token().0, token.0);
            assert!(event.is_writable());
            stream.write(b"DELQUE test-que\n").unwrap();
            poll.registry()
                .reregister(&mut stream, token, Interest::READABLE)
                .unwrap();
        }

        let mut ret = vec![0; 0];
        'end_delque: loop {
            poll.poll(&mut events, None).unwrap();
            for event in events.iter() {
                assert_eq!(event.token().0, token.0);
                assert!(event.is_readable());
                ret.extend(read(&mut stream));
                if ret[ret.len() - 1] == b'\n' {
                    poll.registry()
                        .reregister(&mut stream, token, Interest::WRITABLE)
                        .unwrap();
                    break 'end_delque;
                } else {
                    poll.registry()
                        .reregister(&mut stream, token, Interest::READABLE)
                        .unwrap();
                }
            }
        }
        assert_eq!(ret.len(), 3);
        assert_eq!(&ret[0..ret.len()], b"1 \n");

        let _ = stream.shutdown(Shutdown::Both);
    }
}
