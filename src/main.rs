use anyhow::Result;
use std::{
    env,
    fs::{self},
    io::{BufRead, BufReader, Write},
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
    let mut buf_reader = BufReader::new(&mut stream);
    let received = buf_reader.fill_buf()?;
    let request_string = String::from_utf8_lossy(received).into_owned();

    // Also ok -----
    // let mut request = [0_u8; 1024];
    // let bytes = stream.read(&mut request)?;
    // let request_string = String::from_utf8_lossy(&request[..bytes]).into_owned();
    // -----

    let mut it_lines = request_string.lines();

    let mut code = 404;
    let mut message = Vec::new();
    if let Some(line) = it_lines.next() {
        if line.starts_with("GET ") {
            if let Some(requested_page) = line.split_whitespace().nth(1) {
                code = match requested_page {
                    p if p.starts_with("/echo/") => {
                        message.push("Content-Type: text/plain".to_string());
                        message.push(format!("Content-Length: {}\r\n", p[6..].len()));
                        message.push(p[6..].to_string());
                        200
                    }
                    _ if requested_page.starts_with("/user-agent") => {
                        for next in it_lines {
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
            }
        } else if line.starts_with("POST /files/") {
            let mut length = 0;
            let mut content = String::new();

            if let Some((file_name, _)) = &line[12..].split_once(" ") {
                for con in it_lines {
                    content = con.to_string(); // File's content is at the end of the request

                    if content.starts_with("Content-Length: ") {
                        dbg!(&content);
                        if let Ok(value) = &content[16..].parse::<usize>() {
                            length = *value;
                        }
                    }
                }

                if length == content.len() && !file_name.is_empty() {
                    let path = format!("{}{}", file_dir, file_name);
                    fs::create_dir_all(file_dir)?;
                    fs::write(path, content)?;

                    code = 201;
                }
            }
        }

        let response = match code {
            200 => format!("HTTP/1.1 200 OK\r\n{}\r\n", message.join("\r\n")),
            201 => "HTTP/1.1 201 Created\r\n\r\n".to_string(),
            _ => format!("HTTP/1.1 404 Not Found\r\n{}\r\n", message.join("\r\n")),
        };
        stream.write_all(response.as_bytes())?;
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
                .push(thread::spawn(move || handle_connection(stream, file_dir)));
        } else {
            println!("Connection refused");
        }
    }
}
