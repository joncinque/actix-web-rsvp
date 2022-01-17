use {
    crate::error::RsvpError,
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RsvpParams {
    pub name: String,
    #[serde(default)]
    pub attending: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RsvpModel {
    // rust-csv doesn't support this unfortunately, PR?
    //#[serde(flatten)]
    //pub params: RsvpParams,
    pub name: String,
    pub attending: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RsvpModel {
    pub fn new_with_params(params: RsvpParams, datetime: DateTime<Utc>) -> Self {
        Self {
            name: params.name,
            attending: params.attending,
            created_at: datetime,
            updated_at: datetime,
        }
    }

    pub fn update(&mut self, params: RsvpParams, datetime: DateTime<Utc>) -> Result<(), RsvpError> {
        if self.name != params.name {
            return Err(RsvpError::Update);
        }
        self.attending = params.attending;
        self.updated_at = datetime;
        Ok(())
    }
}
