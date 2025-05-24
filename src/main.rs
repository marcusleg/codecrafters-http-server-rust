use clap::Parser;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::OnceLock;
use std::thread;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    directory: Option<String>,
}

struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
}

struct HttpStatus {
    code: u16,
    text: &'static str,
}

impl HttpStatus {
    const OK: HttpStatus = HttpStatus {
        code: 200,
        text: "OK",
    };
    const BAD_REQUEST: HttpStatus = HttpStatus {
        code: 400,
        text: "Bad Request",
    };
    const NOT_FOUND: HttpStatus = HttpStatus {
        code: 404,
        text: "Not Found",
    };
    const METHOD_NOT_ALLOWED: HttpStatus = HttpStatus {
        code: 405,
        text: "Method Not Allowed",
    };
    const INTERNAL_SERVER_ERROR: HttpStatus = HttpStatus {
        code: 500,
        text: "Internal Server Error",
    };
}

static FILES_DIRECTORY: OnceLock<Option<String>> = OnceLock::new();

fn main() {
    let args = Args::parse();
    FILES_DIRECTORY.set(args.directory).unwrap();

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                thread::spawn(move || {
                    println!("accepted new connection");
                    handle_connection(&mut stream);
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_connection(stream: &mut TcpStream) {
    let mut request = HttpRequest {
        method: String::new(),
        path: String::new(),
        headers: HashMap::new(),
    };

    let reader = BufReader::new(&*stream);
    let mut line_count: usize = 0;
    for line in reader.lines() {
        match line {
            Ok(line) => {
                if line.trim().is_empty() {
                    break;
                } else if line_count == 0 {
                    let parts: Vec<&str> = line.split(" ").collect();
                    if parts.len() < 2 {
                        println!("Invalid HTTP request received.");
                        break;
                    }
                    request.method = parts[0].to_string();
                    request.path = parts[1].to_string();
                    println!("Received {} request for {}", request.method, request.path);
                } else if line.contains(": ") {
                    let parts: Vec<&str> = line.split(": ").collect();
                    if parts.len() == 2 {
                        request
                            .headers
                            .insert(parts[0].to_string().to_lowercase(), parts[1].to_string());
                    }
                    println!("Received header: {}", line)
                } else {
                    println!("Received unknown line: {}", line)
                }
            }
            Err(e) => {
                println!("Failed to read from connection: {}", e);
                break;
            }
        }
        line_count = line_count + 1;
    }

    handle_request(stream, &request);
}

fn handle_request(stream: &mut TcpStream, request: &HttpRequest) {
    match request.method.to_uppercase().as_str() {
        "GET" => {
            if request.path == "/" || request.path == "/index.html" {
                send_response(stream, HttpStatus::OK, None, None);
            } else if request.path.starts_with("/echo/") {
                handle_get_echo(stream, &request);
            } else if request.path.starts_with("/files/") {
                handle_get_files(stream, &request);
            } else if request.path == "/user-agent" {
                handle_get_user_agent(stream, &request);
            } else {
                send_response(stream, HttpStatus::NOT_FOUND, None, None);
            }
        }
        _ => send_response(stream, HttpStatus::METHOD_NOT_ALLOWED, None, None),
    }
}

fn send_response(
    stream: &mut TcpStream,
    status: HttpStatus,
    content_type: Option<&str>,
    body: Option<&str>,
) {
    let response;
    let content_type = content_type.unwrap_or("text/plain");

    match body {
        None => {
            response = format!(
                "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: 0\r\n\r\n",
                status.code, status.text, content_type
            )
        }
        Some(_) => {
            let response_body = body.unwrap();
            let content_length = response_body.len();
            response = format!(
                "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
                status.code, status.text, content_type, content_length, response_body
            )
        }
    }

    stream.write_all(response.as_bytes()).unwrap();
}

fn handle_get_echo(stream: &mut TcpStream, request: &HttpRequest) {
    let response_body = request.path.strip_prefix("/echo/").unwrap();

    send_response(
        stream,
        HttpStatus::OK,
        Some("text/plain"),
        Some(response_body),
    );
}

fn handle_get_files(stream: &mut TcpStream, request: &HttpRequest) {
    let files_directory = FILES_DIRECTORY.get().unwrap();
    if files_directory.is_none() {
        send_response(stream, HttpStatus::NOT_FOUND, None, None);
        return;
    }
    let files_directory = files_directory.as_ref().unwrap();

    let file_name = request.path.strip_prefix("/files/").unwrap();
    let file_path = format!("{}/{}", files_directory, file_name);

    match std::fs::read_to_string(&file_path) {
        Ok(contents) => {
            send_response(
                stream,
                HttpStatus::OK,
                Some("application/octet-stream"),
                Some(&contents),
            );
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                send_response(stream, HttpStatus::NOT_FOUND, None, None);
            } else {
                send_response(stream, HttpStatus::INTERNAL_SERVER_ERROR, None, None);
            }
        }
    }
}

fn handle_get_user_agent(stream: &mut TcpStream, request: &HttpRequest) {
    let user_agent = request.headers.get("user-agent");

    match user_agent {
        None => {
            send_response(stream, HttpStatus::BAD_REQUEST, None, None);
        }
        Some(user_agent) => {
            send_response(stream, HttpStatus::OK, Some("text/plain"), Some(user_agent));
        }
    }
}
