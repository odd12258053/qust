#[macro_use]
extern crate log;

use env_logger::Env;
use qust::Server;
use std::env::args;
use std::process::exit;

const HOST: &str = "127.0.0.1";
const PORT: &str = "9000";

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

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp_micros()
        .format_module_path(false)
        .init();

    let mut host = HOST.to_owned();
    let mut port = PORT.to_owned();

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
                }
            }
            None => break,
        }
    }

    let addr = match [host.as_str(), port.as_str()].join(":").parse() {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("error: {}", e);
            show_help_mini();
            exit(1);
        }
    };

    info!("You can connect to the server using `nc`:");
    info!(" $ nc {}", addr);
    info!("You'll see our welcome message and anything you type we'll be printed here.");
    Server::run(addr).unwrap();
}
