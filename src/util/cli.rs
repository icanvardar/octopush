use crate::{
    core::{app::App, auth::AuthType, profile::Profile},
    util::{output::Runner, system::cwd},
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
        #[arg(short, long)]
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

pub fn run() -> Result<(), std::io::Error> {
    let cli = Cli::parse();

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
                    ssh_key_path = Some(
                        Input::new()
                            .with_prompt("Enter the path of your ssh key")
                            .interact_text()
                            .unwrap(),
                    );
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

            let profile = Profile::build(name, email, auth_type, hostname, ssh_key_path);

            App::add_profile(profile_name, profile)?;

            // TODO: std out meaningful message

            Ok(())
        }
        Command::DeleteProfile { profile_name } => {
            App::delete_profile(profile_name)?;

            // TODO: std out meaningful message

            Ok(())
        }
        Command::ListProfiles => {
            let profiles = App::list_profiles()?;

            // TODO: find a way to prettify the profiles output

            Ok(())
        }
        Command::UseProfile { profile_name } => {
            let cwd = cwd()?;

            App::use_profile(profile_name, cwd)?;

            // TODO: std out meaningful message

            Ok(())
        }
        Command::GetProfile => {
            let cwd = cwd()?;

            let profile = App::get_project_profile(cwd)?;

            // TODO: std out meaningful message

            Ok(())
        }
        Command::ResetProfile => {
            let cwd = cwd()?;

            App::reset_profile_for_project(cwd)?;

            // TODO: std out meaningful message

            Ok(())
        }
    }
}
