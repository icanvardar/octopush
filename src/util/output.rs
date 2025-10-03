use colored::Colorize;
use console::Emoji;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::time::{Duration, Instant};

static GEAR: Emoji<'_, '_> = Emoji("⚙️ ", "");
static CHECK: Emoji<'_, '_> = Emoji("✅ ", "✓ ");
static CROSS: Emoji<'_, '_> = Emoji("❌ ", "✗ ");

pub struct Runner {}

impl Default for Runner {
    fn default() -> Self {
        Self::new()
    }
}

impl Runner {
    pub fn new() -> Self {
        Self {}
    }

    pub fn message(&self, message: &str) {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{}", message);
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
        F: FnOnce() -> Result<R, Box<dyn std::error::Error>>,
    {
        let (initial_prompt, success_prompt, error_prompt) = operation_type.get_spinner_prompt();
        let spinner = self.spinner(&initial_prompt);
        spinner.enable_steady_tick(Duration::from_millis(100));
        let started_at = Instant::now();

        match operation() {
            Ok(result) => {
                // Ensure spinner is visible for a minimal duration
                let min_duration = Duration::from_millis(600);
                let elapsed = started_at.elapsed();
                if elapsed < min_duration {
                    std::thread::sleep(min_duration - elapsed);
                }
                spinner.with_message(format!("{} {}", CHECK, success_prompt));
                self.success(&success_prompt);
                Ok(result)
            }
            Err(e) => {
                let min_duration = Duration::from_millis(600);
                let elapsed = started_at.elapsed();
                if elapsed < min_duration {
                    std::thread::sleep(min_duration - elapsed);
                }
                spinner.with_message(format!("{} {}", CROSS, error_prompt));
                self.error(&format!("{}: {}", error_prompt, e));
                Err(e)
            }
        }
    }
}

pub enum OperationType {
    AddProfile { profile_name: String },
    DeleteProfile { profile_name: String },
    ListProfiles,
    UseProfile { profile_name: String },
    GetProfile,
    ResetProfile,
}

impl OperationType {
    pub fn get_spinner_prompt(&self) -> (String, String, String) {
        match &self {
            OperationType::AddProfile { profile_name } => (
                format!("Adding new profile '{}'", profile_name),
                format!("Profile '{}' was successfully added", profile_name),
                format!("Failed to add profile '{}'", profile_name),
            ),
            OperationType::DeleteProfile { profile_name } => (
                format!("Deleting profile '{}'", profile_name),
                format!("Profile '{}' was successfully deleted", profile_name),
                format!("Failed to delete profile '{}'", profile_name),
            ),
            OperationType::ListProfiles => (
                "Fetching all profiles".to_string(),
                "Profiles successfully fetched".to_string(),
                "Failed to fetch profiles".to_string(),
            ),
            OperationType::UseProfile { profile_name } => (
                format!("Issuing profile '{}' for the repository", profile_name),
                format!(
                    "Profile '{}' has been successfully issued for the repository",
                    profile_name
                ),
                format!(
                    "Failed to issue profile '{}' for the repository",
                    profile_name
                ),
            ),
            OperationType::GetProfile => (
                "Fetching current profile".to_string(),
                "Profile successfully fetched".to_string(),
                "Failed to fetch profile".to_string(),
            ),
            OperationType::ResetProfile => (
                "Switching global profile".to_string(),
                "Global profile successfully set for the repository".to_string(),
                "Failed to reset global profile".to_string(),
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
        let runner = Runner::new();

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
