use nix::libc::{close, dup, dup2};
use nix::unistd::pipe;
use std::env;
use std::{fs, io, path::PathBuf};
use std::{
    fs::File,
    io::{Read, Write},
    os::fd::AsRawFd,
    sync::Mutex,
};

use clap::Parser;
use octopush::{
    core::profile::Profile,
    test_helpers::{TempConfig, get_profiles},
    util::cli,
};

fn build_add_profile_args(profile_name: String, profile: Profile) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "octopush".into(),
        "add-profile".into(),
        "--profile-name".into(),
        profile_name,
        "--name".into(),
        profile.name.clone(),
        "--email".into(),
        profile.email.clone(),
        "--auth-type".into(),
        Into::<&str>::into(profile.auth_type).to_string(),
    ];

    if let Some(ref p) = profile.ssh_key_path {
        args.push("--ssh-key-path".into());
        args.push(p.clone());
    }
    if let Some(ref h) = profile.hostname {
        args.push("--hostname".into());
        args.push(h.clone());
    }

    args
}

fn build_delete_profile_args(profile_name: String) -> Vec<String> {
    let args: Vec<String> = vec![
        "octopush".into(),
        "delete-profile".into(),
        "--profile-name".into(),
        profile_name,
    ];

    args
}

fn build_list_profiles_args() -> Vec<String> {
    let args: Vec<String> = vec!["octopush".into(), "list-profiles".into()];

    args
}

fn read_raw_profiles(base: PathBuf) -> Result<String, std::io::Error> {
    let profiles = base.join("octopush").join("profiles.toml");
    let raw_profiles = fs::read_to_string(profiles)?;

    Ok(raw_profiles)
}

#[test]
fn tests_add_profile_cmd() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = TempConfig::new().unwrap();

    let (profile_names, profiles) = get_profiles();

    let args = build_add_profile_args(profile_names[0].into(), profiles[0].clone());

    let parsed_cli = cli::Cli::try_parse_from(args.clone())?;

    cli::run(parsed_cli)?;

    let raw_profiles = read_raw_profiles(cfg.base.clone())?;

    let expected = r#"
[profile_1]
name = "profile_1_name"
email = "profile_1_email"
auth_type = "None"
    "#;

    assert_eq!(normalize(&raw_profiles), normalize(expected));

    Ok(())
}

#[test]
fn tests_delete_profile_cmd() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = TempConfig::new().unwrap();

    let (profile_names, profiles) = get_profiles();

    let args = build_add_profile_args(profile_names[0].into(), profiles[0].clone());

    let parsed_cli = cli::Cli::try_parse_from(args.clone())?;

    cli::run(parsed_cli)?;

    // after profile addition
    let raw_profiles = read_raw_profiles(cfg.base.clone())?;

    let expected = r#"
[profile_1]
name = "profile_1_name"
email = "profile_1_email"
auth_type = "None"
    "#;

    assert_eq!(normalize(&raw_profiles), normalize(expected));

    let args = build_delete_profile_args(profile_names[0].into());

    let parsed_cli = cli::Cli::try_parse_from(args.clone())?;

    cli::run(parsed_cli)?;

    // after profile deletion

    let raw_profiles = read_raw_profiles(cfg.base.clone())?;

    assert_eq!(normalize(&raw_profiles), "".to_owned());

    Ok(())
}

#[test]
fn tests_list_profile_cmd() -> Result<(), Box<dyn std::error::Error>> {
    let _cfg = TempConfig::new().unwrap();

    let (profile_names, profiles) = get_profiles();

    let args_vec = [
        build_add_profile_args(profile_names[0].into(), profiles[0].clone()),
        build_add_profile_args(profile_names[1].into(), profiles[1].clone()),
        build_add_profile_args(profile_names[2].into(), profiles[2].clone()),
    ];

    // adds profiles
    let _ = args_vec
        .iter()
        .map(|args| -> Result<(), Box<dyn std::error::Error>> {
            let parsed_cli = cli::Cli::try_parse_from(args)?;

            cli::run(parsed_cli)?;

            Ok(())
        })
        .collect::<Vec<_>>();

    let out = capture_stdout(|| {
        let args = build_list_profiles_args();

        let parsed_cli = cli::Cli::try_parse_from(args.clone()).unwrap();
        cli::run(parsed_cli).unwrap();
    });

    // NOTE: test this output right after making the output pretty
    let out = out.replace("\n✅ SUCCESS Profiles successfully fetched\n", "");

    // println!("------ profiles output: {} ------", out);

    assert!(!out.is_empty());

    Ok(())
}

#[test]
fn tests_use_profile_cmd() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = TempConfig::new()?;

    let ([profile_name, _, _], [profile, _, _]) = get_profiles();

    // adds new profile
    let args = build_add_profile_args(profile_name.to_string(), profile.clone());
    let parsed_cli = cli::Cli::try_parse_from(args)?;
    cli::run(parsed_cli)?;

    let prev_cwd = env::current_dir()?;
    env::set_current_dir(&cfg.repo)?;

    let args = vec![
        "octopush".into(),
        "use-profile".into(),
        "--profile-name".into(),
        profile_name.to_string(),
    ];
    let parsed_cli = cli::Cli::try_parse_from(args)?;
    cli::run(parsed_cli)?;

    use octopush::core::app::App;
    let (_, applied, _) = App::get_project_profile(cfg.repo.to_string_lossy().into_owned())?;
    assert_eq!(applied, profile);

    env::set_current_dir(prev_cwd)?;

    Ok(())
}

#[test]
fn tests_get_profile_cmd() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = TempConfig::new()?;

    let ([profile_name, _, _], [profile, _, _]) = get_profiles();

    use octopush::core::app::App;
    App::add_profile(profile_name.to_string(), profile)?;

    let prev_cwd = env::current_dir()?;
    env::set_current_dir(&cfg.repo)?;

    App::use_profile(
        profile_name.to_string(),
        cfg.repo.to_string_lossy().into_owned(),
    )?;

    let out = capture_stdout(|| {
        let args: Vec<String> = vec!["octopush".into(), "get-profile".into()];
        let parsed_cli = cli::Cli::try_parse_from(args).unwrap();
        cli::run(parsed_cli).unwrap();
    });

    let expected = format!(
        "The repository '{}' is associated with profile {}.",
        "repo", profile_name
    );
    assert!(out.contains(&expected));

    env::set_current_dir(prev_cwd)?;

    Ok(())
}

#[test]
fn tests_reset_profile_cmd() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = TempConfig::new()?;

    let ([profile_name, _, _], [profile, _, _]) = get_profiles();

    use octopush::core::app::App;
    App::add_profile(profile_name.to_string(), profile)?;

    let prev_cwd = env::current_dir()?;
    env::set_current_dir(&cfg.repo)?;

    App::use_profile(
        profile_name.to_string(),
        cfg.repo.to_string_lossy().into_owned(),
    )?;

    let (applied_name, _, repo_name) =
        App::get_project_profile(cfg.repo.to_string_lossy().into_owned())?;

    assert_eq!(applied_name, profile_name);

    let args: Vec<String> = vec!["octopush".into(), "reset-profile".into()];
    let parsed_cli = cli::Cli::try_parse_from(args)?;

    cli::run(parsed_cli)?;

    let result = App::get_project_profile(cfg.repo.to_string_lossy().into_owned());
    let err = result.unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    assert_eq!(
        err.to_string(),
        format!("profile not found for '{}'", repo_name)
    );

    env::set_current_dir(prev_cwd)?;

    Ok(())
}

fn normalize(s: &str) -> String {
    s.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
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
