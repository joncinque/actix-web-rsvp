use {
    actix_web_rsvp::model::AddParams,
    awc::Client,
    clap::{App, Arg},
};

#[actix_web::main]
async fn main() {
    let matches = App::new("CSV RSVP Client")
        .version("0.1")
        .about("Client for adding new people to the RSVP file")
        .arg(
            Arg::with_name("url")
                .long("url")
                .value_name("URL")
                .help("Specifies the URL of the web server")
                .default_value("http://127.0.0.1:8080")
                .required(true),
        )
        .arg(
            Arg::with_name("name")
                .value_name("NAME")
                .help("New person's name")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("email")
                .value_name("EMAIL")
                .help("New person's email address")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("plus_one")
                .value_name("PLUS_ONE_NAME")
                .help("New person's plus one's name")
                .required(true)
                .default_value("")
                .takes_value(true),
        )
        .get_matches();

    let client = Client::default();

    // Create add params
    let params = AddParams {
        name: matches.value_of("name").unwrap().to_string(),
        email: matches.value_of("email").unwrap().to_string(),
        plus_one_name: matches.value_of("plus_one").unwrap().to_string(),
    };

    // Create request builder and send request
    let response = client
        .post(format!("{}/add", matches.value_of("url").unwrap()))
        .send_form(&params)
        .await;

    println!("Response: {:?}", response);
}
