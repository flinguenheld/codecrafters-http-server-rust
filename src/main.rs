use anyhow::Result;
use std::{
    env, fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    path::Path,
    thread::{self, JoinHandle},
};

fn main() -> Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let mut file_dir = String::new();
    if let Some(a) = env::args().position(|a| a == "--directory") {
        if let Some(dir) = env::args().nth(a + 1) {
            file_dir = dir;
        }
    }

    let mut pool = ThreadPool::new(5);
    let listener = TcpListener::bind("127.0.0.1:4221")?;
    for tcp_stream in listener.incoming() {
        let stream = tcp_stream?;

        pool.execute(stream, file_dir.clone());
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream, file_dir: String) -> Result<()> {
    let buf_reader = BufReader::new(&mut stream);
    let mut it = buf_reader.lines();

    let mut message = Vec::new();
    if let Some(raw_line) = it.next() {
        let line = raw_line?;

        if line.starts_with("GET ") {
            if let Some(requested_page) = line.split_whitespace().nth(1) {
                let code = match requested_page {
                    p if p.starts_with("/echo/") => {
                        message.push("Content-Type: text/plain".to_string());
                        message.push(format!("Content-Length: {}\r\n", p[6..].len()));
                        message.push(p[6..].to_string());
                        200
                    }
                    _ if requested_page.starts_with("/user-agent") => {
                        for raw_next_line in it {
                            let next = raw_next_line?;

                            if next.contains("User-Agent:") {
                                message.push("Content-Type: text/plain".to_string());
                                message.push(format!("Content-Length: {}\r\n", &next[12..].len()));
                                message.push(next[12..].to_string());
                                break;
                            }
                        }
                        200
                    }
                    file if requested_page.starts_with("/files/") => {
                        let p = format!("{}{}", file_dir, &file[7..]);
                        let path = Path::new(p.as_str());

                        if path.exists() {
                            let txt = fs::read_to_string(path)?;
                            message.push("Content-Type: application/octet-stream".to_string());
                            message.push(format!("Content-Length: {}\r\n", txt.len()));
                            message.push(txt.to_string());
                            200
                        } else {
                            println!("Path {} does not exist.", p);
                            404
                        }
                    }
                    "/" => 200,
                    _ => 404,
                };

                let response = match code {
                    200 => &format!("HTTP/1.1 200 OK\r\n{}\r\n", message.join("\r\n")),
                    _ => &format!("HTTP/1.1 404 Not Found\r\n{}\r\n", message.join("\r\n")),
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

    fn execute(&mut self, stream: TcpStream, file_dir: String) {
        self.currents.retain(|jh| !jh.is_finished());

        if self.currents.len() < self.maxi {
            println!("Connection accepted");
            self.currents
                .push(thread::spawn(|| handle_connection(stream, file_dir)));
        } else {
            println!("Connection refused");
        }
    }
}
