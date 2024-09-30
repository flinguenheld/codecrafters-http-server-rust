use anyhow::Result;
use std::{
    env,
    fs::{self},
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    thread::{self, JoinHandle},
};

#[derive(Default, Debug)]
struct Request<'a> {
    mode: &'a str,
    page: &'a str,
    user_agent: &'a str,
    encoding: &'a str,
    content_length: usize,
    content: &'a str,
}

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

    // Request --
    let mut request = Request::default();
    for line in request_string.lines() {
        dbg!(&line);
        if let Some(page) = line.strip_prefix("GET ") {
            request.mode = "GET";
            if let Some((p, _)) = page.split_once(' ') {
                request.page = p;
            }
        } else if let Some(page) = line.strip_prefix("POST ") {
            request.mode = "POST";
            if let Some((p, _)) = page.split_once(' ') {
                request.page = p;
            }
        } else if let Some(user_agent) = line.strip_prefix("User-Agent: ") {
            request.user_agent = user_agent;
        } else if let Some(encoding) = line.strip_prefix("Accept-Encoding: ") {
            request.encoding = encoding;
        } else if let Some(content_length) = line.strip_prefix("Content-Length: ") {
            if let Ok(value) = content_length.parse::<usize>() {
                request.content_length = value;
            }
        } else if !line.is_empty() && !line.contains(':') {
            request.content = line;
        }
    }

    dbg!(&request);

    // Response --
    let mut response = String::from("HTTP/1.1 404 Not Found\r\n\r\n");
    let mut content: Vec<u8> = Vec::new();
    if request.mode == "GET" {
        if request.page.starts_with("/echo/") {
            response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n{}\r\n",
                compress(request.encoding, &request.page[6..], &mut content)
            );
        } else if request.page.starts_with("/user-agent") {
            response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n{}\r\n",
                compress(request.encoding, request.user_agent, &mut content)
            );
        } else if request.page.starts_with("/files/") {
            let p = format!("{}{}", file_dir, &request.page[7..]);
            let path = Path::new(p.as_str());

            if path.exists() {
                let txt = fs::read_to_string(path)?;

                response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\n{}\r\n",
                    compress(request.encoding, txt.as_str(), &mut content)
                );
            } else {
                println!("Path {} does not exist.", p);
            }
        } else if request.page == "/" {
            response = "HTTP/1.1 200 OK\r\n\r\n".to_string();
        }
    } else if request.mode == "POST" {
        if request.page.starts_with("/files/") {
            let file_name = &request.page[7..];
            if request.content_length == request.content.len() && !file_name.is_empty() {
                let path = format!("{}{}", file_dir, file_name);
                fs::create_dir_all(file_dir)?;
                fs::write(path, request.content)?;

                response = "HTTP/1.1 201 Created\r\n\r\n".to_string();
            }
        }
    }

    let response: Vec<u8> = response
        .as_bytes()
        .iter()
        .chain(content.iter())
        .copied()
        .collect();

    stream.write_all(&response)?;

    Ok(())
}

fn compress<'a>(format: &'a str, content: &'a str, content_bytes: &mut Vec<u8>) -> String {
    match format.to_uppercase().as_str() {
        "GZIP" => {
            *content_bytes = content.as_bytes().to_vec();
            format!(
                "Content-Encoding: gzip\r\nContent-Length: {}\r\n",
                content_bytes.len()
            )
        }
        _ => {
            *content_bytes = content.as_bytes().to_vec();
            format!("Content-Length: {}\r\n", content_bytes.len())
        }
    }
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
