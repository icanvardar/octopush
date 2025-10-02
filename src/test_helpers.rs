use crate::core::{auth::AuthType, profile::Profile};
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
    pub gh_dir: PathBuf,
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

        let gh_dir = base.join("gh");
        Self::init_gh_hosts(&gh_dir);

        Ok(TempConfig {
            _guard: guard,
            base,
            prev,
            repo,
            gh_dir,
        })
    }

    fn init_repo_at(path: &Path) {
        fs::create_dir_all(path).unwrap();
        let o = git::run_git(path, ["init"]).unwrap();
        assert!(o.status.success(), "git init failed: {:?}", o);
    }

    fn init_gh_hosts(path: &Path) {
        fs::create_dir_all(path).unwrap();
        fs::write(
            path.join("hosts.yml"),
            "github.com:\n oauth_token: dummy\n user: someone\n",
        )
        .unwrap();
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

static PROFILE_NAMES: [&str; 3] = ["profile_1", "profile_2", "profile_3"];
static NAMES: [&str; 3] = ["profile_1_name", "profile_2_name", "profile_3_name"];
static EMAILS: [&str; 3] = ["profile_1_email", "profile_2_email", "profile_3_email"];
static AUTH_TYPES: [AuthType; 3] = [AuthType::None, AuthType::SSH, AuthType::GH];
static HOSTNAMES: [Option<&str>; 3] = [None, None, Some("github.com")];
static SSH_KEY_PATHS: [Option<&str>; 3] = [None, Some("~/.ssh/id_ed25519"), None];

pub fn get_profiles<'a>() -> ([&'a str; 3], [Profile; 3]) /* profile_1, profile_2, profile_3 */ {
    let mut profiles: Vec<Profile> = Vec::new();
    for i in 0..PROFILE_NAMES.len() {
        profiles.push(Profile::build(
            NAMES[i].to_string(),
            EMAILS[i].to_string(),
            AUTH_TYPES[i],
            HOSTNAMES[i].map(|s| s.to_string()),
            SSH_KEY_PATHS[i].map(|s| s.to_string()),
        ));
    }

    let profiles: [Profile; 3] = [
        profiles[0].clone(),
        profiles[1].clone(),
        profiles[2].clone(),
    ];

    (PROFILE_NAMES, profiles)
}
