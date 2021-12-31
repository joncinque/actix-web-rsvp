use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RsvpParams {
    pub name: String,
    pub attending: bool,
}
