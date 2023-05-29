use {actix_web_rsvp::model::AddParams, awc::Client, clap::Parser};

/// Client for adding new people to the RSVP file
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// URL hosting the RSVP API
    #[arg(short, long, default_value_t = String::from("http://127.0.0.1:8080"))]
    url: String,

    /// New person's name
    #[arg()]
    name: String,

    /// New person's email address
    #[arg()]
    email: String,

    /// New person's plus-one's name
    #[arg()]
    plus_one: String,
}

#[actix_web::main]
async fn main() {
    let matches = Args::parse();
    let client = Client::default();

    // Create add params
    let params = AddParams {
        name: matches.name,
        email: matches.email,
        plus_one_name: matches.plus_one,
    };

    // Create request builder and send request
    let response = client
        .post(format!("{}/add", matches.url))
        .send_form(&params)
        .await;

    println!("Response: {:?}", response);
}
