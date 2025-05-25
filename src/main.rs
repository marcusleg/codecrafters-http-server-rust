use clap::Parser;
use http_status::HttpStatus;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::OnceLock;
use std::thread;

mod http_status;

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

enum HttpBody {
    Text(String),
    Binary(Vec<u8>),
}

struct HttpResponse {
    status: HttpStatus,
    headers: HashMap<String, String>,
    body: Option<HttpBody>,
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
    let request = parse_request(stream);
    let response = handle_request(&request);
    send_response(stream, response);
}

fn parse_request(stream: &mut TcpStream) -> HttpRequest {
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

    request
}

fn handle_request(request: &HttpRequest) -> HttpResponse {
    match request.method.to_uppercase().as_str() {
        "GET" => {
            if request.path == "/" || request.path == "/index.html" {
                HttpResponse {
                    status: HttpStatus::OK,
                    headers: HashMap::new(),
                    body: None,
                }
            } else if request.path.starts_with("/echo/") {
                handle_get_echo(&request)
            } else if request.path.starts_with("/files/") {
                handle_get_files(&request)
            } else if request.path == "/user-agent" {
                handle_get_user_agent(&request)
            } else {
                HttpResponse {
                    status: HttpStatus::NOT_FOUND,
                    headers: HashMap::new(),
                    body: None,
                }
            }
        }
        _ => HttpResponse {
            status: HttpStatus::METHOD_NOT_ALLOWED,
            headers: HashMap::new(),
            body: None,
        },
    }
}

fn send_response(stream: &mut TcpStream, mut response: HttpResponse) {
    // send status line
    let status_line = format!(
        "HTTP/1.1 {} {}\r\n",
        response.status.code, response.status.text
    );
    stream.write_all(status_line.as_bytes()).unwrap();

    // send headers
    if response.headers.get("Content-Type").is_none() {
        response
            .headers
            .insert("Content-Type".to_string(), "text/plain".to_string());
    }

    let content_length = match response.body {
        None => 0,
        Some(ref content) => match content {
            HttpBody::Text(text) => text.len(),
            HttpBody::Binary(bytes) => bytes.len(),
        },
    };
    response
        .headers
        .insert("Content-Length".to_string(), content_length.to_string());

    let headers_string = response
        .headers
        .iter()
        .map(|(k, v)| format!("{}: {}\r\n", k, v))
        .collect::<Vec<String>>()
        .join("");
    stream.write_all(headers_string.as_bytes()).unwrap();

    // send empty line indicating the headers are complete
    stream.write_all("\r\n".as_bytes()).unwrap();

    // send body
    match response.body {
        None => {}
        Some(HttpBody::Text(text)) => stream.write_all(text.as_bytes()).unwrap(),
        Some(HttpBody::Binary(bytes)) => stream.write_all(&bytes).unwrap(),
    };
}

fn handle_get_echo(request: &HttpRequest) -> HttpResponse {
    let response_body = request.path.strip_prefix("/echo/").unwrap();

    HttpResponse {
        status: HttpStatus::OK,
        headers: HashMap::from([("Content-Type".to_string(), "text/plain".to_string())]),
        body: Some(HttpBody::Text(response_body.to_string())),
    }
}

fn handle_get_files(request: &HttpRequest) -> HttpResponse {
    let files_directory = FILES_DIRECTORY.get().unwrap();
    if files_directory.is_none() {
        return HttpResponse {
            status: HttpStatus::NOT_FOUND,
            headers: HashMap::new(),
            body: None,
        };
    }
    let files_directory = files_directory.as_ref().unwrap();

    let file_name = request.path.strip_prefix("/files/").unwrap();
    let file_path = format!("{}/{}", files_directory, file_name);

    match std::fs::read(&file_path) {
        Ok(contents) => HttpResponse {
            status: HttpStatus::OK,
            headers: HashMap::from([(
                "Content-Type".to_string(),
                "application/octet-stream".to_string(),
            )]),
            body: Some(HttpBody::Binary(contents)),
        },
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                HttpResponse {
                    status: HttpStatus::NOT_FOUND,
                    headers: HashMap::new(),
                    body: None,
                }
            } else {
                HttpResponse {
                    status: HttpStatus::INTERNAL_SERVER_ERROR,
                    headers: HashMap::new(),
                    body: None,
                }
            }
        }
    }
}

fn handle_get_user_agent(request: &HttpRequest) -> HttpResponse {
    let user_agent = request.headers.get("user-agent");

    match user_agent {
        None => HttpResponse {
            status: HttpStatus::BAD_REQUEST,
            headers: HashMap::new(),
            body: None,
        },
        Some(user_agent) => HttpResponse {
            status: HttpStatus::OK,
            headers: HashMap::from([("Content-Type".to_string(), "text/plain".to_string())]),
            body: Some(HttpBody::Text(user_agent.to_string())),
        },
    }
}
