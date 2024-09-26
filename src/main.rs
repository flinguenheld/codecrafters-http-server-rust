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

    dbg!(&file_dir);

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

    let mut content = String::new();
    if let Some(raw_line) = it.next() {
        let line = raw_line?;

        if line.starts_with("GET ") {
            if let Some(requested_page) = line.split_whitespace().nth(1) {
                let code = match requested_page {
                    p if p.starts_with("/echo/") => {
                        let echoed = p[6..].to_string();
                        content = format!(
                            "Content-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                            echoed.len(),
                            echoed
                        );

                        200
                    }
                    _ if requested_page.starts_with("/user-agent") => {
                        for raw_next_line in it {
                            let next_line = raw_next_line?;

                            if next_line.contains("User-Agent:") {
                                let txt = next_line[12..].to_string();
                                content = format!(
                                    "Content-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                                    txt.len(),
                                    txt
                                );
                                break;
                            }
                        }

                        200
                    }

                    file if requested_page.starts_with("/files/") => {
                        dbg!(&file);

                        let mut code_value = 400;
                        let p = format!("{}{}", file_dir, &file[7..]);
                        let path = Path::new(p.as_str());

                        if path.exists() {
                            let txt = fs::read_to_string(path)?;
                            content = format!(
                                "Content-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
                                txt.len(),
                                txt
                            );

                            code_value = 200;
                        }

                        code_value
                    }

                    "/" => {
                        content = "\r\n".to_string();

                        200
                    }
                    _ => 400,
                };

                dbg!(&content);

                let response = match code {
                    200 => &format!("HTTP/1.1 200 OK\r\n{}", content),
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
