mod csvdb;
mod error;
mod model;

use {
    crate::{csvdb::CsvDb, model::{NameParams, RsvpParams}, error::{Error, error_handlers}},
    actix_web::{middleware, web, App, Error as ActixError, HttpResponse, HttpServer, Result},
    chrono::Utc,
    serde_json::json,
    std::{
        fs::OpenOptions,
        sync::{Arc, RwLock},
    },
    tinytemplate::TinyTemplate,
};

static ERROR: &str = include_str!("../templates/error.html");
static NOT_FOUND_MESSAGE: &str = "Your name wasn't found, sorry!";
static INDEX: &str = include_str!("../templates/index.html");
static RSVP: &str = include_str!("../templates/rsvp.html");
static CONFIRM: &str = include_str!("../templates/confirm.html");

struct AppState<'a> {
    db: Arc<RwLock<CsvDb>>,
    tt: TinyTemplate<'a>,
}

fn templates<'a>() -> TinyTemplate<'a> {
    let mut tt = TinyTemplate::new();
    tt.add_template("index.html", INDEX).unwrap();
    tt.add_template("rsvp.html", RSVP).unwrap();
    tt.add_template("error.html", ERROR).unwrap();
    tt.add_template("confirm.html", CONFIRM).unwrap();
    tt
}

fn app_config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("")
            .data(AppState {
                db: Arc::new(RwLock::new(CsvDb::new(
                    OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .open("rsvp.csv")
                        .unwrap(),
                ))),
                tt: templates(),
            })
            .service(web::resource("/")
                .route(web::get().to(index))
                .route(web::post().to(handle_check))
            )
            .service(web::resource("/rsvp").route(web::post().to(handle_rsvp)))
            .service(web::scope("").wrap(error_handlers()))
    );
}

/// Return the main page
async fn index(
    state: web::Data<AppState<'_>>,
) -> Result<HttpResponse> {
    let ctx = json!({"has_error": false, "error": ""});
    let body = state.tt.render("index.html", &ctx).map_err(Error::from)?;
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(body))
}

/// Get an existing rsvp
async fn handle_check(
    state: web::Data<AppState<'_>>,
    params: web::Form<NameParams>,
) -> Result<HttpResponse, ActixError> {
    let mut db = state.db.write().unwrap();
    let record = db.get(&params.into_inner().name)?;
    if let Some(record) = record {
        let ctx = json!({
            "name" : record.name,
            "attending": record.attending,
            "email": record.email,
        });
        let body = state.tt.render("rsvp.html", &ctx).map_err(Error::from)?;
        Ok(HttpResponse::Ok()
            .content_type("text/html")
            .body(body))
    } else {
        let ctx = json!({
            "has_error": true,
            "error" : NOT_FOUND_MESSAGE.to_string(),
        });
        let body = state.tt.render("index.html", &ctx).map_err(Error::from)?;
        Ok(HttpResponse::Ok()
            .content_type("text/html")
            .body(body))
    }
}

/// Add an rsvp to the csv file
async fn handle_rsvp(
    state: web::Data<AppState<'_>>,
    params: web::Form<RsvpParams>,
) -> Result<HttpResponse, ActixError> {
    let mut db = state.db.write().unwrap();
    db.update_time(Utc::now());
    let record = db.upsert(params.into_inner())?;
    let ctx = json!({
        "name" : record.name,
        "attending": record.attending,
        "email": record.email,
    });
    let body = state.tt.render("confirm.html", &ctx).map_err(Error::from)?;
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(body))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=debug");
    env_logger::init();

    // start http server
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

    use crate::csvdb::test::test_db;
    use actix_web::body::{Body, ResponseBody};
    use actix_web::dev::{Service, ServiceResponse};
    use actix_web::http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
    use actix_web::test::{self, TestRequest};
    use actix_web::web::Form;
    use tempfile::tempfile;

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
    async fn handle_check_unit_test() {
        let mut db = test_db(10);
        let records = db.get_all().unwrap();
        let state = TestRequest::default()
            .data(AppState {
                db: Arc::new(RwLock::new(db)),
                tt: templates(),
            })
            .to_http_request();
        let data = state.app_data::<actix_web::web::Data<AppState>>().unwrap();

        // found
        let params = Form(NameParams { name: records[0].name.clone() });
        let resp = handle_check(data.clone(), params).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/html")
        );
        assert!(!resp.body().as_str().contains(NOT_FOUND_MESSAGE));

        // not found
        let params = Form(NameParams { name: "something else".to_string() });
        let resp = handle_check(data.clone(), params).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/html")
        );
        assert!(resp.body().as_str().contains("sorry!"));
    }

    #[actix_rt::test]
    async fn handle_rsvp_unit_test() {
        let state = TestRequest::default()
            .data(AppState {
                db: Arc::new(RwLock::new(CsvDb::new(tempfile().unwrap()))),
                tt: templates(),
            })
            .to_http_request();
        let data = state.app_data::<actix_web::web::Data<AppState>>().unwrap();
        let params = Form(RsvpParams {
            name: "John".to_string(),
            attending: true,
            email: "test".to_string(),
        });
        let resp = handle_rsvp(data.clone(), params).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/plain")
        );
        // TODO test more?
        //assert_eq!(resp.body().as_str(), "Your name is John");
    }

    #[actix_rt::test]
    async fn handle_rsvp_integration_test() {
        let mut app = test::init_service(App::new().configure(app_config)).await;
        let name = "John\n, wow";
        let req = test::TestRequest::post()
            .uri("/rsvp")
            .set_form(&RsvpParams {
                name: name.to_string(),
                attending: true,
                email: "test".to_string(),
            })
            .to_request();
        let resp: ServiceResponse = app.call(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/plain")
        );
        // TODO test more?
        //assert_eq!(resp.response().body().as_str(), format!("Your name is {}", name));
    }
}
