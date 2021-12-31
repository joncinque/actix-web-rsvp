use {
    actix_web::{HttpResponse, ResponseError},
    csv::Error as CsvError,
    derive_more::Display,
    std::io::Error as IoError,
};

#[derive(Debug, Display)]
pub enum Error {
    #[display(fmt = "Error with csv")]
    CsvError,
    #[display(fmt = "Error with io")]
    IoError,
}

impl From<CsvError> for Error {
    fn from(_error: CsvError) -> Self {
        Self::CsvError
    }
}

impl From<IoError> for Error {
    fn from(_error: IoError) -> Self {
        Self::IoError
    }
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match self {
            Error::CsvError => {
                println!("Error with csv");
                HttpResponse::InternalServerError().finish()
            }
            Error::IoError => {
                println!("Error with io");
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}
