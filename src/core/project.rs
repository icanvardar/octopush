use std::{
    io,
    path::{Path, PathBuf},
    str::FromStr,
};

pub struct Project {
    pub path: PathBuf,
}

impl FromStr for Project {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Project::new(s)
    }
}

impl Project {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        let p = path.as_ref();

        if !p.exists() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid path"));
        }

        Ok(Project {
            path: p.to_path_buf(),
        })
    }

    pub fn get_repo_name(&self) -> Result<String, io::Error> {
        match Self::resolve_git_repo_name(&self.path)? {
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
}
