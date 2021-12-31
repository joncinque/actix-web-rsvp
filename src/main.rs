use {
    actix_web::{
        middleware, web, App, Error, HttpResponse, HttpServer, Result, ResponseError,
    },
    csv::{Writer, Error as CsvError},
    derive_more::Display,
    serde::{Deserialize, Serialize},
    std::{path::PathBuf, sync::RwLock},
};

struct AppState {
    rsvp_csv: RwLock<PathBuf>,
}

fn app_config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("")
            .data(AppState {
                rsvp_csv: RwLock::new(PathBuf::from("rsvp.csv")),
            })
            .service(web::resource("/").route(web::get().to(index)))
            .service(web::resource("/rsvp").route(web::post().to(handle_rsvp)))
    );
}

#[derive(Debug, Display)]
pub enum RsvpError {
    #[display(fmt = "Error writing csv")]
    CsvError,
}

impl From<CsvError> for RsvpError {
    fn from(_error: CsvError) -> Self {
        Self::CsvError
    }
}

impl ResponseError for RsvpError {
    fn error_response(&self) -> HttpResponse {
        match self {
            RsvpError::CsvError => {
                println!("Issue writing the csv file");
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RsvpParams {
    name: String,
    attending: bool,
}

/// Return the main page
async fn index() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/form.html")))
}

/// Add an rsvp to the csv file
async fn handle_rsvp(
    state: web::Data<AppState>,
    params: web::Form<RsvpParams>,
) -> Result<HttpResponse, Error> {
    let path = state.rsvp_csv.write().unwrap();
    let mut wtr = Writer::from_path(path.as_path()).unwrap();
    let name = params.name.clone();
    wtr.serialize(params.into_inner()).map_err(|e| RsvpError::from(e))?;
    Ok(HttpResponse::Ok().content_type("text/plain").body(format!(
        "Your name is {}, and AppState path is: {}",
        name, path.to_str().unwrap()
    )))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .configure(app_config)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    use actix_web::body::{Body, ResponseBody};
    use actix_web::dev::{Service, ServiceResponse};
    use actix_web::http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
    use actix_web::test::{self, TestRequest};
    use actix_web::web::Form;

    trait BodyTest {
        fn as_str(&self) -> &str;
    }

    impl BodyTest for ResponseBody<Body> {
        fn as_str(&self) -> &str {
            match self {
                ResponseBody::Body(ref b) => match b {
                    Body::Bytes(ref by) => std::str::from_utf8(by).unwrap(),
                    _ => panic!(),
                },
                ResponseBody::Other(ref b) => match b {
                    Body::Bytes(ref by) => std::str::from_utf8(by).unwrap(),
                    _ => panic!(),
                },
            }
        }
    }

    #[actix_rt::test]
    async fn handle_rsvp_unit_test() {
        let state = TestRequest::default()
            .data(AppState {
                rsvp_csv: RwLock::new(PathBuf::from("test.csv")),
            })
            .to_http_request();
        let data = state.app_data::<actix_web::web::Data<AppState>>().unwrap();
        let params = Form(RsvpParams {
            name: "John".to_string(),
            attending: true,
        });
        let resp = handle_rsvp(data.clone(), params).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/plain")
        );
        assert_eq!(
            resp.body().as_str(),
            "Your name is John, and AppState path is: test.csv"
        );
    }

    #[actix_rt::test]
    async fn handle_rsvp_integration_test() {
        let mut app = test::init_service(App::new().configure(app_config)).await;
        let req = test::TestRequest::post()
            .uri("/rsvp")
            .set_form(&RsvpParams {
                name: "John".to_string(),
                attending: true,
            })
            .to_request();
        let resp: ServiceResponse = app.call(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/plain")
        );
        assert_eq!(
            resp.response().body().as_str(),
            "Your name is John, and AppState path is: rsvp.csv"
        );
    }
}
