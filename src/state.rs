use {
    crate::{csvdb::CsvDb, email::Email},
    std::{fs::OpenOptions, sync::Arc},
    tinytemplate::TinyTemplate,
    tokio::sync::RwLock,
};

static ERROR: &str = include_str!("../templates/error.html");
static FETCH: &str = include_str!("../templates/fetch.html");
static INDEX: &str = include_str!("../templates/index.html");
static RSVP: &str = include_str!("../templates/rsvp.html");
static CONFIRM: &str = include_str!("../templates/confirm.html");
static PHOTOS: &str = include_str!("../templates/photos.html");

pub struct AppState<'a> {
    pub test: bool,
    pub db: Arc<RwLock<CsvDb>>,
    pub tt: TinyTemplate<'a>,
    pub email: Email,
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
impl<'a> AppState<'a> {
    pub fn new<'arg>(
        admin: &'arg str,
        csv_filename: &'arg str,
        from: &'arg str,
        test: bool,
    ) -> Self {
        Self {
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
            email: Email::new(from, admin),
        }
    }

    #[cfg(test)]
    pub fn new_with_db(db: CsvDb) -> Self {
        Self {
            db: Arc::new(RwLock::new(db)),
            ..Self::default()
        }
    }
}

fn templates<'a>() -> TinyTemplate<'a> {
    let mut tt = TinyTemplate::new();
    tt.add_template("fetch.html", FETCH).unwrap();
    tt.add_template("index.html", INDEX).unwrap();
    tt.add_template("rsvp.html", RSVP).unwrap();
    tt.add_template("error.html", ERROR).unwrap();
    tt.add_template("confirm.html", CONFIRM).unwrap();
    tt.add_template("photos.html", PHOTOS).unwrap();
    tt
}
