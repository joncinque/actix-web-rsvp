mod csvdb;
mod email;
mod error;
mod model;

use {
    crate::{
        csvdb::CsvDb,
        email::Email,
        error::{error_handlers, Error},
        model::{NameParams, RsvpParams},
    },
    actix_web::{middleware, web, App, Error as ActixError, HttpResponse, HttpServer, Result},
    chrono::Utc,
    clap::{App as ClapApp, Arg},
    log::{error, info},
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
    test: bool,
    db: Arc<RwLock<CsvDb>>,
    tt: TinyTemplate<'a>,
    email: Email,
}
impl<'a> Default for AppState<'a> {
    fn default() -> Self {
        Self {
            test: true,
            db: Arc::new(RwLock::new(CsvDb::default())),
            tt: templates(),
            email: Email::default(),
        }
    }
}

fn templates<'a>() -> TinyTemplate<'a> {
    let mut tt = TinyTemplate::new();
    tt.add_template("index.html", INDEX).unwrap();
    tt.add_template("rsvp.html", RSVP).unwrap();
    tt.add_template("error.html", ERROR).unwrap();
    tt.add_template("confirm.html", CONFIRM).unwrap();
    tt
}

fn rsvp_data<'a, 'arg>(
    admins: Vec<&'arg str>,
    csv_filename: &'arg str,
    from: &'arg str,
    test: bool,
) -> web::Data<AppState<'a>> {
    web::Data::new(AppState {
        test,
        db: Arc::new(RwLock::new(CsvDb::new(
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(csv_filename)
                .unwrap(),
        ))),
        tt: templates(),
        email: Email::new(from, &admins),
    })
}

fn app_config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("")
            .service(
                web::resource("/")
                    .route(web::get().to(index))
                    .route(web::post().to(handle_check)),
            )
            .service(web::resource("/rsvp").route(web::post().to(handle_rsvp)))
            .wrap(error_handlers()),
    );
}

/// Return the main page
async fn index(state: web::Data<AppState<'_>>) -> Result<HttpResponse> {
    let ctx = json!({"has_error": false, "error": ""});
    let body = state.tt.render("index.html", &ctx).map_err(Error::from)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(body))
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
        Ok(HttpResponse::Ok().content_type("text/html").body(body))
    } else {
        let ctx = json!({
            "has_error": true,
            "error" : NOT_FOUND_MESSAGE.to_string(),
        });
        let body = state.tt.render("index.html", &ctx).map_err(Error::from)?;
        Ok(HttpResponse::Ok().content_type("text/html").body(body))
    }
}

/// Add an rsvp to the csv file
async fn handle_rsvp(
    state: web::Data<AppState<'_>>,
    params: web::Form<RsvpParams>,
) -> Result<HttpResponse, ActixError> {
    let mut db = state.db.write().unwrap();
    let email = &state.email;
    db.update_time(Utc::now());
    let params = params.into_inner();
    info!("New RSVP! {:?}", params);
    match db.upsert(&params) {
        Ok(record) => {
            let contents = db.dump();
            if let Err(error) = email.send_csv(&params, contents, state.test).await {
                error!("Could not send confirmation email: {:?}", error);
            }
            let ctx = json!({
                "name" : record.name,
                "attending": record.attending,
                "email": record.email,
            });
            let body = state.tt.render("confirm.html", &ctx).map_err(Error::from)?;
            Ok(HttpResponse::Ok().content_type("text/html").body(body))
        }
        Err(error) => {
            if let Err(send_error) = email.send_rsvp_error(&error, &params, state.test).await {
                error!(
                    "Could not send error email: {:?}, original error: {:?}",
                    send_error, error
                );
            }
            Err(error.into())
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let matches = ClapApp::new("CSV RSVP Web Server")
        .version("0.1")
        .about("Web server for handling RSVPs to a CSV file")
        .arg(
            Arg::with_name("test")
                .short("t")
                .long("test")
                .help("Test mode, doesn't actually send emails")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("from")
                .value_name("FROM_EMAIL")
                .help("Sets the \"from\" email address")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("csv")
                .long("csv")
                .value_name("CSV_FILE")
                .help("Specifies a CSV file to use for RSVPs")
                .default_value("rsvp.csv")
                .required(true),
        )
        .arg(
            Arg::with_name("admin")
                .value_name("ADMIN_EMAIL")
                .help("Sets the admin email address, receives a message on every RSVP")
                .required(true)
                .takes_value(true)
                .multiple(true),
        )
        .get_matches();
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    // start http server
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(rsvp_data(
                matches.values_of("admin").unwrap().collect::<Vec<_>>(),
                matches.value_of("csv").unwrap(),
                matches.value_of("from").unwrap(),
                matches.is_present("test"),
            ))
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
                ..AppState::default()
            })
            .to_http_request();
        let data = state.app_data::<actix_web::web::Data<AppState>>().unwrap();

        // found
        let params = Form(NameParams {
            name: records[0].name.clone(),
        });
        let resp = handle_check(data.clone(), params).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/html")
        );
        assert!(!resp.body().as_str().contains(NOT_FOUND_MESSAGE));

        // not found
        let params = Form(NameParams {
            name: "something else".to_string(),
        });
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
            .app_data(web::Data::new(AppState::default()))
            .to_http_request();
        let data = state.app_data::<web::Data<AppState>>().unwrap();
        let params = Form(RsvpParams {
            name: "John".to_string(),
            attending: true,
            email: "test".to_string(),
        });
        let resp = handle_rsvp(data.clone(), params).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/html")
        );
        // TODO test more?
        //assert_eq!(resp.body().as_str(), "Your name is John");
    }

    #[actix_rt::test]
    async fn handle_rsvp_integration_test() {
        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState::default()))
                .configure(app_config),
        )
        .await;
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
            HeaderValue::from_static("text/html")
        );
        // TODO test more?
        //assert_eq!(resp.response().body().as_str(), format!("Your name is {}", name));
    }
}
