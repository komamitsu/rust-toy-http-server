extern crate regex;
#[macro_use]
extern crate lazy_static;

use std::error::Error;
use std::fmt;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::str;
use std::thread;
use regex::Regex;

#[derive(Debug)]
struct InvalidHTTPHeaderError(String);

impl fmt::Display for InvalidHTTPHeaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for InvalidHTTPHeaderError {
    fn description(&self) -> &str {
        "Invalid HTTP Header"
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

lazy_static! {
    static ref STATUS_LINE_PATTERN : Regex = Regex::new(r"([[:upper:]]+) \S+ HTTP/(\d.\d)").unwrap();
}

struct ToyHttpServer<'a> {
    host: &'a str,
    port: u16
}

impl <'a> ToyHttpServer<'a> {
    pub fn new(host: &'a str, port: u16) -> Self {
        ToyHttpServer { host: host, port: port }
    }

    // TODO: Improve the error handling if needed

    fn read_headers(stream: &mut TcpStream,
                    header_content: &mut Vec<u8>) -> Result<(), InvalidHTTPHeaderError> {
        for b in stream.bytes() {
            match b {
                Ok(b) => {
                    header_content.push(b);
                    let len = header_content.len();
                    if len >= 4 &&
                        header_content[len - 4] == b'\r' &&
                        header_content[len - 3] == b'\n' &&
                        header_content[len - 2] == b'\r' &&
                        header_content[len - 1] == b'\n' {
                            return Ok(())
                    }
                },
                Err(e) => panic!("Unexpected EOF: {}", e)
            }
        }
        let header_content_for_err = header_content.clone();
        Err(InvalidHTTPHeaderError(String::from_utf8(header_content_for_err).unwrap()))
    }

    fn read_body(len: usize,
                 stream: &mut TcpStream,
                 body_content: &mut Vec<u8>) {
        let mut remaining = len;
        let mut buf = [0; 512];
        while remaining != 0 {
            let size = stream.read(&mut buf).unwrap();
            remaining -= size;
            body_content.extend(&buf[0 .. size]);
        }
    }

    fn handle_client(mut stream: TcpStream) {
        loop {
            let mut header_content = Vec::new();
            if Self::read_headers(&mut stream, &mut header_content).is_err() {
                return
            }
            let headers_str = String::from_utf8(header_content).unwrap();
            let mut headers = headers_str.split("\r\n");
            let status_line = headers.next().unwrap();
            let cap = STATUS_LINE_PATTERN.captures(status_line).unwrap();
            let method = &cap[1];
            let version = &cap[2];
            if ! ["GET", "POST", "PUT"].contains(&method) ||
                (version != "0.9" && version != "1.0" && version != "1.1") {
                panic!("Unexpected status line: {}", status_line);
            }

            let content_length = headers.
                map(|x| x.split(':').collect()).
                find(|x: &Vec<&str>| x[0].to_lowercase() == "content-type").
                and_then(|x| x[1].trim().parse::<usize>().ok());

            match content_length {
                Some(cl) => {
                    println!("Content-Length: {}", cl);
                    let mut body_content = Vec::new();
                    Self::read_body(cl, &mut stream, &mut body_content);
                    // TODO: Do something
                    println!("Content Body: {:?}", String::from_utf8(body_content));
                },
                None => ()
            }

            // FIXME: Send something useful data
            stream.write(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 12\r\n\r\nHello world!").unwrap();
            stream.flush().unwrap();
        }
    }

    pub fn start(&self) {
        let listener = TcpListener::bind(format!("{}:{}", self.host, self.port).as_str()).unwrap();
        
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    thread::spawn(move || Self::handle_client(stream));
                },
                Err(e) => println!("Connection failed: {:?}", e)
            }
        }
    }
}

fn main() {
    ToyHttpServer::new("127.0.0.1", 8080).start()
}
