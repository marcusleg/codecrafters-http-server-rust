use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
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

    let mut reader = BufReader::new(&*stream);
    let request_line = parse_request_line(&mut reader).context("Failed to parse request line")?;

    request.method = request_line.method;
    request.path = request_line.path;
    println!("Received {} request for {}", request.method, request.path);

    request.headers = parse_headers(&mut reader).context("Failed to parse headers")?;

    Ok(request)
}

fn parse_request_line(reader: &mut BufReader<&TcpStream>) -> Result<RequestLine> {
    let mut buffer = Vec::new();
    reader
        .read_until(b'\n', &mut buffer)
        .context("Failed to read request line")?;

    let request_line = String::from_utf8(buffer)
        .context("Request line is not valid UTF-8")?
        .trim_end()
        .to_string();

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

fn parse_headers(reader: &mut BufReader<&TcpStream>) -> Result<HashMap<String, String>> {
    let mut headers = HashMap::new();
    let mut buffer = String::new();

    loop {
        buffer.clear();
        let bytes_read = reader
            .read_line(&mut buffer)
            .context("Failed to read header line")?;

        if bytes_read == 0 || buffer.trim().is_empty() {
            break;
        }

        let line = buffer.trim_end();

        if line.contains(": ") {
            let parts: Vec<&str> = line.split(": ").collect();
            if parts.len() == 2 {
                headers.insert(parts[0].to_string().to_lowercase(), parts[1].to_string());
            }
            println!("Received header: {}", line);
        } else {
            println!("Received unknown line: {}", line);
        }
    }

    Ok(headers)
}
