use crate::core::{auth::AuthType, profile::Profile, project::Project};
use crate::util::git;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub struct App {}

// .octopush file format
// [profile_name]
// name = test
// email = test@email.com
// auth = [none,gh,ssh]
// gh_hostname = [optional(string), default(github.com)]
// ssh_key_path = [optional(string)]

trait ProfileManager {
    const CONFIG_DIR_NAME: &str = if cfg!(test) {
        "octopush-test"
    } else {
        "octopush"
    };
    const PROFILES_FILE_NAME: &str = "profiles.toml";
    const PROJECT_PROFILES_FILE_NAME: &str = "project_profiles.toml";

    fn base_config_dir() -> Result<PathBuf, io::Error> {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            return Ok(PathBuf::from(xdg));
        }

        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "HOME/USERPROFILE environment variable not set",
                )
            })?;

        Ok(PathBuf::from(home).join(".config"))
    }

    fn ensure_app_config_dir() -> Result<PathBuf, io::Error> {
        let dir = Self::base_config_dir()?.join(Self::CONFIG_DIR_NAME);
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn profiles_config_path() -> Result<PathBuf, io::Error> {
        let dir = Self::ensure_app_config_dir()?;
        Ok(dir.join(Self::PROFILES_FILE_NAME))
    }

    fn project_profiles_path() -> Result<PathBuf, io::Error> {
        let dir = Self::ensure_app_config_dir()?;
        Ok(dir.join(Self::PROJECT_PROFILES_FILE_NAME))
    }

    fn read_profile(profile_name: String) -> Result<Option<Profile>, io::Error> {
        let profiles = Self::read_profiles()?;

        Ok(profiles.get(&profile_name).cloned())
    }

    fn read_profiles() -> Result<HashMap<String, Profile>, io::Error> {
        let path = Self::profiles_config_path()?;
        let content = fs::read_to_string(&path).unwrap_or_default();
        if content.trim().is_empty() {
            return Ok(HashMap::new());
        }
        let profiles: HashMap<String, Profile> = toml::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("TOML parse error: {e}")))?;
        Ok(profiles)
    }

    fn write_profiles(profiles: &HashMap<String, Profile>) -> Result<(), io::Error> {
        let path = Self::profiles_config_path()?;
        let toml_string = toml::to_string_pretty(profiles).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("TOML serialize error: {e}"))
        })?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        file.write_all(toml_string.as_bytes())?;
        Ok(())
    }

    fn read_project_profile(repo_name: &str) -> Result<Option<Profile>, io::Error> {
        let map = Self::read_project_profiles()?;
        if let Some(profile_name) = map.get(repo_name) {
            let profiles = Self::read_profiles()?;
            Ok(profiles.get(profile_name).cloned())
        } else {
            Ok(None)
        }
    }

    fn read_project_profiles() -> Result<HashMap<String, String>, io::Error> {
        let path = Self::project_profiles_path()?;
        let content = fs::read_to_string(&path).unwrap_or_default();
        if content.trim().is_empty() {
            return Ok(HashMap::new());
        }
        let map: HashMap<String, String> = toml::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("TOML parse error: {e}")))?;
        Ok(map)
    }

    fn write_project_profiles(map: &HashMap<String, String>) -> Result<(), io::Error> {
        let path = Self::project_profiles_path()?;
        let toml_string = toml::to_string_pretty(map).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("TOML serialize error: {e}"))
        })?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        file.write_all(toml_string.as_bytes())?;
        Ok(())
    }

    fn add_profile(profile_name: String, profile: Profile) -> Result<(), io::Error> {
        let mut profiles = Self::read_profiles()?;
        if profiles.contains_key(&profile_name) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("profile '{}' already exists", profile_name),
            ));
        }
        profiles.insert(profile_name, profile);
        Self::write_profiles(&profiles)
    }

    fn update_profile(profile_name: String, profile: Profile) -> Result<(), io::Error> {
        let mut profiles = Self::read_profiles()?;

        match profiles.get(&profile_name) {
            Some(p) => match p.auth_type {
                AuthType::None => {
                    if profile.hostname.is_some() || profile.ssh_key_path.is_some() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "you cannot update 'hostname' or 'ssh_key_path' for 'none' auth type"
                            ),
                        ));
                    }
                }
                AuthType::SSH => {
                    if profile.hostname.is_some() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("you cannot update 'hostname' for 'ssh' auth type"),
                        ));
                    }
                }
                AuthType::GH => {
                    if profile.ssh_key_path.is_some() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("you cannot update 'ssh_key_path' for 'gh' auth type"),
                        ));
                    }
                }
            },
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("profile '{}' not found", profile_name),
                ));
            }
        }

        profiles.insert(profile_name, profile);
        Self::write_profiles(&profiles)
    }

    fn delete_profile(profile_name: String) -> Result<(), io::Error> {
        let mut profiles = Self::read_profiles()?;
        let removed = profiles.remove(&profile_name);
        if removed.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("profile '{}' not found", profile_name),
            ));
        }
        Self::write_profiles(&profiles)
    }

    fn apply_profile_to_repo(
        profile: &Profile,
        project_path: String,
    ) -> Result<(), std::io::Error> {
        let repo = Path::new(&project_path);
        git::ensure_repo(repo)?;

        git::set_local_identity(repo, &profile.name, &profile.email)?;

        let remote = git::get_remote_url(repo, "origin")?;

        match profile.auth_type {
            AuthType::SSH => {
                if let Some(key) = &profile.ssh_key_path {
                    git::ensure_ssh_command(repo, key)?;
                }
                if let Some(url) = remote {
                    if let Some((host, owner, repo_name)) = git::parse_remote(&url) {
                        if url.starts_with("https://") {
                            let ssh_url = git::to_ssh(&host, &owner, &repo_name);
                            let _ = git::set_remote_url(repo, "origin", &ssh_url)?;
                        }
                    }
                }
                let _ = git::clear_gh_credential_helper(repo)?;
            }
            AuthType::GH => {
                if let Some(url) = remote {
                    if let Some((host, owner, repo_name)) = git::parse_remote(&url) {
                        if url.starts_with("git@") || url.starts_with("ssh://") {
                            let https_url = git::to_https(&host, &owner, &repo_name);
                            let _ = git::set_remote_url(repo, "origin", &https_url)?;
                        }
                        let _gh_ok = git::is_gh_authenticated(&host);
                    }
                }
                let _ = git::set_gh_credential_helper(repo)?;
                let _ = git::clear_ssh_command(repo)?;
            }
            AuthType::None => {
                let _ = git::clear_ssh_command(repo)?;
                let _ = git::clear_gh_credential_helper(repo)?;
            }
        }

        Ok(())
    }
}

impl ProfileManager for App {}

impl App {
    pub fn add_profile(profile_name: String, profile: Profile) -> Result<(), io::Error> {
        <Self as ProfileManager>::add_profile(profile_name, profile)
    }

    // NOTE: it's not being used in the cli for now
    pub fn update_profile(profile_name: String, profile: Profile) -> Result<(), io::Error> {
        <Self as ProfileManager>::update_profile(profile_name, profile)
    }

    pub fn delete_profile(profile_name: String) -> Result<(), io::Error> {
        <Self as ProfileManager>::delete_profile(profile_name)
    }

    pub fn list_profiles() -> Result<HashMap<String, Profile>, io::Error> {
        <Self as ProfileManager>::read_profiles()
    }

    pub fn use_profile(profile_name: String, project_path: String) -> Result<(), io::Error> {
        let profile = <Self as ProfileManager>::read_profile(profile_name.clone())?;
        if profile.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("profile '{}' not found", profile_name),
            ));
        }
        let profile = profile.unwrap();

        let project = Project::new(project_path.clone())?;
        let repo_name = project.get_repo_name()?;

        let mut map = <Self as ProfileManager>::read_project_profiles()?;
        map.insert(repo_name, profile_name);
        <Self as ProfileManager>::write_project_profiles(&map)?;

        <Self as ProfileManager>::apply_profile_to_repo(&profile, project_path)?;

        Ok(())
    }

    pub fn get_project_profile(
        project_path: String,
    ) -> Result<(Profile, String) /* Profile and repo_name */, io::Error> {
        let project = Project::new(project_path)?;
        let repo_name = project.get_repo_name()?;

        match App::read_project_profile(&repo_name)? {
            Some(profile) => Ok((profile, repo_name)),
            None => Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("profile not found for '{}'", repo_name),
            )),
        }
    }

    pub fn reset_profile_for_project(project_path: String) -> Result<(), io::Error> {
        let project = Project::new(project_path.clone())?;
        let repo_name = project.get_repo_name()?;

        let mut map = <Self as ProfileManager>::read_project_profiles()?;
        map.remove(&repo_name);
        <Self as ProfileManager>::write_project_profiles(&map)?;

        let repo = std::path::Path::new(&project_path);
        git::ensure_repo(repo)?;
        let _ = git::unset_local(repo, "user.name");
        let _ = git::unset_local(repo, "user.email");
        let _ = git::clear_ssh_command(repo)?;
        let _ = git::clear_gh_credential_helper(repo)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_helpers::TempConfig;

    static CONFIG_DIR_NAME: &str = "octopush-test";
    static PROFILES_FILE_NAME: &str = "profiles.toml";
    static PROJECT_PROFILES_FILE_NAME: &str = "project_profiles.toml";
    static PROFILE_1_PROFILE_NAME: &str = "profile_1_profile_name";
    static PROFILE_2_PROFILE_NAME: &str = "profile_2_profile_name";
    static PROFILE_1_NAME: &str = "profile_1_name";
    static PROFILE_2_NAME: &str = "profile_2_name";
    static PROFILE_1_EMAIL: &str = "profile-1@email.com";
    static PROFILE_2_EMAIL: &str = "profile-2@email.com";
    static PROFILE_1_AUTH_TYPE: AuthType = AuthType::SSH;
    static PROFILE_2_AUTH_TYPE: AuthType = AuthType::GH;
    static SSH_KEY_PATH: &str = "~/.ssh/id_rsa";
    static HOSTNAME: &str = "mycompany.github.com";
    static REPO_1_NAME: &str = "repo_1";
    static REPO_2_NAME: &str = "repo_2";

    struct TestPM {}

    impl ProfileManager for TestPM {}

    // ProfileManager tests

    #[test]
    fn gets_config_dir_returns_path_when_exists() -> Result<(), std::io::Error> {
        let cfg = TempConfig::new()?;

        let config_dir = TestPM::base_config_dir()?;

        assert_eq!(config_dir, cfg.base);

        Ok(())
    }

    #[test]
    fn gets_app_config_dir_returns_path() -> Result<(), std::io::Error> {
        let _cfg = TempConfig::new();

        let path_exists = TestPM::ensure_app_config_dir()?.exists();

        assert_eq!(true, path_exists);

        Ok(())
    }

    #[test]
    fn gets_profiles_config_path_returns_path() -> Result<(), std::io::Error> {
        let cfg = TempConfig::new()?;

        let path = TestPM::profiles_config_path()?;

        let expected_path = cfg.base.join(CONFIG_DIR_NAME).join(PROFILES_FILE_NAME);

        assert_eq!(path, expected_path);

        Ok(())
    }

    #[test]
    fn gets_project_profiles_path() -> Result<(), std::io::Error> {
        let cfg = TempConfig::new()?;

        let path = TestPM::project_profiles_path()?;

        let expected_path = cfg
            .base
            .join(CONFIG_DIR_NAME)
            .join(PROJECT_PROFILES_FILE_NAME);

        assert_eq!(path, expected_path);

        Ok(())
    }

    #[test]
    fn reads_profile_when_no_profiles_exist_returns_none() -> Result<(), std::io::Error> {
        let _cfg = TempConfig::new();

        let result = TestPM::read_profile(PROFILE_1_NAME.to_owned())?;

        assert!(result.is_none());

        Ok(())
    }

    #[test]
    fn reads_profiles_when_no_profiles_exist_returns_empty_hashmap() -> Result<(), std::io::Error> {
        let _cfg = TempConfig::new();
        let result = TestPM::read_profiles()?;

        assert!(result.is_empty());

        Ok(())
    }

    #[test]
    fn reads_profile_when_profiles_exist_returns_profile() -> Result<(), std::io::Error> {
        let _cfg = TempConfig::new();
        let ((profile_name, profile), _) = get_profiles();

        let profiles: HashMap<String, Profile> =
            HashMap::from([(profile_name.to_string(), profile.clone())]);

        TestPM::write_profiles(&profiles)?;

        let mut found_profile = TestPM::read_profile(profile_name.to_string())?;

        assert!(!found_profile.is_none());
        assert_eq!(found_profile.take(), Some(profile.clone()));

        Ok(())
    }

    #[test]
    fn reads_profiles_when_profiles_exist_returns_profile() -> Result<(), std::io::Error> {
        let _cfg = TempConfig::new();
        let ((profile_1_name, profile_1), (profile_2_name, profile_2)) = get_profiles();

        let profiles: HashMap<String, Profile> = HashMap::from([
            (profile_1_name.to_string(), profile_1.clone()),
            (profile_2_name.to_string(), profile_2.clone()),
        ]);

        TestPM::write_profiles(&profiles)?;

        let found_profiles = TestPM::read_profiles()?;

        assert!(!found_profiles.is_empty());
        assert_eq!(
            found_profiles.get(profile_1_name).unwrap().to_owned(),
            profile_1
        );
        assert_eq!(
            found_profiles.get(profile_2_name).unwrap().to_owned(),
            profile_2
        );

        Ok(())
    }

    #[test]
    fn reads_project_profile_when_no_profiles_exist_returns_none() -> Result<(), std::io::Error> {
        let _cfg = TempConfig::new();

        let result = TestPM::read_project_profile(REPO_1_NAME)?;

        assert!(result.is_none());

        Ok(())
    }

    #[test]
    fn reads_project_profiles_when_no_profiles_exist_returns_empty_hashmap()
    -> Result<(), std::io::Error> {
        let _cfg = TempConfig::new();
        let result = TestPM::read_project_profiles()?;

        assert!(result.is_empty());

        Ok(())
    }

    #[test]
    fn reads_project_profile_when_profile_exists_returns_profile() -> Result<(), std::io::Error> {
        let _cfg = TempConfig::new();

        let ((profile_1_name, profile_1), _) = get_profiles();

        let profiles: HashMap<String, Profile> =
            HashMap::from([(profile_1_name.to_string(), profile_1.clone())]);

        TestPM::write_profiles(&profiles)?;

        let project_profiles: HashMap<String, String> =
            HashMap::from([(REPO_1_NAME.to_string(), profile_1_name.to_string())]);

        TestPM::write_project_profiles(&project_profiles)?;

        let result = TestPM::read_project_profile(REPO_1_NAME)?;

        assert_eq!(profile_1, result.unwrap());

        Ok(())
    }

    #[test]
    fn reads_project_profiles_when_profiles_exist_returns_hashmap() -> Result<(), std::io::Error> {
        let _cfg = TempConfig::new();

        let ((profile_1_name, profile_1), (profile_2_name, profile_2)) = get_profiles();

        let profiles: HashMap<String, Profile> = HashMap::from([
            (profile_1_name.to_string(), profile_1),
            (profile_2_name.to_string(), profile_2),
        ]);

        TestPM::write_profiles(&profiles)?;

        let project_profiles: HashMap<String, String> = HashMap::from([
            (REPO_1_NAME.to_string(), profile_1_name.to_string()),
            (REPO_2_NAME.to_string(), profile_2_name.to_string()),
        ]);

        TestPM::write_project_profiles(&project_profiles)?;

        let result = TestPM::read_project_profiles()?;

        assert_eq!(project_profiles, result);

        Ok(())
    }

    #[test]
    fn adds_profile_successfully() -> Result<(), std::io::Error> {
        let _cfg = TempConfig::new();

        let ((profile_1_name, profile_1), _) = get_profiles();

        TestPM::add_profile(profile_1_name.to_string(), profile_1)?;

        Ok(())
    }

    #[test]
    fn errors_on_duplicate_profile() -> Result<(), std::io::Error> {
        let _cfg = TempConfig::new();

        let ((profile_1_name, profile_1), _) = get_profiles();

        TestPM::add_profile(profile_1_name.to_string(), profile_1.clone())?;
        let result = TestPM::add_profile(profile_1_name.to_string(), profile_1);

        let err = result.unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(
            err.to_string(),
            format!("profile '{}' already exists", profile_1_name)
        );

        Ok(())
    }

    #[test]
    fn updates_profile_successfully() -> Result<(), std::io::Error> {
        let _cfg = TempConfig::new();

        let ((profile_1_name, mut profile_1), _) = get_profiles();

        TestPM::add_profile(profile_1_name.to_string(), profile_1.clone())?;
        TestPM::update_profile(profile_1_name.to_string(), profile_1.clone())?;

        profile_1.name = "isim".to_string();

        TestPM::update_profile(profile_1_name.to_string(), profile_1.clone())?;
        let updated_profile = TestPM::read_profile(profile_1_name.to_string())?;

        assert!(updated_profile.is_some());
        assert_eq!(updated_profile.unwrap(), profile_1);

        Ok(())
    }

    #[test]
    fn errors_on_nonexistent_profile() {
        let _cfg = TempConfig::new();

        let ((profile_1_name, profile_1), _) = get_profiles();

        let result = TestPM::update_profile(profile_1_name.to_string(), profile_1);
        let err = result.unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
        assert_eq!(
            err.to_string(),
            format!("profile '{}' not found", profile_1_name)
        );
    }

    #[test]
    fn errors_when_updating_wrong_profile_fields() {
        let _cfg = TempConfig::new();

        // profile auth types in order: 1 ssh 2 gh 3 none
        let ((profile_1_name, mut profile_1), (profile_2_name, mut profile_2)) = get_profiles();
        let profile_3_name = "profile_3_name";
        let mut profile_3 = Profile::build(
            profile_3_name.to_string(),
            "profile_3".to_string(),
            "profile_3@mail.com".to_string(),
            AuthType::None,
            None,
            None,
        );

        let profiles: HashMap<String, Profile> = HashMap::from([
            (profile_1_name.to_string(), profile_1.clone()),
            (profile_2_name.to_string(), profile_2.clone()),
            (profile_3_name.to_string(), profile_3.clone()),
        ]);
        let _ = TestPM::write_profiles(&profiles);

        // update wrong fields
        profile_1.hostname = Some("hostname.github.com".to_owned());
        profile_2.ssh_key_path = Some("some_path/to_key".to_owned());
        profile_3.hostname = Some("hostname.github.com".to_owned());

        let result = TestPM::update_profile(profile_1_name.to_string(), profile_1);
        let err = result.unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            format!("you cannot update 'hostname' for 'ssh' auth type"),
        );

        let result = TestPM::update_profile(profile_2_name.to_string(), profile_2);
        let err = result.unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            format!("you cannot update 'ssh_key_path' for 'gh' auth type"),
        );

        let result = TestPM::update_profile(profile_3_name.to_string(), profile_3);
        let err = result.unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            format!("you cannot update 'hostname' or 'ssh_key_path' for 'none' auth type"),
        );
    }

    #[test]
    fn delete_profile_successfully() -> Result<(), std::io::Error> {
        let _cfg = TempConfig::new();

        let ((profile_1_name, profile_1), _) = get_profiles();

        TestPM::add_profile(profile_1_name.to_string(), profile_1)?;

        let profile = TestPM::read_profile(profile_1_name.to_string())?;
        assert!(profile.is_some());

        TestPM::delete_profile(profile_1_name.to_string())?;

        let profile = TestPM::read_profile(profile_1_name.to_string())?;
        assert!(profile.is_none());

        Ok(())
    }

    #[test]
    fn errors_when_deleting_nonexistent_profile() {
        let _cfg = TempConfig::new();

        let profile_name = "profile_name";
        let result = TestPM::delete_profile(profile_name.to_owned());
        let err = result.unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
        assert_eq!(
            err.to_string(),
            format!("profile '{}' not found", profile_name)
        );
    }

    // App tests
    #[test]
    fn use_profile_errors_when_missing_profile() {
        let cfg = TempConfig::new().unwrap();

        let err = App::use_profile("nope".to_string(), cfg.repo.to_string_lossy().to_string())
            .unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn use_profile_applies_ssh_and_records_mapping() {
        let cfg = TempConfig::new().unwrap();

        // Arrange: write SSH profile and set an https remote to verify rewrite
        let ((ssh_profile_name, ssh_profile), _) = get_profiles();
        let profiles: HashMap<String, Profile> =
            HashMap::from([(ssh_profile_name.to_string(), ssh_profile.clone())]);
        TestPM::write_profiles(&profiles).unwrap();

        // set remote to https so it should be converted to ssh
        let _ = git::run_git(
            &cfg.repo,
            ["remote", "add", "origin", "https://github.com/acme/app.git"],
        )
        .unwrap();

        // Act
        App::use_profile(
            ssh_profile_name.to_string(),
            cfg.repo.to_string_lossy().to_string(),
        )
        .unwrap();

        // Assert: mapping exists for this repo
        let repo_name = Project::new(&cfg.repo).unwrap().get_repo_name().unwrap();
        let mapping = TestPM::read_project_profiles().unwrap();
        assert_eq!(mapping.get(&repo_name), Some(&ssh_profile_name.to_string()));

        // Assert: identity set
        let g1 = git::run_git(&cfg.repo, ["config", "--local", "user.name"]).unwrap();
        assert_eq!(String::from_utf8_lossy(&g1.stdout).trim(), ssh_profile.name);
        let g2 = git::run_git(&cfg.repo, ["config", "--local", "user.email"]).unwrap();
        assert_eq!(
            String::from_utf8_lossy(&g2.stdout).trim(),
            ssh_profile.email
        );

        // Assert: sshCommand set (skip strict check on Windows)
        let ssh_cmd = git::run_git(&cfg.repo, ["config", "--local", "core.sshCommand"]).unwrap();
        if cfg!(windows) {
            assert!(ssh_cmd.status.success());
        } else {
            let out = String::from_utf8_lossy(&ssh_cmd.stdout);
            assert!(out.contains(SSH_KEY_PATH));
        }

        // Assert: remote rewritten to ssh
        let url = git::get_remote_url(&cfg.repo, "origin").unwrap();
        assert_eq!(url.as_deref(), Some("git@github.com:acme/app.git"));

        // Assert: gh credential helper cleared
        let gh = git::run_git(
            &cfg.repo,
            ["config", "--local", "--get", "credential.helper"],
        )
        .unwrap();
        assert!(!gh.status.success());
    }

    #[test]
    fn use_profile_applies_gh_and_records_mapping() {
        let cfg = TempConfig::new().unwrap();

        // Arrange: write GH profile and set an ssh remote to verify rewrite
        let (_, gh_pair) = get_profiles();
        let (gh_profile_name, gh_profile) = gh_pair;
        let profiles: HashMap<String, Profile> =
            HashMap::from([(gh_profile_name.to_string(), gh_profile.clone())]);
        TestPM::write_profiles(&profiles).unwrap();

        // set remote to ssh so it should be converted to https
        let _ = git::run_git(
            &cfg.repo,
            ["remote", "add", "origin", "git@github.com:acme/app.git"],
        )
        .unwrap();

        // Act
        App::use_profile(
            gh_profile_name.to_string(),
            cfg.repo.to_string_lossy().to_string(),
        )
        .unwrap();

        // Assert: mapping exists for this repo
        let repo_name = Project::new(&cfg.repo).unwrap().get_repo_name().unwrap();
        let mapping = TestPM::read_project_profiles().unwrap();
        assert_eq!(mapping.get(&repo_name), Some(&gh_profile_name.to_string()));

        // Assert: identity set
        let g1 = git::run_git(&cfg.repo, ["config", "--local", "user.name"]).unwrap();
        assert_eq!(String::from_utf8_lossy(&g1.stdout).trim(), gh_profile.name);
        let g2 = git::run_git(&cfg.repo, ["config", "--local", "user.email"]).unwrap();
        assert_eq!(String::from_utf8_lossy(&g2.stdout).trim(), gh_profile.email);

        // Assert: sshCommand cleared
        let ssh =
            git::run_git(&cfg.repo, ["config", "--local", "--get", "core.sshCommand"]).unwrap();
        assert!(!ssh.status.success());

        // Assert: remote rewritten to https
        let url = git::get_remote_url(&cfg.repo, "origin").unwrap();
        assert_eq!(url.as_deref(), Some("https://github.com/acme/app.git"));

        // Assert: gh credential helper set
        let gh = git::run_git(&cfg.repo, ["config", "--local", "credential.helper"]).unwrap();
        assert_eq!(
            String::from_utf8_lossy(&gh.stdout).trim(),
            "!gh auth git-credential"
        );
        let gh2 = git::run_git(&cfg.repo, ["config", "--local", "credential.useHttpPath"]).unwrap();
        assert_eq!(String::from_utf8_lossy(&gh2.stdout).trim(), "true");
    }

    #[test]
    fn use_profile_applies_none_clears_auth_helpers() {
        let cfg = TempConfig::new().unwrap();

        // Arrange: create a NONE profile and write it
        let profile_name = "none_profile";
        let none_profile = Profile::build(
            profile_name.to_string(),
            "None User".to_string(),
            "none@example.com".to_string(),
            AuthType::None,
            None,
            None,
        );
        let profiles: HashMap<String, Profile> =
            HashMap::from([(profile_name.to_string(), none_profile.clone())]);
        TestPM::write_profiles(&profiles).unwrap();

        // Pre-set ssh and gh helpers to verify they get cleared
        git::ensure_ssh_command(&cfg.repo, "/tmp/fake_key").unwrap_or(());
        let _ = git::run_git(
            &cfg.repo,
            [
                "config",
                "--local",
                "credential.helper",
                "!gh auth git-credential",
            ],
        )
        .unwrap();

        // Act
        App::use_profile(
            profile_name.to_string(),
            cfg.repo.to_string_lossy().to_string(),
        )
        .unwrap();

        // Assert: ssh cleared
        let ssh =
            git::run_git(&cfg.repo, ["config", "--local", "--get", "core.sshCommand"]).unwrap();
        assert!(!ssh.status.success());
        // Assert: gh helper cleared
        let gh = git::run_git(
            &cfg.repo,
            ["config", "--local", "--get", "credential.helper"],
        )
        .unwrap();
        assert!(!gh.status.success());
    }

    #[test]
    fn get_project_profile_returns_selected_profile() {
        let cfg = TempConfig::new().unwrap();

        // Arrange: write a profile and use it
        let ((ssh_profile_name, ssh_profile), _) = get_profiles();
        let profiles: HashMap<String, Profile> =
            HashMap::from([(ssh_profile_name.to_string(), ssh_profile.clone())]);
        TestPM::write_profiles(&profiles).unwrap();

        App::use_profile(
            ssh_profile_name.to_string(),
            cfg.repo.to_string_lossy().to_string(),
        )
        .unwrap();

        let (found_profile, _) =
            App::get_project_profile(cfg.repo.to_string_lossy().to_string()).unwrap();

        // Assert
        assert_eq!(found_profile, ssh_profile);
    }

    #[test]
    fn reset_profile_clears_mapping_and_git() {
        let cfg = TempConfig::new().unwrap();

        // Arrange: write a profile and use it
        let ((ssh_profile_name, ssh_profile), _) = get_profiles();
        let profiles: HashMap<String, Profile> =
            HashMap::from([(ssh_profile_name.to_string(), ssh_profile.clone())]);
        TestPM::write_profiles(&profiles).unwrap();

        App::use_profile(
            ssh_profile_name.to_string(),
            cfg.repo.to_string_lossy().to_string(),
        )
        .unwrap();

        // Pre-verify mapping exists
        let repo_name = Project::new(&cfg.repo).unwrap().get_repo_name().unwrap();
        let mapping = TestPM::read_project_profiles().unwrap();
        assert!(mapping.get(&repo_name).is_some());

        // Act
        App::reset_profile_for_project(cfg.repo.to_string_lossy().to_string()).unwrap();

        // Assert: mapping removed
        let mapping_after = TestPM::read_project_profiles().unwrap();
        assert!(mapping_after.get(&repo_name).is_none());

        // Assert: git identity cleared
        let g1 = git::run_git(&cfg.repo, ["config", "--local", "--get", "user.name"]).unwrap();
        assert!(!g1.status.success());
        let g2 = git::run_git(&cfg.repo, ["config", "--local", "--get", "user.email"]).unwrap();
        assert!(!g2.status.success());
        let ssh =
            git::run_git(&cfg.repo, ["config", "--local", "--get", "core.sshCommand"]).unwrap();
        assert!(!ssh.status.success());
        let gh = git::run_git(
            &cfg.repo,
            ["config", "--local", "--get", "credential.helper"],
        )
        .unwrap();
        assert!(!gh.status.success());
    }

    fn get_profiles<'a>() -> ((&'a str, Profile), (&'a str, Profile)) {
        let profile_1 = Profile::build(
            PROFILE_1_PROFILE_NAME.to_string(),
            PROFILE_1_NAME.to_string(),
            PROFILE_1_EMAIL.to_string(),
            PROFILE_1_AUTH_TYPE,
            None,
            Some(SSH_KEY_PATH.to_string()),
        );

        let profile_2 = Profile::build(
            PROFILE_2_PROFILE_NAME.to_string(),
            PROFILE_2_NAME.to_string(),
            PROFILE_2_EMAIL.to_string(),
            PROFILE_2_AUTH_TYPE,
            Some(HOSTNAME.to_string()),
            None,
        );

        (
            (PROFILE_1_PROFILE_NAME, profile_1),
            (PROFILE_2_PROFILE_NAME, profile_2),
        )
    }
}
