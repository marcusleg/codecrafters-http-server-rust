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
    let mut method = String::new();
    let mut path = String::new();
    let mut headers: HashMap<String, String> = HashMap::new();

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
                    method = parts[0].to_string();
                    path = parts[1].to_string();
                    println!("Received {} request for {}", method, path);
                } else if line.contains(": ") {
                    let parts: Vec<&str> = line.split(": ").collect();
                    if parts.len() == 2 {
                        headers.insert(parts[0].to_string().to_lowercase(), parts[1].to_string());
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

    handle_request(stream, &method, &path, &headers);
}

fn handle_request(
    stream: &mut TcpStream,
    method: &str,
    path: &str,
    headers: &HashMap<String, String>,
) {
    match method.to_uppercase().as_str() {
        "GET" => {
            if path == "/" || path == "/index.html" {
                send_response(stream, 200, None, None);
            } else if path.starts_with("/echo/") {
                handle_get_echo(stream, path);
            } else if path.starts_with("/files/") {
                handle_get_files(stream, path);
            } else if path == "/user-agent" {
                handle_get_user_agent(stream, headers);
            } else {
                send_response(stream, 404, None, None);
            }
        }
        _ => send_response(stream, 405, None, None),
    }
}

fn send_response(
    stream: &mut TcpStream,
    status_code: usize,
    content_type: Option<&str>,
    body: Option<&str>,
) {
    let status_text = match status_code {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        _ => todo!(),
    };

    let response;
    let content_type = content_type.unwrap_or("text/plain");

    match body {
        None => {
            response = format!(
                "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: 0\r\n\r\n",
                status_code, status_text, content_type
            )
        }
        Some(_) => {
            let response_body = body.unwrap();
            let content_length = response_body.len();
            response = format!(
                "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
                status_code, status_text, content_type, content_length, response_body
            )
        }
    }

    stream.write_all(response.as_bytes()).unwrap();
}

fn handle_get_echo(stream: &mut TcpStream, path: &str) {
    let response_body = path.strip_prefix("/echo/").unwrap();

    send_response(stream, 200, Some(response_body), None);
}

fn handle_get_files(stream: &mut TcpStream, path: &str) {
    let files_directory = FILES_DIRECTORY.get().unwrap();
    if files_directory.is_none() {
        send_response(stream, 404, None, None);
        return;
    }
    let files_directory = files_directory.as_ref().unwrap();

    let file_name = path.strip_prefix("/files/").unwrap();
    let file_path = format!("{}/{}", files_directory, file_name);

    match std::fs::read_to_string(&file_path) {
        Ok(contents) => {
            send_response(
                stream,
                200,
                Some("application/octet-stream"),
                Some(&contents),
            );
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                send_response(stream, 404, None, None);
            } else {
                send_response(stream, 500, None, None);
            }
        }
    }
}

fn handle_get_user_agent(stream: &mut TcpStream, headers: &HashMap<String, String>) {
    let user_agent = headers.get("user-agent");

    match user_agent {
        None => {
            send_response(stream, 400, None, None);
        }
        Some(user_agent) => {
            send_response(stream, 200, Some("text/plain"), Some(user_agent));
        }
    }
}
