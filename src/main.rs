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
    stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n").unwrap();

    let reader = BufReader::new(&*stream);
    for line in reader.lines() {
        match line {
            Ok(line) => {
                if line.trim().is_empty() {
                    break;
                }

                println!("Received: {}", line)
            }
            Err(e) => {
                println!("Failed to read from connection: {}", e);
                break;
            }
        }
    }

    stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").unwrap();
}
