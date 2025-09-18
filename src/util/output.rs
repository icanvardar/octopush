use colored::Colorize;
use console::Emoji;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::{sync::Arc, time::Duration};

use crate::core::app::App;

static GEAR: Emoji<'_, '_> = Emoji("⚙️ ", "");
static CHECK: Emoji<'_, '_> = Emoji("✅ ", "✓ ");
static CROSS: Emoji<'_, '_> = Emoji("❌ ", "✗ ");

pub struct Runner {
    app: Arc<App>,
}

impl Runner {
    pub fn new(app: Arc<App>) -> Self {
        Self { app }
    }

    pub fn success(&self, message: &str) {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(
            out,
            "{}{} {}",
            CHECK,
            "SUCCESS".bold().bright_green(),
            message.green()
        );
    }

    pub fn error(&self, message: &str) {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(
            out,
            "{}{} {}",
            CROSS,
            "ERROR".bold().bright_red(),
            message.red()
        );
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

        match operation(&self.app) {
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

#[cfg(test)]
mod tests {
    use super::*;

    use nix::libc::{close, dup, dup2};
    use nix::unistd::pipe;
    use std::sync::Mutex;
    use std::{
        fs::File,
        io::{self, Read, Write},
        os::fd::AsRawFd,
    };

    #[test]
    fn test_success_and_error_output() -> Result<(), std::io::Error> {
        let runner = build_runner()?;

        let message = "success message";
        let output = capture_stdout(|| runner.success(message));
        let expected_output = format!("{}{} {}", CHECK, "SUCCESS", message) + "\n";

        assert_eq!(output, expected_output);

        let message = "error message";
        let output = capture_stdout(|| runner.error(message));
        let expected_output = format!("{}{} {}", CROSS, "ERROR", message) + "\n";

        assert_eq!(output, expected_output);

        Ok(())
    }

    fn build_runner() -> Result<Runner, std::io::Error> {
        let app = App::new()?;
        let runner = Runner::new(Arc::new(app));

        Ok(runner)
    }

    static STDOUT_LOCK: Mutex<()> = Mutex::new(());

    fn capture_stdout<F: FnOnce()>(f: F) -> String {
        let _guard = STDOUT_LOCK.lock().unwrap();

        let stdout_fd = io::stdout().as_raw_fd();

        let saved_fd = unsafe { dup(stdout_fd) };
        assert!(saved_fd >= 0, "dup(stdout) failed");

        let (read_fd, write_fd) = pipe().expect("pipe failed");

        let rc = unsafe { dup2(write_fd.as_raw_fd(), stdout_fd) };
        assert!(rc >= 0, "dup2 to stdout failed");

        f();
        let _ = io::stdout().flush();

        let rc = unsafe { dup2(saved_fd, stdout_fd) };
        assert!(rc >= 0, "dup2 restore failed");

        unsafe {
            let _ = close(saved_fd);
        }
        drop(write_fd);

        let mut output = String::new();
        let mut file: File = read_fd.into();
        let _ = file.read_to_string(&mut output);

        output
    }
}
