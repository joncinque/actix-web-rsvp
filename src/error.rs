use {
    crate::{model::RsvpParams, state::AppState},
    actix_http::{body::Body, Response},
    actix_web::{
        body::MessageBody,
        dev::ServiceResponse,
        http::StatusCode,
        middleware::errhandlers::{ErrorHandlerResponse, ErrorHandlers},
        web, ResponseError, Result as ActixResult,
    },
    csv::Error as CsvError,
    derive_more::Display,
    lettre::{
        address::AddressError, error::Error as EmailError,
        transport::sendmail::Error as SendmailError, transport::stub::Error as StubTransportError,
    },
    serde_json::{json, Error as SerdeError},
    std::io::Error as IoError,
    tinytemplate::error::Error as TemplateError,
};

#[derive(Debug, Display)]
pub enum Error {
    #[display(fmt = "Error with csv: {}", _0)]
    Csv(CsvError),
    #[display(fmt = "Error with io: {}", _0)]
    Io(IoError),
    #[display(fmt = "Error updating record")]
    Update(RsvpParams),
    #[display(fmt = "Error on template: {}", _0)]
    Template(TemplateError),
    #[display(fmt = "Error on email: {}", _0)]
    Email(EmailError),
    #[display(fmt = "Error on address: {}", _0)]
    Address(AddressError),
    #[display(fmt = "Error on sendmail: {}", _0)]
    Sendmail(SendmailError),
    #[display(fmt = "Error on stub emailing: {}", _0)]
    Stub(StubTransportError),
    #[display(fmt = "Error on serde: {}", _0)]
    Serde(SerdeError),
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

impl From<AddressError> for Error {
    fn from(error: AddressError) -> Self {
        Self::Address(error)
    }
}

impl From<SendmailError> for Error {
    fn from(error: SendmailError) -> Self {
        Self::Sendmail(error)
    }
}

impl From<StubTransportError> for Error {
    fn from(error: StubTransportError) -> Self {
        Self::Stub(error)
    }
}

impl From<EmailError> for Error {
    fn from(error: EmailError) -> Self {
        Self::Email(error)
    }
}

impl From<SerdeError> for Error {
    fn from(error: SerdeError) -> Self {
        Self::Serde(error)
    }
}

impl ResponseError for Error {}

// Custom error handlers, to return HTML responses when an error occurs.
pub fn error_handlers() -> ErrorHandlers<Body> {
    ErrorHandlers::new()
        .handler(StatusCode::NOT_FOUND, not_found)
        .handler(StatusCode::INTERNAL_SERVER_ERROR, internal_server_error)
}

// Error handler for a 404 Page not found error.
fn not_found<B: MessageBody>(res: ServiceResponse<B>) -> ActixResult<ErrorHandlerResponse<B>> {
    let response = get_error_response(&res, "Page not found");
    Ok(ErrorHandlerResponse::Response(
        res.into_response(response.into_body()),
    ))
}

// Error handler for a 500 Internal Error
fn internal_server_error<B: MessageBody>(
    res: ServiceResponse<B>,
) -> ActixResult<ErrorHandlerResponse<B>> {
    let response = get_error_response(&res, "Internal error");
    Ok(ErrorHandlerResponse::Response(
        res.into_response(response.into_body()),
    ))
}

// Generic error handler.
fn get_error_response<B: MessageBody>(res: &ServiceResponse<B>, error: &str) -> Response<Body> {
    // Provide a fallback to a simple plain text response in case an error occurs during the
    // rendering of the error page.
    let fallback = |e: &str| {
        Response::build(res.status())
            .content_type("text/plain")
            .body(e.to_string())
    };

    let tt = res
        .request()
        .app_data::<web::Data<AppState<'_>>>()
        .map(|t| &t.get_ref().tt);
    match tt {
        Some(tt) => {
            let ctx = json!({
                "error" : error.to_string(),
                "status_code" : res.status().as_str().to_string(),
            });
            let body = tt.render("error.html", &ctx);

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
