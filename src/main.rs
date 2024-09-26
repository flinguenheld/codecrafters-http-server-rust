use anyhow::Result;
use std::{
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    thread::{self, JoinHandle},
};

fn main() -> Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let mut pool = ThreadPool::new(5);
    let listener = TcpListener::bind("127.0.0.1:4221")?;
    for tcp_stream in listener.incoming() {
        let stream = tcp_stream?;

        pool.execute(stream);
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> Result<()> {
    let buf_reader = BufReader::new(&mut stream);
    let mut it = buf_reader.lines();

    let mut content = String::new();
    if let Some(raw_line) = it.next() {
        let line = raw_line?;

        if line.starts_with("GET ") {
            if let Some(requested_page) = line.split_whitespace().nth(1) {
                let code = match requested_page {
                    p if p.starts_with("/echo/") => {
                        content = p[6..].to_string();
                        200
                    }
                    _ if requested_page.starts_with("/user-agent") => {
                        for raw_next_line in it {
                            let next_line = raw_next_line?;

                            if next_line.contains("User-Agent:") {
                                content = next_line[12..].to_string();
                                break;
                            }
                        }

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
    Ok(())
}

struct ThreadPool {
    maxi: usize,
    currents: Vec<JoinHandle<Result<()>>>,
}

impl ThreadPool {
    fn new(maxi: usize) -> Self {
        Self {
            maxi,
            currents: Vec::new(),
        }
    }

    fn execute(&mut self, stream: TcpStream) {
        self.currents.retain(|jh| !jh.is_finished());

        if self.currents.len() < self.maxi {
            println!("Connection accepted");
            self.currents
                .push(thread::spawn(|| handle_connection(stream)));
        } else {
            println!("Connection refused");
        }
    }
}
