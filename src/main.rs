mod csvdb;
mod email;
mod error;
mod model;
mod state;

use {
    crate::{
        error::{error_handlers, Error},
        model::{
            AddParams, ErrorContext, IndexContext, NameParams, PhotosContext, RsvpParams,
            NUM_PHOTOS,
        },
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
            .service(web::resource("/photos").route(web::get().to(photos)))
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
    let admin = state.email.admin.clone();
    let ctx = serde_json::to_value(IndexContext { admin })?;
    let body = state.tt.render("index.html", &ctx).map_err(Error::from)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

/// Return the photos page
async fn photos(state: web::Data<AppState<'_>>) -> Result<HttpResponse> {
    let admin = state.email.admin.clone();
    let photo_indices = (1..=NUM_PHOTOS)
        .collect::<Vec<_>>()
        .try_into()
        .expect("Wrong size");
    let ctx = serde_json::to_value(PhotosContext {
        admin,
        photo_indices,
    })?;
    let body = state.tt.render("photos.html", &ctx).map_err(Error::from)?;
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
            Arg::with_name("port")
                .long("port")
                .short("p")
                .value_name("PORT")
                .help("Sets the port to bind to")
                .default_value("8080")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("admin")
                .value_name("ADMIN_EMAIL")
                .help("Sets the admin email address, receives a message on every RSVP")
                .required(true)
                .takes_value(true),
        )
        .get_matches();
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    // start http server
    let bind_address = format!("127.0.0.1:{}", matches.value_of("port").unwrap());
    HttpServer::new(move || {
        App::new()
            .service(Files::new("/static", "./static").prefer_utf8(true))
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(AppState::new(
                matches.value_of("admin").unwrap(),
                matches.value_of("csv").unwrap(),
                matches.value_of("from").unwrap(),
                matches.is_present("test"),
            )))
            .configure(app_config)
    })
    .bind(&bind_address)?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::csvdb::test::{test_add, test_db, test_rsvp},
        actix_http::body::BoxBody,
        actix_web::{
            body::MessageBody,
            dev::{Service, ServiceResponse},
            http::{
                header::{HeaderValue, CONTENT_TYPE},
                StatusCode,
            },
            test::{self, TestRequest},
            web::Form,
        },
    };

    trait BodyTest {
        fn into_str(self) -> String;
    }

    impl BodyTest for BoxBody {
        fn into_str(self) -> String {
            let b = self.try_into_bytes().unwrap();
            std::str::from_utf8(&b).unwrap().to_string()
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
        assert!(!resp.into_body().into_str().contains(NOT_FOUND_MESSAGE));

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
        assert!(!resp.into_body().into_str().contains(NOT_FOUND_MESSAGE));

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
        assert!(resp.into_body().into_str().contains(NOT_FOUND_MESSAGE));

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
        assert!(resp.into_body().into_str().contains(NOT_FOUND_MESSAGE));
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
        assert!(resp.into_body().into_str().contains("Success"));

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
        assert!(resp.into_body().into_str().contains("Confirmation"));
    }

    #[actix_rt::test]
    async fn handle_rsvp_integration_test() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState::default()))
                .configure(app_config),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/rsvp")
            .set_form(&test_rsvp())
            .to_request();
        let resp: ServiceResponse = app.call(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/html")
        );
        let (_, resp) = resp.into_parts();
        assert!(resp.into_body().into_str().contains("Confirmation"));
    }

    #[test]
    fn index_array() {
        let photo_indices: [usize; NUM_PHOTOS] = (1..=NUM_PHOTOS)
            .collect::<Vec<_>>()
            .try_into()
            .expect("Wrong size");
        assert_eq!(photo_indices.len(), NUM_PHOTOS);
    }
}
