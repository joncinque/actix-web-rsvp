use {
    crate::error::Error,
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NameParams {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RsvpParams {
    pub name: String,
    pub attending: bool,
    pub email: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RsvpModel {
    pub name: String,
    pub attending: bool,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RsvpModel {
    pub fn new_with_params(params: RsvpParams, datetime: DateTime<Utc>) -> Self {
        Self {
            name: params.name,
            attending: params.attending,
            email: params.email,
            created_at: datetime,
            updated_at: datetime,
        }
    }

    pub fn update(&mut self, params: RsvpParams, datetime: DateTime<Utc>) -> Result<(), Error> {
        if self.name != params.name {
            return Err(Error::Update);
        }
        self.attending = params.attending;
        self.email = params.email;
        self.updated_at = datetime;
        Ok(())
    }
}
