use criterion::{criterion_group, criterion_main, Criterion};
use std::io::prelude::*;
use std::net::{Shutdown, TcpStream};

macro_rules! hello_multi {
    ($clients:expr) => {
        |iters| {
            let mut streams = Vec::new();
            for _ in 0..$clients {
                streams.push(TcpStream::connect("127.0.0.1:9000").unwrap());
            }
            let mut iter = 0..iters;
            let ret = b"0 Hello\n";
            let start = std::time::Instant::now();
            loop {
                let mut close = false;
                let mut count = 0u8;
                let mut buf = vec![0; 4096];
                for mut stream in streams.iter() {
                    match iter.next() {
                        Some(_) => {
                            stream.write(b"HELLO bar\n").unwrap();
                            count += 1;
                        }
                        None => {
                            close = true;
                            break;
                        }
                    }
                }
                let mut it_stream = streams.iter();
                for _ in 0..count {
                    if let Some(mut stream) = it_stream.next() {
                        let size = stream.read(&mut buf).unwrap();
                        assert_eq!(&buf[0..size], ret);
                    }
                }
                if close {
                    break;
                }
            }
            let elapsed = start.elapsed();
            for stream in streams.iter() {
                let _ = stream.shutdown(Shutdown::Both);
            }
            elapsed
        }
    };
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Hello");
    group.bench_function("hello", |b| {
        b.iter_custom(|iters| {
            let mut stream = TcpStream::connect("127.0.0.1:9000").unwrap();
            let mut buf = vec![0; 4096];
            let ret = b"0 Hello\n";
            let start = std::time::Instant::now();
            for _ in 0..iters {
                stream.write(b"HELLO bar\n").unwrap();
                let size = stream.read(&mut buf).unwrap();
                assert_eq!(&buf[0..size], ret);
            }
            let elapsed = start.elapsed();
            let _ = stream.shutdown(Shutdown::Both);
            elapsed
        })
    });
    group.bench_function("hello multi", |b| b.iter_custom(hello_multi!(10)));
    group.bench_function("hello multi 100", |b| b.iter_custom(hello_multi!(100)));
    group.finish();
    let mut group = c.benchmark_group("JOB");
    group.bench_function("addjob", |b| {
        b.iter_custom(|iters| {
            let mut stream = TcpStream::connect("127.0.0.1:9000").unwrap();
            let mut buf = vec![0; 4096];
            let ret = b"1";
            let start = std::time::Instant::now();
            for _ in 0..iters {
                stream.write(b"ADDJOB aa 300 bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n").unwrap();
                let _size = stream.read(&mut buf).unwrap();
                assert_eq!(&buf[0..1], ret);
            }
            let elapsed = start.elapsed();
            let _ = stream.shutdown(Shutdown::Both);
            elapsed
        })
    });
    group.bench_function("getjob", |b| {
        b.iter_custom(|iters| {
            let mut stream = TcpStream::connect("127.0.0.1:9000").unwrap();
            let mut buf = vec![0; 4096];
            let ret = b"1";
            for _ in 0..iters {
                stream.write(b"ADDJOB aa 300 bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n").unwrap();
                let _size = stream.read(&mut buf).unwrap();
                assert_eq!(&buf[0..1], ret);
            }
            let start = std::time::Instant::now();
            for _ in 0..iters {
                stream.write(b"GETJOB aa\n").unwrap();
                let _size = stream.read(&mut buf).unwrap();
                assert_eq!(&buf[0..1], ret);
            }
            let elapsed = start.elapsed();
            stream.write(b"DELQUE aa\n").unwrap();
            let _size = stream.read(&mut buf).unwrap();
            let _ = stream.shutdown(Shutdown::Both);
            elapsed
        })
    });
    group.bench_function("ackjob", |b| {
        b.iter_custom(|iters| {
            let mut stream = TcpStream::connect("127.0.0.1:9000").unwrap();
            let mut buf = vec![0; 4096];
            let ret = b"1";
            let mut jobs = Vec::new();
            for _ in 0..iters {
                stream.write(b"ADDJOB aa 300 bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n").unwrap();
                let _size = stream.read(&mut buf).unwrap();
                assert_eq!(&buf[0..1], ret);
            }
            for _ in 0..iters {
                stream.write(b"GETJOB aa\n").unwrap();
                let size = stream.read(&mut buf).unwrap();
                assert_eq!(size, 94);
                assert_eq!(&buf[0..1], ret);
                jobs.push(buf[2..34].to_vec());
            }
            let start = std::time::Instant::now();
            for job in jobs.iter() {
                stream.write([b"ACKJOB ", job.as_slice(), b"\n"].concat().as_slice()).unwrap();
                let _size = stream.read(&mut buf).unwrap();
                assert_eq!(&buf[0..1], ret);
            }
            let elapsed = start.elapsed();
            stream.write(b"DELQUE aa\n").unwrap();
            let _size = stream.read(&mut buf).unwrap();
            let _ = stream.shutdown(Shutdown::Both);
            elapsed
        })
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
