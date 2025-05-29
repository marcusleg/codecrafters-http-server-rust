use crate::http_body::HttpBody;
use crate::http_headers::HttpHeaders;
use crate::http_status::HttpStatus;
use anyhow::{Context, Result};
use std::io::Write;
use std::net::TcpStream;

pub struct HttpResponse {
    pub(crate) status: HttpStatus,
    pub(crate) headers: HttpHeaders,
    pub(crate) body: Option<HttpBody>,
}

pub fn send(stream: &mut TcpStream, mut response: HttpResponse) -> Result<()> {
    send_status_line(stream, &mut response)?;
    send_headers(stream, &mut response)?;
    send_body(stream, &mut response)?;

    Ok(())
}

fn send_body(stream: &mut TcpStream, response: &mut HttpResponse) -> Result<()> {
    match &response.body {
        None => {}
        Some(HttpBody::Text(text)) => stream
            .write_all(text.as_bytes())
            .context("Failed to send body")?,
        Some(HttpBody::Binary(bytes)) => stream.write_all(&bytes).context("Failed to send body")?,
    };
    Ok(())
}

fn send_status_line(stream: &mut TcpStream, response: &mut HttpResponse) -> Result<()> {
    let status_line = format!(
        "HTTP/1.1 {} {}\r\n",
        response.status.code, response.status.text
    );
    stream
        .write_all(status_line.as_bytes())
        .context("Failed to send status line")?;
    Ok(())
}

fn send_headers(stream: &mut TcpStream, response: &mut HttpResponse) -> Result<()> {
    // set default content type
    if response.headers.get("Content-Type").is_none() {
        response
            .headers
            .insert("Content-Type".to_string(), "text/plain".to_string());
    }

    let content_length = match response.body {
        None => 0,
        Some(ref content) => match content {
            HttpBody::Text(text) => text.len(),
            HttpBody::Binary(bytes) => bytes.len(),
        },
    };

    response
        .headers
        .insert("Content-Length".to_string(), content_length.to_string());

    let headers_string = response
        .headers
        .iter()
        .map(|(k, v)| format!("{}: {}\r\n", k, v))
        .collect::<Vec<String>>()
        .join("");

    stream
        .write_all(headers_string.as_bytes())
        .context("Failed to send headers")?;

    // send empty line indicating the headers are complete
    stream
        .write_all("\r\n".as_bytes())
        .context("Failed to send empty line")?;

    Ok(())
}
