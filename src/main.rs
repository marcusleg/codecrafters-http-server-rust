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
    handle_request(stream, &request);
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

fn handle_request(stream: &mut TcpStream, request: &HttpRequest) {
    match request.method.to_uppercase().as_str() {
        "GET" => {
            if request.path == "/" || request.path == "/index.html" {
                send_response(
                    stream,
                    HttpResponse {
                        status: HttpStatus::OK,
                        headers: HashMap::new(),
                        body: None,
                    },
                );
            } else if request.path.starts_with("/echo/") {
                handle_get_echo(stream, &request);
            } else if request.path.starts_with("/files/") {
                handle_get_files(stream, &request);
            } else if request.path == "/user-agent" {
                handle_get_user_agent(stream, &request);
            } else {
                send_response(
                    stream,
                    HttpResponse {
                        status: HttpStatus::NOT_FOUND,
                        headers: HashMap::new(),
                        body: None,
                    },
                );
            }
        }
        _ => send_response(
            stream,
            HttpResponse {
                status: HttpStatus::METHOD_NOT_ALLOWED,
                headers: HashMap::new(),
                body: None,
            },
        ),
    }
}

fn send_response(stream: &mut TcpStream, response: HttpResponse) {
    let mut headers = response.headers;
    if headers.get("Content-Type").is_none() {
        headers.insert("Content-Type".to_string(), "text/plain".to_string());
    }

    let response_string = match response.body {
        None => {
            headers.insert("Content-Length".to_string(), "0".to_string());

            let headers_string = headers
                .iter()
                .map(|(k, v)| format!("{}: {}\r\n", k, v))
                .collect::<Vec<String>>()
                .join("");
            format!(
                "HTTP/1.1 {} {}\r\n{}\r\n",
                response.status.code, response.status.text, headers_string
            )
        }
        Some(HttpBody::Text(text)) => {
            let headers_string = headers
                .iter()
                .map(|(k, v)| format!("{}: {}\r\n", k, v))
                .collect::<Vec<String>>()
                .join("");
            format!(
                "HTTP/1.1 {} {}\r\n{}\r\n{}",
                response.status.code, response.status.text, headers_string, text
            )
        }
        Some(HttpBody::Binary(bytes)) => {
            let headers_string = headers
                .iter()
                .map(|(k, v)| format!("{}: {}\r\n", k, v))
                .collect::<Vec<String>>()
                .join("");
            let response_headers = format!(
                "HTTP/1.1 {} {}\r\n{}\r\n",
                response.status.code, response.status.text, headers_string
            );
            stream.write_all(response_headers.as_bytes()).unwrap();
            stream.write_all(&bytes).unwrap();
            return;
        }
    };

    stream.write_all(response_string.as_bytes()).unwrap();
}

fn handle_get_echo(stream: &mut TcpStream, request: &HttpRequest) {
    let response_body = request.path.strip_prefix("/echo/").unwrap();

    send_response(
        stream,
        HttpResponse {
            status: HttpStatus::OK,
            headers: HashMap::from([("Content-Type".to_string(), "text/plain".to_string())]),
            body: Some(HttpBody::Text(response_body.to_string())),
        },
    );
}

fn handle_get_files(stream: &mut TcpStream, request: &HttpRequest) {
    let files_directory = FILES_DIRECTORY.get().unwrap();
    if files_directory.is_none() {
        send_response(
            stream,
            HttpResponse {
                status: HttpStatus::NOT_FOUND,
                headers: HashMap::new(),
                body: None,
            },
        );
        return;
    }
    let files_directory = files_directory.as_ref().unwrap();

    let file_name = request.path.strip_prefix("/files/").unwrap();
    let file_path = format!("{}/{}", files_directory, file_name);

    match std::fs::read(&file_path) {
        Ok(contents) => {
            send_response(
                stream,
                HttpResponse {
                    status: HttpStatus::OK,
                    headers: HashMap::from([(
                        "Content-Type".to_string(),
                        "application/octet-stream".to_string(),
                    )]),
                    body: Some(HttpBody::Binary(contents)),
                },
            );
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                send_response(
                    stream,
                    HttpResponse {
                        status: HttpStatus::NOT_FOUND,
                        headers: HashMap::new(),
                        body: None,
                    },
                );
            } else {
                send_response(
                    stream,
                    HttpResponse {
                        status: HttpStatus::INTERNAL_SERVER_ERROR,
                        headers: HashMap::new(),
                        body: None,
                    },
                );
            }
        }
    }
}

fn handle_get_user_agent(stream: &mut TcpStream, request: &HttpRequest) {
    let user_agent = request.headers.get("user-agent");

    match user_agent {
        None => {
            send_response(
                stream,
                HttpResponse {
                    status: HttpStatus::BAD_REQUEST,
                    headers: HashMap::new(),
                    body: None,
                },
            );
        }
        Some(user_agent) => {
            send_response(
                stream,
                HttpResponse {
                    status: HttpStatus::OK,
                    headers: HashMap::from([(
                        "Content-Type".to_string(),
                        "text/plain".to_string(),
                    )]),
                    body: Some(HttpBody::Text(user_agent.to_string())),
                },
            );
        }
    }
}
