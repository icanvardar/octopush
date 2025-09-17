use colored::Colorize;
use console::Emoji;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

use crate::core::app::App;

static GEAR: Emoji<'_, '_> = Emoji("⚙️ ", "");
static CHECK: Emoji<'_, '_> = Emoji("✅ ", "✓ ");
static CROSS: Emoji<'_, '_> = Emoji("❌ ", "✗ ");

pub struct Runner<'a> {
    app: &'a App,
}

impl<'a> Runner<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
    }

    pub fn success(&self, message: &str) {
        println!(
            "{}{} {}",
            CHECK,
            "SUCCESS".bold().bright_green(),
            message.green()
        );
    }

    pub fn error(&self, message: &str) {
        println!("{}{} {}", CROSS, "ERROR".bold().bright_red(), message.red());
    }

    pub fn spinner(&self, message: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(format!("{} {}", GEAR, message));
        pb
    }

    pub fn run<F, R>(
        &self,
        operation: F,
        operation_type: OperationType,
    ) -> Result<R, Box<dyn std::error::Error>>
    where
        F: FnOnce(&App) -> Result<R, Box<dyn std::error::Error>>,
    {
        let (initial_prompt, success_prompt, error_prompt) = operation_type.get_spinner_prompt();
        let spinner = self.spinner(&format!("{}", initial_prompt));
        spinner.enable_steady_tick(Duration::from_millis(100));

        match operation(self.app) {
            Ok(result) => {
                spinner.finish_with_message(format!("{} {}", CHECK, success_prompt));
                self.success(&format!("{}", success_prompt));
                Ok(result)
            }
            Err(e) => {
                spinner.finish_with_message(format!("{} {}", CROSS, error_prompt));
                self.error(&format!("{}: {}", error_prompt, e));
                Err(e)
            }
        }
    }
}

pub enum OperationType {
    Add,
    Delete,
    List,
    Use,
    Get,
    Reset,
}

impl OperationType {
    pub fn get_spinner_prompt(&self) -> (&str, &str, &str) {
        match &self {
            OperationType::Add => (
                "adding new profile",
                "profile successfully added",
                "unable to add new profile",
            ),
            OperationType::Delete => (
                "deleting profile",
                "profile successfully deleted",
                "unable to delete profile",
            ),
            OperationType::List => (
                "fetching profiles",
                "profiles fetched",
                "unable to fetch profiles",
            ),
            OperationType::Use => (
                "issuing profile for the repo",
                "profile issued for the repo",
                "unable to issue profile",
            ),
            OperationType::Get => (
                "fetching profile",
                "profile fetched",
                "unable to fetch profile",
            ),
            OperationType::Reset => (
                "switching the global profile",
                "global profile set for the repo",
                "unable to reset profile",
            ),
        }
    }
}
