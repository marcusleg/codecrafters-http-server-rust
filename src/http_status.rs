pub struct HttpStatus {
    pub(crate) code: u16,
    pub(crate) text: &'static str,
}

impl HttpStatus {
    pub(crate) const OK: HttpStatus = HttpStatus {
        code: 200,
        text: "OK",
    };
    pub(crate) const BAD_REQUEST: HttpStatus = HttpStatus {
        code: 400,
        text: "Bad Request",
    };
    pub(crate) const NOT_FOUND: HttpStatus = HttpStatus {
        code: 404,
        text: "Not Found",
    };
    pub(crate) const METHOD_NOT_ALLOWED: HttpStatus = HttpStatus {
        code: 405,
        text: "Method Not Allowed",
    };
    pub(crate) const INTERNAL_SERVER_ERROR: HttpStatus = HttpStatus {
        code: 500,
        text: "Internal Server Error",
    };
}
