use anyhow::Result;
use std::{
    io::{prelude::*, BufReader},
    net::TcpListener,
};

fn main() -> Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221")?;
    for tcp_stream in listener.incoming() {
        match tcp_stream {
            Ok(mut stream) => {
                println!("accepted new connection");

                let mut reader = BufReader::new(&mut stream);

                let mut line = String::new();
                reader.read_line(&mut line)?;

                if line.starts_with("GET ") {
                    if let Some(page) = line.split_whitespace().nth(1) {
                        let response = match page {
                            txt if txt.starts_with("/echo/") => {
                                &format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",txt.len() - 6, &txt[6..])
                            }
                            "/" => "HTTP/1.1 200 OK\r\n\r\n",
                            _ => "HTTP/1.1 404 Not Found\r\n\r\n",
                        };

                        stream.write_all(response.as_bytes())?;
                    }
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    Ok(())
}
