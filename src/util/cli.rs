use crate::util::path_completer::dialoguer_path_input;
use crate::{
    core::{app::App, auth::AuthType, profile::Profile},
    util::{
        output::{OperationType, Runner},
        system::cwd,
    },
};
use clap::{Parser, Subcommand};
use dialoguer::{Input, Select};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    AddProfile {
        #[arg(short, long)]
        profile_name: Option<String>,
        // this name field is for name field in the git config
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        email: Option<String>,
        #[arg(short, long)]
        auth_type: Option<AuthType>,
        #[arg(short('g'), long)]
        hostname: Option<String>,
        #[arg(short, long)]
        ssh_key_path: Option<String>,
    },
    DeleteProfile {
        #[arg(short, long)]
        profile_name: String,
    },
    ListProfiles,
    UseProfile {
        #[arg(short, long)]
        profile_name: String,
    },
    GetProfile,
    ResetProfile,
}

// NOTE:
// no whitespaces in the profile name are allowed
// validate email in the email field
// make the ssh_key_path input autocompletable

pub fn run() -> Result<(), std::io::Error> {
    let cli = Cli::parse();
    let runner = Runner::new();

    match cli.command {
        Command::AddProfile {
            profile_name,
            name,
            email,
            auth_type,
            hostname,
            ssh_key_path,
        } => {
            let profile_name = match profile_name {
                Some(profile_name) => profile_name.clone(),
                None => Input::new()
                    .with_prompt("Enter a profile name for the new profile")
                    .interact_text()
                    .unwrap(),
            };

            let name = match name {
                Some(name) => name.clone(),
                None => Input::new()
                    .with_prompt("Enter a name for the new profile")
                    .interact_text()
                    .unwrap(),
            };

            let email = match email {
                Some(email) => email.clone(),
                None => Input::new()
                    .with_prompt("Enter an email for the new profile")
                    .interact_text()
                    .unwrap(),
            };

            let auth_type = match auth_type {
                Some(auth_type) => auth_type,
                None => {
                    let auth_items = ["none", "ssh", "gh"];
                    let auth_type = Select::new()
                        .with_prompt("Select the authentication type...")
                        .items(auth_items)
                        .default(0)
                        .interact()
                        .unwrap();

                    auth_items[auth_type].parse().unwrap()
                }
            };

            let mut hostname: Option<String> = hostname;
            let mut ssh_key_path: Option<String> = ssh_key_path;

            match auth_type {
                AuthType::None => {}
                AuthType::SSH => {
                    let input = dialoguer_path_input("Enter the path of your ssh key: ");
                    ssh_key_path = if input.trim().is_empty() {
                        None
                    } else {
                        Some(input)
                    };
                }
                AuthType::GH => {
                    hostname = Some(
                        Input::new()
                            .with_prompt("Enter the hostname of authenticated account")
                            .interact_text()
                            .unwrap(),
                    );
                }
            }

            let profile = Profile::build(
                profile_name.clone(),
                name,
                email,
                auth_type,
                hostname,
                ssh_key_path,
            );

            let _ = runner.run(
                || {
                    App::add_profile(profile_name.clone(), profile)?;

                    Ok(())
                },
                OperationType::AddProfile {
                    profile_name: profile_name.clone(),
                },
            );

            Ok(())
        }
        Command::DeleteProfile { profile_name } => {
            let _ = runner.run(
                || {
                    App::delete_profile(profile_name.clone())?;

                    Ok(())
                },
                OperationType::DeleteProfile {
                    profile_name: profile_name.clone(),
                },
            );

            Ok(())
        }
        Command::ListProfiles => {
            let _ = runner.run(
                || {
                    let profiles = App::list_profiles()?;

                    println!("{:?}", profiles);

                    Ok(())
                },
                OperationType::ListProfiles,
            );

            Ok(())
        }
        Command::UseProfile { profile_name } => {
            let cwd = cwd()?;

            let _ = runner.run(
                || {
                    let _ = App::use_profile(profile_name.clone(), cwd)?;

                    Ok(())
                },
                OperationType::UseProfile {
                    profile_name: profile_name.clone(),
                },
            );

            Ok(())
        }
        Command::GetProfile => {
            let cwd = cwd()?;

            let _ = runner.run(
                || {
                    let (profile, repo_name) = App::get_project_profile(cwd)?;

                    runner.message(
                        format!(
                            "The repository '{}' is associated with profile {}.",
                            repo_name, profile.id
                        )
                        .as_str(),
                    );

                    Ok(())
                },
                OperationType::GetProfile,
            );

            Ok(())
        }
        Command::ResetProfile => {
            let cwd = cwd()?;

            let _ = runner.run(
                || {
                    App::reset_profile_for_project(cwd)?;

                    Ok(())
                },
                OperationType::ResetProfile,
            );

            Ok(())
        }
    }
}
