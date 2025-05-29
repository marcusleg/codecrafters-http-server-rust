use crate::http_body::HttpBody;
use crate::http_headers::HttpHeaders;
use anyhow::{anyhow, Context, Result};
use std::io::{BufRead, BufReader, Read};
use std::net::TcpStream;

pub struct HttpRequest {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) headers: HttpHeaders,
    pub(crate) body: Option<HttpBody>,
}

struct RequestLine {
    method: String,
    path: String,
}

pub fn parse(stream: &mut TcpStream) -> Result<HttpRequest> {
    let mut request = HttpRequest {
        method: String::new(),
        path: String::new(),
        headers: HttpHeaders::new(),
        body: None,
    };

    let mut reader = BufReader::new(&*stream);
    let request_line = parse_request_line(&mut reader).context("Failed to parse request line")?;

    request.method = request_line.method;
    request.path = request_line.path;
    println!("Received {} request for {}", request.method, request.path);

    request.headers = parse_headers(&mut reader).context("Failed to parse headers")?;

    let content_length = request.headers.get("content-length");
    if content_length.is_some() {
        let content_length: usize = content_length
            .context("Unable to read Content-Length header")?
            .parse()
            .context("Unable to parse Content-Length header")?;
        let body = parse_body(&mut reader, content_length).context("Failed to parse body")?;
        request.body = Some(body);
    }

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

fn parse_headers(reader: &mut BufReader<&TcpStream>) -> Result<HttpHeaders> {
    let mut headers = HttpHeaders::new();
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

fn parse_body(reader: &mut BufReader<&TcpStream>, content_length: usize) -> Result<HttpBody> {
    let mut buffer = vec![0; content_length];

    reader
        .read_exact(&mut buffer)
        .context("Failed to read body")?;

    Ok(HttpBody::Binary(buffer))
}
