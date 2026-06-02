use webserver::ThreadPool;
use std::{
    fs,
    io::{BufReader, prelude::*},
    thread,
    time::Duration,
};
use tiny_http::shim::{SmolTcpListener, SmolTcpStream};

fn main() {
    println!("Attempting to bind to 127.0.0.1:7878 using Twizzler shim...");
    
    let listener = SmolTcpListener::bind("0.0.0.0:7878").expect("Failed to bind");    
    let pool = ThreadPool::new(4);

    loop {
        match listener.accept() {
            Ok((stream, _addr)) => {
                println!("Connection received!");
                pool.execute(|| {
                    handle_connection(stream);
                });
            }
            Err(e) => {
            }
        }
    }
}

fn handle_connection(mut stream: SmolTcpStream) {
    let mut buf_reader = BufReader::new(&mut stream);

    let request_line = match buf_reader.lines().next() {
        Some(Ok(line)) => line,
        _ => return,
    };

    let (status_line, contents) = match &request_line[..] {
        "GET / HTTP/1.1" => {
            ("HTTP/1.1 200 OK", include_str!("../hello.html"))
        }
        _ => {
            ("HTTP/1.1 404 NOT FOUND", include_str!("../404.html"))
        }
    };

    let length = contents.len();
    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).unwrap();
}
