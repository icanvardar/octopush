use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
pub enum AuthType {
    #[default]
    None,
    SSH,
    GH,
}

impl FromStr for AuthType {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "none" => Ok(AuthType::None),
            "ssh" => Ok(AuthType::SSH),
            "gh" => Ok(AuthType::GH),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid auth type",
            )),
        }
    }
}
