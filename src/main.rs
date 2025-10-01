use clap::Parser;
use octopush::util::cli;

fn main() -> Result<(), std::io::Error> {
    cli::run(cli::Cli::parse())?;

    Ok(())
}
