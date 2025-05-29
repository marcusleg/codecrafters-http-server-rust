pub enum HttpBody {
    Text(String),
    Binary(Vec<u8>),
}

impl HttpBody {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            HttpBody::Text(text) => text.as_bytes(),
            HttpBody::Binary(bytes) => bytes,
        }
    }
}
