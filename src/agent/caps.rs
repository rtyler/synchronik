use std::collections::HashMap;
use std::path::PathBuf;

use serde::Serialize;

/*
 * The Capability trait defines the interface for determining
 * whether the given agent's execution environment has a capability
 * or not
 */
pub trait Capability {
    fn has_capability() -> Option<Self>
    where
        Self: Sized + Serialize,
    {
        match locate_on_path(&Self::binary_name()) {
            Some(pathbuf) => Self::from(pathbuf),
            None => None,
        }
    }

    fn binary_name() -> String
    where
        Self: Sized;
    fn from(pathbuf: PathBuf) -> Option<Self>
    where
        Self: Sized;
}

/*
 * Locate a binary given the name on the search path
 */
fn locate_on_path(bin: &str) -> Option<PathBuf> {
    if let Ok(path) = std::env::var("PATH") {
        for path_dir in std::env::split_paths(&path) {
            let full_path = path_dir.join(bin);
            if full_path.is_file() {
                // TODO: Should check to see if the path is executable
                return Some(PathBuf::from(full_path));
            }
        }
    }
    None
}

/*
 * Git capability will determine whether `git` exists on the system
 */
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "name")]
pub struct Git {
    path: PathBuf,
    data: Option<HashMap<String, String>>,
}

impl Capability for Git {
    fn binary_name() -> String {
        "git".into()
    }
    fn from(pb: PathBuf) -> Option<Self> {
        Some(Self {
            path: pb,
            data: None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "name")]
pub struct Cargo {
    path: PathBuf,
    data: Option<HashMap<String, String>>,
}

impl Capability for Cargo {
    fn binary_name() -> String {
        "cargo".into()
    }
    fn from(pb: PathBuf) -> Option<Self> {
        Some(Self {
            path: pb,
            data: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_git_capability() {
        let cap = Git::has_capability();
        assert!(cap.is_some(), "Somehow this machine doesn't have Git?");
    }
}
