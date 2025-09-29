use rustyline::completion::{Candidate, Completer};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{Context, Editor, Helper};
use std::fs;
use std::path::{Path, PathBuf};

struct PathCandidate {
    display: String,
    replacement: String,
}

impl Candidate for PathCandidate {
    fn display(&self) -> &str {
        &self.display
    }

    fn replacement(&self) -> &str {
        &self.replacement
    }
}

struct PathCompleter {}

impl Helper for PathCompleter {}

impl Highlighter for PathCompleter {}

impl Hinter for PathCompleter {
    type Hint = String;

    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<Self::Hint> {
        None
    }
}

impl Validator for PathCompleter {
    fn validate(&self, _ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        Ok(ValidationResult::Valid(None))
    }
}

impl Completer for PathCompleter {
    type Candidate = PathCandidate;

    fn complete(
        &self,
        line: &str,
        _pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let (expanded_line, _home_for_tilde): (String, Option<PathBuf>) = if line.starts_with('~') {
            let home = std::env::var("HOME")
                .ok()
                .or_else(|| std::env::var("USERPROFILE").ok())
                .map(PathBuf::from);
            if let Some(home) = home {
                let mut s = home.to_string_lossy().into_owned();
                s.push_str(&line[1..]);
                (s, Some(home))
            } else {
                (line.to_string(), None)
            }
        } else {
            (line.to_string(), None)
        };

        let input_path = Path::new(&expanded_line);
        let (dir, prefix): (&Path, String) = if input_path.is_dir() {
            (input_path, String::new())
        } else {
            let parent = input_path.parent().unwrap_or(Path::new("."));
            let file_name = input_path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            (parent, file_name)
        };

        let mut items: Vec<(String, String, bool)> = vec![]; // (display, replacement, is_dir)
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name_os = match path.file_name() {
                    Some(n) => n,
                    None => continue,
                };
                let name = name_os.to_string_lossy().to_string();
                if name.starts_with(prefix.as_str()) {
                    let is_dir = path.is_dir();
                    // Keep completion relative to the currently typed segment
                    let mut disp = name.clone();
                    if is_dir {
                        disp.push('/');
                    }
                    let repl = disp.clone();
                    items.push((disp, repl, is_dir));
                }
            }
        }

        // Sort: directories first, then files; both alphabetically case-insensitive
        items.sort_by(|a, b| {
            use std::cmp::Ordering;
            match (a.2, b.2) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => a.0.to_lowercase().cmp(&b.0.to_lowercase()),
            }
        });

        let candidates = items
            .into_iter()
            .map(|(display, replacement, _)| PathCandidate {
                display,
                replacement,
            })
            .collect();

        // Replace only the segment after the last '/'
        let start = line.rfind('/').map(|i| i + 1).unwrap_or(0);
        Ok((start, candidates))
    }
}

pub fn dialoguer_path_input(prompt: &str) -> String {
    let mut rl: Editor<PathCompleter, DefaultHistory> = Editor::new().unwrap();
    rl.set_helper(Some(PathCompleter {}));

    let line = match rl.readline(prompt) {
        Ok(line) => line,
        Err(ReadlineError::Interrupted) => {
            eprintln!("Aborted by user (Ctrl+C).");
            std::process::exit(130);
        }
        Err(ReadlineError::Eof) => {
            eprintln!("Aborted (Ctrl+D).");
            std::process::exit(0);
        }
        Err(err) => {
            eprintln!("Input error: {err}");
            std::process::exit(1);
        }
    };

    // Post-process: if user entered a relative path (or used completion),
    // convert to absolute and collapse home to '~' for nicer output.
    let input = line.trim();
    if input.is_empty() {
        return line;
    }

    // Expand input to an absolute path (resolving '~' and relative components)
    let home_dir = std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok())
        .map(PathBuf::from);

    let abs_path = if input.starts_with('~') {
        if let Some(home) = &home_dir {
            let mut s = home.to_path_buf();
            let rest = &input[1..];
            if !rest.is_empty() {
                s.push(rest.trim_start_matches('/'));
            }
            s
        } else {
            PathBuf::from(input)
        }
    } else {
        let p = Path::new(input);
        if p.is_absolute() {
            p.to_path_buf()
        } else if let Ok(cwd) = std::env::current_dir() {
            cwd.join(p)
        } else {
            p.to_path_buf()
        }
    };

    // Normalize REMOVE-only of "/./" segments without filesystem access.
    let mut abs_str = abs_path.display().to_string();
    while abs_str.contains("/./") {
        abs_str = abs_str.replace("/./", "/");
    }
    if abs_str.ends_with("/.") {
        abs_str.truncate(abs_str.len().saturating_sub(2));
        if abs_str.is_empty() {
            abs_str.push('/');
        }
    }

    // Collapse to '~' if under home directory (after normalization)
    if let Some(home) = &home_dir {
        let home_str = home.display().to_string();
        if abs_str.starts_with(&home_str) {
            let suffix = &abs_str[home_str.len()..];
            return format!("~{}", suffix);
        }
    }

    abs_str
}
