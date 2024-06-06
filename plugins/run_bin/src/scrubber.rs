use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ExeEntry {
    pub exec: String,
    pub path: Option<PathBuf>,
    // pub name: String,
}

pub fn scrubber() -> Result<Vec<(ExeEntry, u64)>, Box<dyn std::error::Error>> {
    // Create iterator over all the files in the XDG_DATA_DIRS
    let mut final_path = Vec::new();
    let mut file_set = HashSet::new();
    let key = "PATH";
    for path in env::split_paths(&env::var_os(key).unwrap()) {
        let paths = fs::read_dir(path).unwrap();
        for exe in paths {
            let path = exe.unwrap().path();
            file_set.insert(path);
        }
    }
    for (id, path) in file_set.into_iter().enumerate() {
        final_path.push((
            ExeEntry {
                exec: path.file_name().unwrap().to_str().unwrap().to_string(),
                path: Some(path),
            },
            id as u64,
        ))
    }
    Ok(final_path)
}
