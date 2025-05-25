use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Lines};
use std::net::TcpStream;

pub struct HttpRequest {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) headers: HashMap<String, String>,
}

struct RequestLine {
    method: String,
    path: String,
}

pub fn parse(stream: &mut TcpStream) -> Result<HttpRequest> {
    let mut request = HttpRequest {
        method: String::new(),
        path: String::new(),
        headers: HashMap::new(),
    };

    let reader = BufReader::new(&*stream);
    let mut lines = reader.lines();

    let request_line = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("Failed to read request line"))??;
    let parsed_request_line =
        parse_request_line(request_line).context("Failed to parse request line")?;
    request.method = parsed_request_line.method;
    request.path = parsed_request_line.path;
    println!("Received {} request for {}", request.method, request.path);

    request.headers = parse_headers(&mut lines).context("Failed to parse headers")?;

    Ok(request)
}

fn parse_request_line(request_line: String) -> Result<RequestLine> {
    let parts: Vec<&str> = request_line.split(" ").collect();
    if parts.len() == 3 {
        Ok(RequestLine {
            method: parts.get(0).unwrap().to_string(),
            path: parts.get(1).unwrap().to_string(),
        })
    } else {
        Err(anyhow!("Invalid request line: {}", request_line))
    }
}

fn parse_headers(lines: &mut Lines<BufReader<&TcpStream>>) -> Result<HashMap<String, String>> {
    let mut headers = HashMap::new();

    for line in lines {
        match line {
            Ok(line) => {
                if line.trim().is_empty() {
                    break;
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
    }

    Ok(headers)
}
