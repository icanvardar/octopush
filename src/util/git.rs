use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

pub fn run_git<I, S>(repo: &Path, args: I) -> Result<Output, std::io::Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("git").arg("-C").arg(repo).args(args).output()
}

pub fn ensure_repo(repo: &Path) -> Result<(), std::io::Error> {
    let git_dir = repo.join(".git");
    if git_dir.is_dir() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "not a git repository",
        ))
    }
}

pub fn set_local_identity(repo: &Path, name: &str, email: &str) -> Result<(), std::io::Error> {
    let o1 = run_git(repo, ["config", "--local", "user.name", name])?;
    if !o1.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "failed to set user.name",
        ));
    }
    let o2 = run_git(repo, ["config", "--local", "user.email", email])?;
    if !o2.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "failed to set user.email",
        ));
    }
    Ok(())
}

pub fn unset_local(repo: &Path, key: &str) -> Result<(), std::io::Error> {
    let _ = run_git(repo, ["config", "--local", "--unset", key]);
    Ok(())
}

pub fn get_remote_url(repo: &Path, remote: &str) -> Result<Option<String>, std::io::Error> {
    let o = run_git(repo, ["remote", "get-url", remote])?;
    if o.status.success() {
        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if s.is_empty() { Ok(None) } else { Ok(Some(s)) }
    } else {
        Ok(None)
    }
}

pub fn set_remote_url(repo: &Path, remote: &str, url: &str) -> Result<(), std::io::Error> {
    let o = run_git(repo, ["remote", "set-url", remote, url])?;
    if !o.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "failed to set remote url",
        ));
    }
    Ok(())
}

pub fn ensure_ssh_command(repo: &Path, key_path: &str) -> Result<(), std::io::Error> {
    let val = format!("ssh -i {} -F /dev/null", key_path);
    let o = run_git(repo, ["config", "--local", "core.sshCommand", &val])?;
    if !o.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "failed to set core.sshCommand",
        ));
    }
    Ok(())
}

pub fn clear_ssh_command(repo: &Path) -> Result<(), std::io::Error> {
    unset_local(repo, "core.sshCommand")
}

pub fn set_gh_credential_helper(repo: &Path) -> Result<(), std::io::Error> {
    let o = run_git(
        repo,
        [
            "config",
            "--local",
            "credential.helper",
            "!gh auth git-credential",
        ],
    )?;
    if !o.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "failed to set gh credential helper",
        ));
    }
    let _ = run_git(
        repo,
        ["config", "--local", "credential.useHttpPath", "true"],
    )?;
    Ok(())
}

pub fn clear_gh_credential_helper(repo: &Path) -> Result<(), std::io::Error> {
    let _ = unset_local(repo, "credential.helper");
    let _ = unset_local(repo, "credential.useHttpPath");
    Ok(())
}

pub fn parse_remote(url: &str) -> Option<(String, String, String)> {
    if let Some(rest) = url.strip_prefix("git@") {
        let mut parts = rest.splitn(2, ":");
        let host = parts.next()?.to_string();
        let path = parts.next()?;
        return split_path(host, path);
    }

    if let Some(rest) = url.strip_prefix("ssh://") {
        let after_user = rest.split('@').last().unwrap_or(rest);
        let mut parts = after_user.splitn(2, '/');
        let host = parts.next()?.to_string();
        let path = parts.next()?;
        return split_path(host, path);
    }

    if let Some(rest) = url.strip_prefix("https://") {
        let mut parts = rest.splitn(2, '/');
        let host = parts.next()?.to_string();
        let path = parts.next()?;
        return split_path(host, path);
    }
    None
}

fn split_path(host: String, path: &str) -> Option<(String, String, String)> {
    let mut it = path.trim_matches('/').splitn(2, '/');
    let owner = it.next()?.to_string();
    let repo = it.next()?.trim_end_matches(".git").to_string();
    Some((host, owner, repo))
}

pub fn to_ssh(host: &str, owner: &str, repo: &str) -> String {
    format!("git@{}:{}/{}.git", host, owner, repo)
}

pub fn to_https(host: &str, owner: &str, repo: &str) -> String {
    format!("https://{}/{}/{}.git", host, owner, repo)
}

pub fn gh_hosts_file() -> Option<PathBuf> {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs_home_config(".config"));
    base.map(|b| b.join("gh").join("hosts.yml"))
}

fn dirs_home_config(sub: &str) -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(|p| PathBuf::from(p).join(sub))
}

pub fn is_gh_authenticated(host: &str) -> bool {
    if let Some(path) = gh_hosts_file() {
        if let Ok(text) = fs::read_to_string(path) {
            let has_host = text
                .lines()
                .any(|l| l.trim_start().starts_with(&format!("{}:", host)));
            let has_token = text.contains("oauth_token:");
            if has_host && has_token {
                return true;
            }
        }
    }

    if std::env::var_os("GH_TOKEN").is_some() || std::env::var_os("GITHUB_TOKEN").is_some() {
        return true;
    }

    let out = std::process::Command::new("gh")
        .args(["auth", "status", "--hostname", host])
        .output();

    match out {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TempConfig;

    #[test]
    fn ensure_repo_ok_and_err() {
        let t = TempConfig::new().unwrap();

        assert!(ensure_repo(&t.repo).is_ok());

        let not_repo = t.base.join("not_repo");
        fs::create_dir_all(&not_repo).unwrap();
        assert!(ensure_repo(&not_repo).is_err());
    }

    #[test]
    fn set_and_get_remote_url() {
        let t = TempConfig::new().unwrap();

        let add = run_git(
            &t.repo,
            ["remote", "add", "origin", "https://example.com/foo/bar.git"],
        )
        .unwrap();
        assert!(add.status.success());

        assert!(set_remote_url(&t.repo, "origin", "https://example.com/acme/app.git").is_ok());
        let url = get_remote_url(&t.repo, "origin").unwrap();
        assert_eq!(url.as_deref(), Some("https://example.com/acme/app.git"));
    }

    #[test]
    fn set_and_clear_local_identity() {
        let t = TempConfig::new().unwrap();

        set_local_identity(&t.repo, "Test User", "test@example.com").unwrap();

        let g1 = run_git(&t.repo, ["config", "--local", "user.name"]).unwrap();
        assert_eq!(String::from_utf8_lossy(&g1.stdout).trim(), "Test User");

        let g2 = run_git(&t.repo, ["config", "--local", "user.email"]).unwrap();
        assert_eq!(
            String::from_utf8_lossy(&g2.stdout).trim(),
            "test@example.com"
        );

        unset_local(&t.repo, "user.signingkey").unwrap();
    }

    #[cfg(not(windows))]
    #[test]
    fn set_and_clear_ssh_command() {
        let t = TempConfig::new().unwrap();

        ensure_ssh_command(&t.repo, "/tmp/fake_key").unwrap();
        let g = run_git(&t.repo, ["config", "--local", "core.sshCommand"]).unwrap();
        assert!(String::from_utf8_lossy(&g.stdout).contains("/tmp/fake_key"));

        clear_ssh_command(&t.repo).unwrap();
        let g2 = run_git(&t.repo, ["config", "--local", "--get", "core.sshCommand"]).unwrap();
        assert!(!g2.status.success());
    }

    #[test]
    fn gh_auth_env_token_is_authenticated() {
        let _cfg = TempConfig::new().unwrap();

        unsafe {
            env::set_var("GH_TOKEN", "dummy-token");
        }
        assert!(is_gh_authenticated("github.com"));
        unsafe {
            env::remove_var("GH_TOKEN");
        }
    }

    #[test]
    fn gh_auth_hosts_yml_with_token_is_authenticated() {
        let _cfg = TempConfig::new().unwrap();

        assert!(is_gh_authenticated("github.com"));
    }

    #[test]
    fn parse_and_format_remote_variants() {
        let (h, o, r) = parse_remote("git@github.com:acme/app.git").unwrap();
        assert_eq!(
            (h, o, r),
            ("github.com".into(), "acme".into(), "app".into())
        );

        let (h, o, r) = parse_remote("ssh://git@github.com/acme/app.git").unwrap();
        assert_eq!(
            (h, o, r),
            ("github.com".into(), "acme".into(), "app".into())
        );

        let (h, o, r) = parse_remote("https://github.com/acme/app.git").unwrap();
        assert_eq!(
            (h, o, r),
            ("github.com".into(), "acme".into(), "app".into())
        );

        assert_eq!(
            to_ssh("github.com", "acme", "app"),
            "git@github.com:acme/app.git"
        );
        assert_eq!(
            to_https("github.com", "acme", "app"),
            "https://github.com/acme/app.git"
        );
    }
}
