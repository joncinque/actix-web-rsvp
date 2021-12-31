use {
    actix_web::{HttpResponse, ResponseError},
    csv::Error as CsvError,
    derive_more::Display,
};

#[derive(Debug, Display)]
pub enum Error {
    #[display(fmt = "Error writing csv")]
    CsvError,
}

impl From<CsvError> for Error {
    fn from(_error: CsvError) -> Self {
        Self::CsvError
    }
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match self {
            Error::CsvError => {
                println!("Issue writing the csv file");
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}
