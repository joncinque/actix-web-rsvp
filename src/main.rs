mod csvdb;
mod email;
mod error;
mod model;
mod state;

use {
    crate::{
        error::{error_handlers, Error},
        model::{AddParams, ErrorContext, NameParams, RsvpParams},
        state::AppState,
    },
    actix_files::Files,
    actix_web::{middleware, web, App, Error as ActixError, HttpResponse, HttpServer, Result},
    chrono::Utc,
    clap::{App as ClapApp, Arg},
    log::{error, info},
    tinytemplate::TinyTemplate,
};

static NOT_FOUND_MESSAGE: &str = "Your name was not found, sorry! Please use the exact name from the invitation email, or contact the admin if you think something is wrong.";

fn name_not_found(tt: &TinyTemplate<'_>) -> Result<HttpResponse, ActixError> {
    let ctx = serde_json::to_value(ErrorContext {
        has_error: true,
        error: NOT_FOUND_MESSAGE.to_string(),
    })?;
    let body = tt.render("fetch.html", &ctx).map_err(Error::from)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

fn app_config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("")
            .service(web::resource("/").route(web::get().to(index)))
            .service(
                web::resource("/fetch")
                    .route(web::get().to(fetch))
                    .route(web::post().to(handle_fetch)),
            )
            .service(web::resource("/rsvp").route(web::post().to(handle_rsvp)))
            .service(web::resource("/add").route(web::post().to(handle_add)))
            .wrap(error_handlers()),
    );
}

/// Return the index page
async fn index(state: web::Data<AppState<'_>>) -> Result<HttpResponse> {
    let ctx = serde_json::to_value(ErrorContext::default())?;
    let body = state.tt.render("index.html", &ctx).map_err(Error::from)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

/// Return the fetch page
async fn fetch(state: web::Data<AppState<'_>>) -> Result<HttpResponse> {
    let ctx = serde_json::to_value(ErrorContext::default())?;
    let body = state.tt.render("fetch.html", &ctx).map_err(Error::from)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

/// Get an existing rsvp
async fn handle_fetch(
    state: web::Data<AppState<'_>>,
    params: web::Form<NameParams>,
) -> Result<HttpResponse, ActixError> {
    if params.name.is_empty() {
        return name_not_found(&state.tt);
    }
    let mut db = state.db.write().unwrap();
    let record = db.get(&params.into_inner().name)?;
    if let Some(record) = record {
        let ctx = serde_json::to_value(record)?;
        let body = state.tt.render("rsvp.html", &ctx).map_err(Error::from)?;
        Ok(HttpResponse::Ok().content_type("text/html").body(body))
    } else {
        name_not_found(&state.tt)
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
            let ctx = serde_json::to_value(record)?;
            let body = state.tt.render("confirm.html", &ctx).map_err(Error::from)?;
            Ok(HttpResponse::Ok().content_type("text/html").body(body))
        }
        Err(error) => {
            // it'd be better to do this generically, but oh well!
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

/// Add a person to the csv file
async fn handle_add(
    state: web::Data<AppState<'_>>,
    params: web::Form<AddParams>,
) -> Result<HttpResponse, ActixError> {
    let mut db = state.db.write().unwrap();
    db.update_time(Utc::now());
    let params = params.into_inner();
    info!("New person! {:?}", params);
    let model = db.insert(&params)?;
    Ok(HttpResponse::Ok()
        .content_type("text/plain")
        .body(format!("Success adding!\n{:?}", model)))
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
            .service(Files::new("/static", "./static").prefer_utf8(true))
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(AppState::new(
                matches.values_of("admin").unwrap().collect::<Vec<_>>(),
                matches.value_of("csv").unwrap(),
                matches.value_of("from").unwrap(),
                matches.is_present("test"),
            )))
            .configure(app_config)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::csvdb::test::{test_add, test_db, test_rsvp},
        actix_web::{
            body::{Body, ResponseBody},
            dev::{Service, ServiceResponse},
            http::{header::CONTENT_TYPE, HeaderValue, StatusCode},
            test::{self, TestRequest},
            web::Form,
        },
    };

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
    async fn handle_fetch_unit_test() {
        let mut db = test_db(10);
        let records = db.get_all().unwrap();
        let state = TestRequest::default()
            .data(AppState::new_with_db(db))
            .to_http_request();
        let data = state.app_data::<actix_web::web::Data<AppState>>().unwrap();

        // found
        let params = Form(NameParams {
            name: records[0].name.clone(),
        });
        let resp = handle_fetch(data.clone(), params).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/html")
        );
        assert!(!resp.body().as_str().contains(NOT_FOUND_MESSAGE));

        // found plus-one
        let params = Form(NameParams {
            name: records[0].plus_one_name.clone(),
        });
        let resp = handle_fetch(data.clone(), params).await.unwrap();
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
        let resp = handle_fetch(data.clone(), params).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/html")
        );
        println!("{}", resp.body().as_str());
        assert!(resp.body().as_str().contains(NOT_FOUND_MESSAGE));

        // not found empty
        let params = Form(NameParams {
            name: "".to_string(),
        });
        let resp = handle_fetch(data.clone(), params).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/html")
        );
        println!("{}", resp.body().as_str());
        assert!(resp.body().as_str().contains(NOT_FOUND_MESSAGE));
    }

    #[actix_rt::test]
    async fn handle_add_unit_test() {
        let state = TestRequest::default()
            .app_data(web::Data::new(AppState::default()))
            .to_http_request();
        let data = state.app_data::<web::Data<AppState>>().unwrap();
        let params = Form(test_add());
        let resp = handle_add(data.clone(), params).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/plain")
        );
        assert!(resp.body().as_str().contains("Success"));

        let params = Form(test_add());
        let _error = handle_add(data.clone(), params).await.unwrap_err();
    }

    #[actix_rt::test]
    async fn handle_rsvp_unit_test() {
        let state = TestRequest::default()
            .app_data(web::Data::new(AppState::default()))
            .to_http_request();
        let data = state.app_data::<web::Data<AppState>>().unwrap();
        let params = Form(test_rsvp());
        let resp = handle_rsvp(data.clone(), params).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/html")
        );
        assert!(resp.body().as_str().contains("Confirmation"));
    }

    #[actix_rt::test]
    async fn handle_rsvp_integration_test() {
        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState::default()))
                .configure(app_config),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/rsvp")
            .set_form(&test_rsvp())
            .to_request();
        let mut resp: ServiceResponse = app.call(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/html")
        );
        assert!(resp.take_body().as_str().contains("Confirmation"));
    }
}
