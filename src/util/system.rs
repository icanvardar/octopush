use std::{env, io::Error};

pub fn cwd() -> Result<String, Error> {
    Ok(env::current_dir()?.to_string_lossy().into_owned())
}
