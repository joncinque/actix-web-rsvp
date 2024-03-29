use {
    crate::error::Error,
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
};

pub const NUM_PHOTOS: usize = 1;

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ErrorContext {
    pub has_error: bool,
    pub error: String,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct IndexContext {
    pub admin: String,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct Attendance {
    pub attending: u32,
    pub attending_secondary: u32,
    pub attending_tertiary: u32,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct PhotosContext {
    pub admin: String,
    pub photo_indices: [usize; NUM_PHOTOS],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NameParams {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AddParams {
    pub name: String,
    pub email: String,
    pub plus_one_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RsvpParams {
    pub name: String,
    pub email: String,
    pub attending: bool,
    pub attending_secondary: bool,
    pub attending_tertiary: bool,
    pub meal_choice: String,
    pub dietary_restrictions: String,
    pub plus_one_attending: bool,
    pub plus_one_name: String,
    pub plus_one_meal_choice: String,
    pub plus_one_dietary_restrictions: String,
    pub comments: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RsvpModel {
    pub name: String,
    pub email: String,
    pub attending: bool,
    pub attending_secondary: bool,
    pub attending_tertiary: bool,
    pub meal_choice: String,
    pub dietary_restrictions: String,
    pub plus_one_attending: bool,
    pub plus_one_name: String,
    pub plus_one_meal_choice: String,
    pub plus_one_dietary_restrictions: String,
    pub comments: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RsvpModel {
    pub fn new_with_rsvp(params: &RsvpParams, datetime: DateTime<Utc>) -> Self {
        Self {
            name: params.name.clone(),
            email: params.email.clone(),
            attending: params.attending,
            attending_secondary: params.attending_secondary,
            attending_tertiary: params.attending_tertiary,
            meal_choice: params.meal_choice.clone(),
            dietary_restrictions: params.dietary_restrictions.clone(),
            plus_one_attending: params.plus_one_attending,
            plus_one_name: params.plus_one_name.clone(),
            plus_one_meal_choice: params.plus_one_meal_choice.clone(),
            plus_one_dietary_restrictions: params.plus_one_dietary_restrictions.clone(),
            comments: params.comments.clone(),
            created_at: datetime,
            updated_at: datetime,
        }
    }

    pub fn update(&mut self, params: &RsvpParams, datetime: DateTime<Utc>) -> Result<(), Error> {
        if self.name != params.name {
            return Err(Error::Update(params.clone()));
        }
        self.email = params.email.clone();
        self.attending = params.attending;
        self.attending_secondary = params.attending_secondary;
        self.attending_tertiary = params.attending_tertiary;
        if !params.meal_choice.is_empty() {
            self.meal_choice = params.meal_choice.clone();
        }
        self.dietary_restrictions = params.dietary_restrictions.clone();
        self.plus_one_attending = params.plus_one_attending;
        self.plus_one_name = params.plus_one_name.clone();
        if !params.plus_one_meal_choice.is_empty() {
            self.plus_one_meal_choice = params.plus_one_meal_choice.clone();
        }
        self.plus_one_dietary_restrictions = params.plus_one_dietary_restrictions.clone();
        self.comments = params.comments.clone();
        self.updated_at = datetime;
        Ok(())
    }

    pub fn new_with_add(params: &AddParams, datetime: DateTime<Utc>) -> Self {
        Self {
            name: params.name.clone(),
            email: params.email.clone(),
            attending: false,
            attending_secondary: false,
            attending_tertiary: false,
            meal_choice: String::default(),
            dietary_restrictions: String::default(),
            plus_one_attending: false,
            plus_one_name: params.plus_one_name.clone(),
            plus_one_meal_choice: String::default(),
            plus_one_dietary_restrictions: String::default(),
            comments: String::default(),
            created_at: datetime,
            updated_at: datetime,
        }
    }
}
