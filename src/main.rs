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

    let mut request = Request::default();
    for line in request_string.lines() {
        dbg!(&line);
        if line.starts_with("GET ") {
            request.mode = "GET";
            if let Some((p, _)) = &line[4..].split_once(' ') {
                request.page = p;
            }
        } else if line.starts_with("POST ") {
            request.mode = "POST";
            if let Some((p, _)) = &line[5..].split_once(' ') {
                request.page = p;
            }
        } else if line.starts_with("User-Agent: ") {
            request.user_agent = &line[12..];
        } else if line.starts_with("Accept-Encoding: ") {
            request.encoding = &line[17..];
        } else if line.starts_with("Content-Length: ") {
            if let Ok(value) = &line[16..].parse::<usize>() {
                request.content_length = *value;
            }
        } else if !line.is_empty() && !line.contains(':') {
            request.content = line;
        }
    }

    dbg!(&request);

    let mut code = 404;
    let mut output = Vec::new();
    if request.mode == "GET" {
        if request.page.starts_with("/echo/") {
            output.push("Content-Type: text/plain".to_string());
            output.push(format!("Content-Length: {}\r\n", request.page[6..].len()));
            output.push(request.page[6..].to_string());
            code = 200;
        } else if request.page.starts_with("/user-agent") {
            output.push("Content-Type: text/plain".to_string());
            output.push(format!("Content-Length: {}\r\n", request.user_agent.len()));
            output.push(request.user_agent.to_string());
            code = 200;
        } else if request.page.starts_with("/files/") {
            let p = format!("{}{}", file_dir, &request.page[7..]);
            let path = Path::new(p.as_str());

            if path.exists() {
                let txt = fs::read_to_string(path)?;
                output.push("Content-Type: application/octet-stream".to_string());
                output.push(format!("Content-Length: {}\r\n", txt.len()));
                output.push(txt.to_string());
                code = 200;
            } else {
                println!("Path {} does not exist.", p);
            }
        } else if request.page == "/" {
            code = 200;
        }
    } else if request.mode == "POST" {
        if request.page.starts_with("/files/") {
            let file_name = &request.page[7..];
            if request.content_length == request.content.len() && !file_name.is_empty() {
                let path = format!("{}{}", file_dir, file_name);
                fs::create_dir_all(file_dir)?;
                fs::write(path, request.content)?;

                code = 201;
            }
        }
    }

    let response = match code {
        200 => format!("HTTP/1.1 200 OK\r\n{}\r\n", output.join("\r\n")),
        201 => "HTTP/1.1 201 Created\r\n\r\n".to_string(),
        _ => format!("HTTP/1.1 404 Not Found\r\n{}\r\n", output.join("\r\n")),
    };
    stream.write_all(response.as_bytes())?;
    // }
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
