use crate::http_body::HttpBody;
use crate::http_headers::HttpHeaders;
use crate::http_status::HttpStatus;
use anyhow::{Context, Result};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fmt;
use std::io::Write;
use std::net::TcpStream;

pub struct HttpResponse {
    pub(crate) status: HttpStatus,
    pub(crate) headers: HttpHeaders,
    pub(crate) body: Option<HttpBody>,
}

#[derive(Debug, PartialEq, Eq)]
enum ContentEncoding {
    None,
    Gzip,
}

impl fmt::Display for ContentEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn send(
    stream: &mut TcpStream,
    request_headers: HttpHeaders,
    mut response: HttpResponse,
) -> Result<()> {
    send_status_line(stream, &mut response.status)?;

    let content_encoding = determine_content_encoding(&request_headers);
    compress_body(&mut response.body, &content_encoding);

    set_content_encoding_header(&mut response, content_encoding);
    set_content_length_header(&mut response);
    set_content_type_header(&mut response);

    send_headers(stream, &mut response.headers)?;
    send_body(stream, &mut response.body)?;

    Ok(())
}

fn compress_body(response_body: &mut Option<HttpBody>, content_encoding: &ContentEncoding) {
    if response_body.is_none() {
        return;
    }

    match content_encoding {
        ContentEncoding::None => return,
        ContentEncoding::Gzip => {
            let bytes = response_body.as_ref().unwrap().as_bytes();
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder
                .write_all(bytes)
                .context("Failed to compress response body with gzip encoding");
            let compressed_bytes = encoder.finish().context("Failed to finish gzip encoder");
            *response_body = Some(HttpBody::Binary(compressed_bytes.unwrap()));
        }
    }
}

fn set_content_encoding_header(response: &mut HttpResponse, content_encoding: ContentEncoding) {
    if content_encoding == ContentEncoding::None {
        return;
    }

    response.headers.insert(
        "Content-Encoding".to_string(),
        content_encoding.to_string().to_lowercase(),
    );
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

fn determine_content_encoding(request_headers: &HttpHeaders) -> ContentEncoding {
    let accept_encoding = request_headers.get("accept-encoding");
    if accept_encoding.is_none() {
        return ContentEncoding::None;
    }

    let mut encodings = accept_encoding.unwrap().split(", ");
    if encodings.find(|s| s.contains("gzip")).is_some() {
        return ContentEncoding::Gzip;
    }
    ContentEncoding::None
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
