use serde::{Deserialize, Serialize};

use crate::core::auth::AuthType;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub struct Project {
    pub path: String,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct Profile {
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
        name: String,
        email: String,
        auth_type: AuthType,
        hostname: Option<String>,
        ssh_key_path: Option<String>,
    ) -> Self {
        Profile {
            name,
            email,
            auth_type,
            hostname,
            ssh_key_path,
        }
    }
}

pub struct App {
    pub profiles: Option<Vec<Profile>>,
    pub global_profile_name: Option<Profile>,
    pub current_scopes: Option<HashMap<Project, Profile>>,
}

// .octopush file format
// [profile_name]
// name = test
// email = test@email.com
// auth = [none,gh,ssh]
// gh_hostname = [optional(string), default(github.com)]
// ssh_key_path = [optional(string)]

trait ProfileManager {
    const CONFIG_DIR_NAME: &str = "octopush";
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
        let profiles = Self::read_profiles()?;

        Ok(profiles.get(repo_name).cloned())
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
        if !profiles.contains_key(&profile_name) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("profile '{}' not found", profile_name),
            ));
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

        let repo_name = get_repo_name(Path::new(&project_path))?;

        let mut map = <Self as ProfileManager>::read_project_profiles()?;
        map.insert(repo_name, profile_name);
        <Self as ProfileManager>::write_project_profiles(&map)
    }

    pub fn get_project_profile(project_path: String) -> Result<Profile, io::Error> {
        let repo_name = get_repo_name(Path::new(&project_path))?;

        match App::read_project_profile(&repo_name)? {
            Some(profile) => Ok(profile),
            None => Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("profile not found for '{}'", repo_name),
            )),
        }
    }

    pub fn reset_profile_for_project(project_path: String) -> Result<(), io::Error> {
        let repo_name = get_repo_name(Path::new(&project_path))?;

        let mut map = <Self as ProfileManager>::read_project_profiles()?;
        map.remove(&repo_name);
        <Self as ProfileManager>::write_project_profiles(&map)
    }
}

fn get_repo_name(project_path: &Path) -> Result<String, io::Error> {
    match resolve_git_repo_name(project_path)? {
        Some(name) => Ok(name),
        None => {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "no git repository found for given project path",
            ));
        }
    }
}

fn resolve_git_repo_name(start: &Path) -> Result<Option<String>, io::Error> {
    let mut cur = if start.is_file() {
        start
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        start.to_path_buf()
    };

    loop {
        let git_dir = cur.join(".git");
        if git_dir.is_dir() {
            if let Some(name) = cur.file_name().and_then(|n| n.to_str()) {
                return Ok(Some(name.to_string()));
            } else {
                return Ok(None);
            }
        }

        if !cur.pop() {
            break;
        }
    }

    Ok(None)
}
