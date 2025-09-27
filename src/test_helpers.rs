use crate::util::git;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::{env, sync::MutexGuard, time::SystemTime};
use std::{fs, io, time::UNIX_EPOCH};

static ENV_LOCK: Mutex<()> = Mutex::new(());

pub struct TempConfig {
    pub _guard: MutexGuard<'static, ()>,
    pub base: PathBuf,
    pub prev: Option<std::ffi::OsString>,
    pub repo: PathBuf,
}

impl TempConfig {
    pub fn new() -> Result<Self, io::Error> {
        let guard = match ENV_LOCK.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };

        let unique = format!(
            "octopush-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let base = env::temp_dir().join(unique);
        fs::create_dir_all(&base)?;

        let prev = env::var_os("XDG_CONFIG_HOME");
        unsafe {
            env::set_var("XDG_CONFIG_HOME", &base);
        }

        let repo = base.join("repo");
        Self::init_repo_at(&repo);

        Ok(TempConfig {
            _guard: guard,
            base,
            prev,
            repo,
        })
    }

    fn init_repo_at(path: &Path) {
        fs::create_dir_all(path).unwrap();
        let o = git::run_git(path, ["init"]).unwrap();
        assert!(o.status.success(), "git init failed: {:?}", o);
    }
}

impl Drop for TempConfig {
    fn drop(&mut self) {
        if let Some(v) = &self.prev {
            unsafe {
                env::set_var("XDG_CONFIG_HOME", v);
            }
        } else {
            unsafe {
                env::remove_var("XDG_CONFIG_HOME");
            }
        }
        let _ = fs::remove_dir_all(&self.base);
        // _lock is released automatically here
    }
}
