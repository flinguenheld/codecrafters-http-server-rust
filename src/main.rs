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

                let mut content: &str = "";
                if line.starts_with("GET ") {
                    if let Some(page) = line.split_whitespace().nth(1) {
                        let code = match page {
                            p if p.starts_with("/echo/") => {
                                content = &p[6..];
                                200
                            }
                            _ if page.starts_with("/user-agent") => {
                                while !line.contains("User-Agent:") {
                                    line.clear();
                                    reader.read_line(&mut line)?;
                                }

                                content = &line[12..line.len() - 2];
                                200
                            }
                            "/" => 200,
                            _ => 400,
                        };

                        let response = match code {
                            200 => {
                                if content.is_empty() {
                                    "HTTP/1.1 200 OK\r\n\r\n"
                                } else {
                                    &format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                                    content.len(), content)
                                }
                            }
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
