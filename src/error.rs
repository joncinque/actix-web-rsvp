use {
    actix_web::{HttpResponse, ResponseError},
    csv::Error as CsvError,
    derive_more::Display,
    std::io::Error as IoError,
};

#[derive(Debug, Display)]
pub enum RsvpError {
    #[display(fmt = "Error with csv")]
    Csv(CsvError),
    #[display(fmt = "Error with io")]
    Io(IoError),
    #[display(fmt = "Error updating record ")]
    Update,
}

impl From<CsvError> for RsvpError {
    fn from(error: CsvError) -> Self {
        Self::Csv(error)
    }
}

impl From<IoError> for RsvpError {
    fn from(error: IoError) -> Self {
        Self::Io(error)
    }
}

impl ResponseError for RsvpError {
    fn error_response(&self) -> HttpResponse {
        println!("{}", self);
        match self {
            Self::Csv(_) => HttpResponse::InternalServerError().finish(),
            Self::Io(_) => HttpResponse::InternalServerError().finish(),
            Self::Update => HttpResponse::InternalServerError().finish(),
        }
    }
}
