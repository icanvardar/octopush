use nix::libc::{close, dup, dup2};
use nix::unistd::pipe;
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

fn build_add_profile_args(profile: Profile) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "octopush".into(),
        "add-profile".into(),
        "--profile-name".into(),
        profile.id.clone(),
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
fn test_profile_creation() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = TempConfig::new().unwrap();

    let (profile_1, _, _) = get_profiles();

    let args = build_add_profile_args(profile_1.clone());

    let parsed_cli = cli::Cli::try_parse_from(args.clone())?;

    cli::run(parsed_cli)?;

    let raw_profiles = read_raw_profiles(cfg.base.clone())?;

    let expected = r#"
[profile_1]
id = "profile_1"
name = "profile_1_name"
email = "profile_1_email"
auth_type = "None"
    "#;

    assert_eq!(normalize(&raw_profiles), normalize(expected));

    Ok(())
}

#[test]
fn test_profile_deletion() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = TempConfig::new().unwrap();

    let (profile_1, _, _) = get_profiles();

    let args = build_add_profile_args(profile_1.clone());

    let parsed_cli = cli::Cli::try_parse_from(args.clone())?;

    cli::run(parsed_cli)?;

    // after profile addition
    let raw_profiles = read_raw_profiles(cfg.base.clone())?;

    let expected = r#"
[profile_1]
id = "profile_1"
name = "profile_1_name"
email = "profile_1_email"
auth_type = "None"
    "#;

    assert_eq!(normalize(&raw_profiles), normalize(expected));

    let args = build_delete_profile_args(profile_1.id);

    let parsed_cli = cli::Cli::try_parse_from(args.clone())?;

    cli::run(parsed_cli)?;

    // after profile deletion

    let raw_profiles = read_raw_profiles(cfg.base.clone())?;

    assert_eq!(normalize(&raw_profiles), "".to_owned());

    Ok(())
}

#[test]
fn test_profile_listing() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = TempConfig::new().unwrap();

    let (profile_1, profile_2, profile_3) = get_profiles();

    let args_vec = vec![
        build_add_profile_args(profile_1.clone()),
        build_add_profile_args(profile_2.clone()),
        build_add_profile_args(profile_3.clone()),
    ];

    // adds profiles
    let _ = args_vec.iter().map(|args| {
        let parsed_cli = cli::Cli::try_parse_from(args).unwrap();

        cli::run(parsed_cli).unwrap();
    });

    //let out = capture_stdout(|| {
    let args = build_list_profiles_args();

    let parsed_cli = cli::Cli::try_parse_from(args.clone()).unwrap();
    cli::run(parsed_cli).unwrap();
    // });

    //println!("profiles output: {}", out);

    Ok(())
}

#[test]
fn test_profile_selection() {}

#[test]
fn test_profile_fetching() {}

#[test]
fn test_profile_resetting() {}

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
