use {
    actix_http::{body::Body, Response},
    actix_web::{web, HttpResponse, ResponseError, Result as ActixResult},
    actix_web::dev::ServiceResponse,
    actix_web::http::StatusCode,
    actix_web::middleware::errhandlers::{ErrorHandlerResponse, ErrorHandlers},
    csv::Error as CsvError,
    derive_more::Display,
    std::io::Error as IoError,
    tinytemplate::{error::Error as TemplateError, TinyTemplate},
};

#[derive(Debug, Display)]
pub enum Error {
    #[display(fmt = "Error with csv: {}", _0)]
    Csv(CsvError),
    #[display(fmt = "Error with io: {}", _0)]
    Io(IoError),
    #[display(fmt = "Error updating record")]
    Update,
    #[display(fmt = "Error on template: {}", _0)]
    Template(TemplateError)
}

impl From<CsvError> for Error {
    fn from(error: CsvError) -> Self {
        Self::Csv(error)
    }
}

impl From<IoError> for Error {
    fn from(error: IoError) -> Self {
        Self::Io(error)
    }
}

impl From<TemplateError> for Error {
    fn from(error: TemplateError) -> Self {
        Self::Template(error)
    }
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        println!("{}", self);
        match self {
            Self::Csv(_) => HttpResponse::InternalServerError().finish(),
            Self::Io(_) => HttpResponse::InternalServerError().finish(),
            Self::Update => HttpResponse::InternalServerError().finish(),
            Self::Template(_) => HttpResponse::InternalServerError().finish(),
        }
    }
}

// Custom error handlers, to return HTML responses when an error occurs.
pub fn error_handlers() -> ErrorHandlers<Body> {
    ErrorHandlers::new().handler(StatusCode::NOT_FOUND, not_found)
}

// Error handler for a 404 Page not found error.
fn not_found<B>(res: ServiceResponse<B>) -> ActixResult<ErrorHandlerResponse<B>> {
    let response = get_error_response(&res, "Page not found");
    Ok(ErrorHandlerResponse::Response(
        res.into_response(response.into_body()),
    ))
}

// Generic error handler.
fn get_error_response<B>(res: &ServiceResponse<B>, error: &str) -> Response<Body> {
    let request = res.request();

    // Provide a fallback to a simple plain text response in case an error occurs during the
    // rendering of the error page.
    let fallback = |e: &str| {
        Response::build(res.status())
            .content_type("text/plain")
            .body(e.to_string())
    };

    let tt = request
        .app_data::<web::Data<TinyTemplate<'_>>>()
        .map(|t| t.get_ref());
    match tt {
        Some(tt) => {
            let mut context = std::collections::HashMap::new();
            context.insert("error", error.to_owned());
            context.insert("status_code", res.status().as_str().to_owned());
            let body = tt.render("error.html", &context);

            match body {
                Ok(body) => Response::build(res.status())
                    .content_type("text/html")
                    .body(body),
                Err(_) => fallback(error),
            }
        }
        None => fallback(error),
    }
}

