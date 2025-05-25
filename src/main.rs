use clap::Parser;
use http_request::HttpRequest;
use http_response::HttpResponse;
use http_status::HttpStatus;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::OnceLock;
use std::thread;

mod http_request;
mod http_response;
mod http_status;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    directory: Option<String>,
}

enum HttpBody {
    Text(String),
    Binary(Vec<u8>),
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
    match http_request::parse(stream) {
        Ok(request) => {
            let response = handle_request(&request);
            http_response::send(stream, response);
        }
        Err(err) => {
            eprintln!("Failed to parse HTTP request: {}", err);

            http_response::send(
                stream,
                HttpResponse {
                    status: HttpStatus::BAD_REQUEST,
                    headers: HashMap::new(),
                    body: None,
                },
            );
        }
    };
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
        "POST" => {
            if request.path.starts_with("/files/") {
                handle_post_files(&request)
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

fn handle_post_files(request: &HttpRequest) -> HttpResponse {
    if request.headers.get("content-type").unwrap() != "application/octet-stream" {
        return HttpResponse {
            status: HttpStatus::BAD_REQUEST,
            headers: HashMap::new(),
            body: None,
        };
    }

    let file_name = request.path.strip_prefix("/files/").unwrap();
    let files_directory = FILES_DIRECTORY.get().unwrap();
    let files_directory = files_directory.as_ref().unwrap();
    let file_path = format!("{}/{}", files_directory, file_name);

    let mut file = std::fs::File::create(&file_path).unwrap();
    file.write_all(request.body.as_ref().unwrap()).unwrap();

    HttpResponse {
        status: HttpStatus::CREATED,
        headers: HashMap::new(),
        body: None,
    }
}
