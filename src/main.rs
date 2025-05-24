use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");
                handle_connection(&mut stream);
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

    handle_request(&method, &path, stream);
}

fn handle_request(method: &str, path: &str, stream: &mut TcpStream) {
    match method.to_uppercase().as_str() {
        "GET" => match path {
            "/index.html" => {
                stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n").unwrap();
            }
            _ => {
                stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").unwrap();
            }
        },
        _ => {
            stream
                .write_all(b"HTTP/1.1 405 Method Not Allowed\r\n\r\n")
                .unwrap();
        }
    }
}
