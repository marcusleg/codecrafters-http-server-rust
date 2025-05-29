use crate::http_headers::HttpHeaders;
use anyhow::{Context, Result};
use clap::Parser;
use http_body::HttpBody;
use http_request::HttpRequest;
use http_response::HttpResponse;
use http_status::HttpStatus;
use std::fmt::Debug;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::OnceLock;
use std::thread;

mod http_body;
mod http_headers;
mod http_request;
mod http_response;
mod http_status;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    directory: Option<String>,
}

static FILES_DIRECTORY: OnceLock<Option<String>> = OnceLock::new();

fn main() {
    let args = Args::parse();
    FILES_DIRECTORY
        .set(args.directory)
        .expect("Failed to set files directory");

    let listener = TcpListener::bind("127.0.0.1:4221").expect("Failed to bind to port");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                thread::spawn(move || {
                    println!("accepted new connection");
                    if let Err(e) = handle_connection(&mut stream) {
                        eprintln!("Error handling connection: {}", e);
                    }
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_connection(stream: &mut TcpStream) -> Result<()> {
    let request = match http_request::parse(stream) {
        Ok(request) => request,
        Err(_) => {
            http_response::send(
                stream,
                HttpResponse {
                    status: HttpStatus::BAD_REQUEST,
                    headers: HttpHeaders::new(),
                    body: None,
                },
            )
            .context("Failed to send response")?;
            return Ok(());
        }
    };

    let response = handle_request(&request).unwrap_or_else(|_| HttpResponse {
        status: HttpStatus::INTERNAL_SERVER_ERROR,
        headers: HttpHeaders::new(),
        body: None,
    });

    http_response::send(stream, response).context("Failed to send response")?;
    Ok(())
}

fn handle_request(request: &HttpRequest) -> Result<HttpResponse> {
    match request.method.to_uppercase().as_str() {
        "GET" => {
            if request.path == "/" || request.path == "/index.html" {
                Ok(HttpResponse {
                    status: HttpStatus::OK,
                    headers: HttpHeaders::new(),
                    body: None,
                })
            } else if request.path.starts_with("/echo/") {
                handle_get_echo(&request)
            } else if request.path.starts_with("/files/") {
                handle_get_files(&request)
            } else if request.path == "/user-agent" {
                handle_get_user_agent(&request)
            } else {
                Ok(HttpResponse {
                    status: HttpStatus::NOT_FOUND,
                    headers: HttpHeaders::new(),
                    body: None,
                })
            }
        }
        "POST" => {
            if request.path.starts_with("/files/") {
                handle_post_files(&request)
            } else {
                Ok(HttpResponse {
                    status: HttpStatus::NOT_FOUND,
                    headers: HttpHeaders::new(),
                    body: None,
                })
            }
        }
        _ => Ok(HttpResponse {
            status: HttpStatus::METHOD_NOT_ALLOWED,
            headers: HttpHeaders::new(),
            body: None,
        }),
    }
}

fn handle_get_echo(request: &HttpRequest) -> Result<HttpResponse> {
    let response_body = request
        .path
        .strip_prefix("/echo/")
        .context("Failed to strip prefix")?;

    Ok(HttpResponse {
        status: HttpStatus::OK,
        headers: HttpHeaders::from([("Content-Type".to_string(), "text/plain".to_string())]),
        body: Some(HttpBody::Text(response_body.to_string())),
    })
}

fn handle_get_files(request: &HttpRequest) -> Result<HttpResponse> {
    let files_directory = FILES_DIRECTORY.get().unwrap();
    if files_directory.is_none() {
        return Ok(HttpResponse {
            status: HttpStatus::NOT_FOUND,
            headers: HttpHeaders::new(),
            body: None,
        });
    }
    let files_directory = files_directory
        .as_ref()
        .context("Failed to get files directory")?;

    let file_name = request
        .path
        .strip_prefix("/files/")
        .context("Failed to strip prefix")?;
    let file_path = format!("{}/{}", files_directory, file_name);

    match std::fs::read(&file_path) {
        Ok(contents) => Ok(HttpResponse {
            status: HttpStatus::OK,
            headers: HttpHeaders::from([(
                "Content-Type".to_string(),
                "application/octet-stream".to_string(),
            )]),
            body: Some(HttpBody::Binary(contents)),
        }),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Ok(HttpResponse {
                    status: HttpStatus::NOT_FOUND,
                    headers: HttpHeaders::new(),
                    body: None,
                })
            } else {
                Err(e).context("Failed to read file")?
            }
        }
    }
}

fn handle_get_user_agent(request: &HttpRequest) -> Result<HttpResponse> {
    let user_agent = request
        .headers
        .get("user-agent")
        .context("Failed to get user agent")?;

    Ok(HttpResponse {
        status: HttpStatus::OK,
        headers: HttpHeaders::from([("Content-Type".to_string(), "text/plain".to_string())]),
        body: Some(HttpBody::Text(user_agent.to_string())),
    })
}

fn handle_post_files(request: &HttpRequest) -> Result<HttpResponse> {
    if request
        .headers
        .get("content-type")
        .context("Failed to get content type")?
        != "application/octet-stream"
    {
        return Ok(HttpResponse {
            status: HttpStatus::BAD_REQUEST,
            headers: HttpHeaders::new(),
            body: None,
        });
    }

    let file_name = request
        .path
        .strip_prefix("/files/")
        .context("Failed to strip prefix")?;
    let files_directory = FILES_DIRECTORY
        .get()
        .context("Failed to get files directory")?;
    let files_directory = files_directory
        .as_ref()
        .context("Failed to get files directory")?;
    let file_path = format!("{}/{}", files_directory, file_name);

    let mut file = std::fs::File::create(&file_path).context("Failed to create file")?;
    file.write_all(request.body.as_ref().unwrap().as_bytes())
        .context("Failed to write file")?;

    Ok(HttpResponse {
        status: HttpStatus::CREATED,
        headers: HttpHeaders::new(),
        body: None,
    })
}
