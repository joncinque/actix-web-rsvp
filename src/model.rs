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
    pub attending_secondary: bool,
    pub attending_tertiary: bool,
    pub dietary_restrictions: String,
    pub plus_one_attending: bool,
    pub plus_one_name: String,
    pub plus_one_dietary_restrictions: String,
    pub comments: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RsvpModel {
    pub name: String,
    pub attending: bool,
    pub email: String,
    pub attending_secondary: bool,
    pub attending_tertiary: bool,
    pub dietary_restrictions: String,
    pub plus_one_attending: bool,
    pub plus_one_name: String,
    pub plus_one_dietary_restrictions: String,
    pub comments: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RsvpModel {
    pub fn new_with_params(params: &RsvpParams, datetime: DateTime<Utc>) -> Self {
        Self {
            name: params.name.clone(),
            attending: params.attending,
            email: params.email.clone(),
            attending_secondary: params.attending_secondary,
            attending_tertiary: params.attending_tertiary,
            dietary_restrictions: params.dietary_restrictions.clone(),
            plus_one_attending: params.plus_one_attending,
            plus_one_name: params.plus_one_name.clone(),
            plus_one_dietary_restrictions: params.plus_one_dietary_restrictions.clone(),
            comments: params.comments.clone(),
            created_at: datetime,
            updated_at: datetime,
        }
    }

    pub fn update(&mut self, params: &RsvpParams, datetime: DateTime<Utc>) -> Result<(), Error> {
        if self.name != params.name {
            return Err(Error::Update);
        }
        self.attending = params.attending;
        self.email = params.email.clone();
        self.attending_secondary = params.attending_secondary;
        self.attending_tertiary = params.attending_tertiary;
        self.dietary_restrictions = params.dietary_restrictions.clone();
        self.plus_one_attending = params.plus_one_attending;
        self.plus_one_name = params.plus_one_name.clone();
        self.plus_one_dietary_restrictions = params.plus_one_dietary_restrictions.clone();
        self.comments = params.comments.clone();
        self.updated_at = datetime;
        Ok(())
    }
}
