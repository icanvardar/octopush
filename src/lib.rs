pub mod core {
    pub mod app;
    pub mod auth;
    pub mod profile;
    pub mod project;
}

pub mod util {
    pub mod cli;
    pub mod git;
    pub mod output;
    pub mod system;
}

#[cfg(test)]
pub mod test_helpers;
