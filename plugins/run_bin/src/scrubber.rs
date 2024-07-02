use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ExeEntry {
    pub exec: String,
    pub path: Option<PathBuf>,
    // pub name: String,
}

fn resolve_symlink(path: PathBuf) -> Result<PathBuf, std::io::Error> {
    if path.exists() {
        if path.is_symlink() {
            fs::read_link(path)
        } else {
            Ok(path.to_path_buf())
        }
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Path does not exist",
        ))
    }
}

fn remove_duplicates<I>(paths: I) -> HashSet<PathBuf>
where
    I: IntoIterator<Item = PathBuf>,
{
    let path_iter = paths.into_iter();
    let mut unique_paths = HashSet::new();

    // Collect unique paths into the HashSet
    for path in path_iter {
        unique_paths.insert(path);
    }
    unique_paths
}

pub fn scrubber() -> Result<Vec<(ExeEntry, u64)>, Box<dyn Error>> {
    // Create iterator over all the files in the XDG_DATA_DIRS
    let mut file_set = HashSet::new();
    let key = "PATH";
    let split_paths = remove_duplicates(env::split_paths(&env::var_os(key).unwrap()));
    for path in split_paths {
        if !path.is_dir() {
            continue;
        }
        let path_res = fs::read_dir(&path);
        match path_res {
            Ok(paths) => {
                for exe in paths {
                    match exe {
                        Ok(executable) => {
                            let exec_path = executable.path();
                            match resolve_symlink(exec_path) {
                                Ok(org_file_path) => {
                                    file_set.insert(org_file_path);
                                }
                                Err(err) => {
                                    eprintln!("Error occured finding the symlink org path {}", err)
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("Error occured getting DirEntry {}", err);
                        }
                    }
                }
            }
            Err(err) => {
                eprintln!("Error occured reading the directory {:?}\n{}", &path, err);
            }
        }
    }
    let mut final_entries = Vec::with_capacity(file_set.len());
    for (id, path) in file_set.into_iter().enumerate() {
        final_entries.push((
            ExeEntry {
                exec: path.file_name().unwrap().to_str().unwrap().to_string(),
                path: Some(path),
            },
            id as u64,
        ))
    }
    Ok(final_entries)
}
