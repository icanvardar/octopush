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
    pub mod path_completer;
    pub mod system;
}

#[cfg(any(test, feature = "test-helpers"))]
pub mod test_helpers;
