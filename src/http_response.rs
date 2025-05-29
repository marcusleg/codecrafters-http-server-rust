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
    send_status_line(stream, &mut response.status)?;

    set_content_length_header(&mut response);
    set_content_type_header(&mut response);

    send_headers(stream, &mut response.headers)?;
    send_body(stream, &mut response.body)?;

    Ok(())
}

fn set_content_length_header(response: &mut HttpResponse) {
    let content_lemgth = determine_content_length(&response.body);
    if content_lemgth > 0 {
        response
            .headers
            .insert("Content-Length".to_string(), content_lemgth.to_string());
    }
}

fn set_content_type_header(response: &mut HttpResponse) {
    if response.headers.get("Content-Type").is_some() {
        return;
    }

    let content_type = determine_content_type(&response.body);
    response
        .headers
        .insert("Content-Type".to_string(), content_type);
}

fn determine_content_length(body: &Option<HttpBody>) -> usize {
    match body {
        None => 0,
        Some(HttpBody::Text(text)) => text.len(),
        Some(HttpBody::Binary(bytes)) => bytes.len(),
    }
}

fn determine_content_type(body: &Option<HttpBody>) -> String {
    match body {
        None => "text/plain".to_string(),
        Some(HttpBody::Text(_)) => "text/plain".to_string(),
        Some(HttpBody::Binary(_)) => "application/octet-stream".to_string(),
    }
}

fn send_body(stream: &mut TcpStream, body: &Option<HttpBody>) -> Result<()> {
    match &body {
        None => {}
        Some(HttpBody::Text(text)) => stream
            .write_all(text.as_bytes())
            .context("Failed to send body")?,
        Some(HttpBody::Binary(bytes)) => stream.write_all(&bytes).context("Failed to send body")?,
    };
    Ok(())
}

fn send_status_line(stream: &mut TcpStream, status: &HttpStatus) -> Result<()> {
    let status_line = format!("HTTP/1.1 {} {}\r\n", status.code, status.text);
    stream
        .write_all(status_line.as_bytes())
        .context("Failed to send status line")?;
    Ok(())
}

fn send_headers(stream: &mut TcpStream, headers: &HttpHeaders) -> Result<()> {
    let headers_string = headers
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
