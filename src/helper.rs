use std::path::{PathBuf, Path};
use std::fs::{read_dir, remove_dir_all, remove_file};
use crate::error::PistonError;

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

pub struct Helper {

}


impl Helper {
    pub fn empty_directory(dir_path: &Path) -> Result<(), PistonError>{
        println!("Emptying {:?}", dir_path);
        let entries = read_dir(dir_path).map_err(|e| PistonError::ReadDirError {
            path: dir_path.to_path_buf(),
            source: e,
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| PistonError::ReadDirError {
                path: dir_path.to_path_buf(),
                source: e,
            })?;
            let entry_path = entry.path();
            if entry_path.is_dir() {
                remove_dir_all(&entry_path).map_err(|e| PistonError::RemoveSubdirError {
                    path: entry_path.clone(),
                    source: e,
                })?;
            }else {
                remove_file(&entry_path).map_err(|e| PistonError::RemoveFileError {
                    path: entry_path.clone(),
                    source: e,
                })?;
            }
        }
        println!("Directory emptied: {:?}", &dir_path);
        Ok(())
    }

    pub fn load_env_file() -> io::Result<HashMap<String, String>> {
        let path = std::env::current_dir()?.join(".env");
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut env_map = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;  // Skip empty or comment lines
            }
            if let Some(eq_index) = line.find('=') {
                let key = line[..eq_index].trim().to_string();
                let value = line[eq_index + 1..].trim().to_string();
                if !key.is_empty() {
                    env_map.insert(key, value);
                }
            }
        }

        Ok(env_map)
    }
}
