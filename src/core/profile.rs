use crate::core::auth::AuthType;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq)]
pub struct Profile {
    // this field is the same as profile_name
    pub id: String,
    pub name: String,
    pub email: String,
    pub auth_type: AuthType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_key_path: Option<String>,
}

impl Profile {
    pub fn build(
        id: String,
        name: String,
        email: String,
        auth_type: AuthType,
        hostname: Option<String>,
        ssh_key_path: Option<String>,
    ) -> Self {
        Profile {
            id,
            name,
            email,
            auth_type,
            hostname,
            ssh_key_path,
        }
    }
}
