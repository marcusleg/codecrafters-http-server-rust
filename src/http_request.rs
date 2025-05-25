use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;

pub struct HttpRequest {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) headers: HashMap<String, String>,
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
    request.method = parsed_request_line.0;
    request.path = parsed_request_line.1;
    println!("Received {} request for {}", request.method, request.path);

    for line in lines {
        match line {
            Ok(line) => {
                if line.trim().is_empty() {
                    break;
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
    }

    Ok(request)
}

fn parse_request_line(request_line: String) -> Result<(String, String)> {
    let parts: Vec<&str> = request_line.split(" ").collect();
    if parts.len() >= 2 {
        Ok((
            parts.get(0).unwrap().to_string(),
            parts.get(1).unwrap().to_string(),
        ))
    } else {
        Err(anyhow!("Invalid request line: {}", request_line))
    }
}
