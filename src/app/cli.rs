use interaction::{Interaction, InteractionBuilder};
use qust::command;
use qust::command::ENABLE_COMMANDS;
use std::env;
use std::env::args;
use std::io;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::exit;
use std::str::from_utf8;

const HOST: &str = "127.0.0.1";
const PORT: &str = "9000";
const HISTORY_SIZE: usize = 3000;

fn show_help() {
    println!(
        "{}",
        [
            format!("{} {}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION")).as_str(),
            env!("CARGO_PKG_AUTHORS"),
            "",
            env!("CARGO_PKG_DESCRIPTION"),
            "",
            "USAGE:",
            format!("    {} -h {} -p {}", env!("CARGO_BIN_NAME"), HOST, PORT).as_str(),
            "",
            "OPTIONS:",
            "    -h, --host <host>",
            format!("        Set a host. Default: {}", HOST).as_str(),
            "    -p, --port <port>",
            format!("        Set a port. Default: {}", PORT).as_str(),
            "    --history-size <size>",
            format!("        Set a size of history. Default: {}", HISTORY_SIZE).as_str(),
            "    --help",
            "        Prints help information. Use --help for more details.",
            "    --version",
            "        Prints version information.",
            "",
        ]
        .join("\n")
    );
}

fn show_help_mini() {
    println!(
        "{}",
        [
            "",
            "USAGE:",
            format!("    {} -h {} -p {}", env!("CARGO_BIN_NAME"), HOST, PORT).as_str(),
            "",
            "For more information try --help",
            "",
        ]
        .join("\n")
    );
}

fn show_version() {
    println!("{} {}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));
}

macro_rules! show_help {
    ($arg: expr) => {
        if $arg == "--help" {
            show_help();
            exit(0);
        } else if $arg == "--version" {
            show_version();
            exit(0);
        }
    };
}

struct App {
    history_file: PathBuf,
    history_limit: usize,
    addr: SocketAddr,
}

impl App {
    fn new() -> Self {
        App {
            history_file: [
                env::var("HOME").unwrap_or(String::from(".")).as_str(),
                ".qust_history",
            ]
            .iter()
            .collect(),
            history_limit: 3000,
            addr: format!("{}:{}", HOST, PORT).parse().unwrap(),
        }
    }

    fn arg_parse(mut self) -> Self {
        let mut host = HOST.to_owned();
        let mut port = PORT.to_owned();
        let mut history_size = HISTORY_SIZE.clone();

        let mut args = args();
        // skip arg[0]
        args.next();
        loop {
            match args.next() {
                Some(arg) => {
                    show_help!(arg);
                    if arg == "-h" || arg == "--host" {
                        match args.next() {
                            Some(arg) => {
                                show_help!(arg);
                                host = arg.to_owned()
                            }
                            None => {
                                println!("error: Not found host. Please you set a host.");
                                show_help_mini();
                                exit(1);
                            }
                        }
                    } else if arg == "-p" || arg == "--port" {
                        match args.next() {
                            Some(arg) => {
                                show_help!(arg);
                                port = arg.to_owned()
                            }
                            None => {
                                println!("error: Not found port. Please you set a port.");
                                show_help_mini();
                                exit(1);
                            }
                        }
                    } else if arg == "--history-size" {
                        match args.next() {
                            Some(arg) => {
                                show_help!(arg);
                                history_size = match arg.parse() {
                                    Ok(s) => s,
                                    Err(e) => {
                                        eprintln!("error: {}", e);
                                        show_help();
                                        exit(1);
                                    }
                                }
                            }
                            None => {
                                println!(
                                    "error: Not found size. Please you set a size of history."
                                );
                                show_help_mini();
                                exit(1);
                            }
                        }
                    }
                }
                None => break,
            }
        }

        self.addr = match [host.as_str(), port.as_str()].join(":").parse() {
            Ok(addr) => addr,
            Err(e) => {
                eprintln!("error: {}", e);
                show_help_mini();
                exit(1);
            }
        };
        self.history_limit = history_size;
        self
    }

    fn interaction(&self) -> Interaction {
        InteractionBuilder::new()
            .prompt(format!("{} >", &self.addr).as_bytes())
            .history_limit(self.history_limit)
            .completion(|input, completions| {
                if input.len() == 0 {
                    for cmd in ENABLE_COMMANDS.iter() {
                        completions.push(cmd.as_str().to_vec());
                    }
                } else {
                    for cmd in command::Command::estimate(input.as_slice()) {
                        completions.push(cmd.as_str().to_vec());
                    }
                }
            })
            .load_history(self.history_file.as_path())
            .unwrap()
            .build()
    }

    fn run(self) {
        let mut inter = self.interaction();
        'input: loop {
            match inter.line() {
                Ok(input) => {
                    if input.len() == 0 {
                        continue 'input;
                    }
                    let mut iter = input.splitn(2, |c| *c == b' ');
                    let cmd = match iter
                        .next()
                        .and_then(|cmd| from_utf8(cmd).ok())
                        .and_then(|cmd| command::Command::from(cmd.to_uppercase().as_bytes()))
                    {
                        Some(cmd) => cmd,
                        None => {
                            eprintln!("Invalid the command. In this version, the follow is a enable command.");
                            for cmd in ENABLE_COMMANDS.iter() {
                                eprintln!("* {}", from_utf8(cmd.as_str()).unwrap());
                            }
                            continue 'input;
                        }
                    };

                    if cmd == command::Command::QUIT {
                        break 'input;
                    }

                    let mut stream = TcpStream::connect("127.0.0.1:9000").unwrap();
                    let request = iter
                        .next()
                        .and_then(|tail| Some([cmd.as_str(), b" ", tail, b"\n"].concat()))
                        .unwrap_or([cmd.as_str(), b"\n"].concat());
                    if let Err(e) = stream.write(request.as_slice()) {
                        eprintln!("Error: {}", e);
                        continue 'input;
                    }
                    let mut buf = vec![0; 4096];
                    match stream.read(&mut buf) {
                        Ok(n) => {
                            // TODO
                            print!("{}", from_utf8(&buf[..n]).unwrap());
                        }
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            continue 'input;
                        }
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                    inter.save_history(self.history_file.as_path()).unwrap();
                    break;
                }
                Err(_) => {
                    break;
                }
            }
        }
    }
}

fn main() {
    App::new().arg_parse().run();
}
