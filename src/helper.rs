use std::path::{PathBuf, Path};
use std::fs::{read_dir, remove_dir_all, remove_file};
use anyhow::{Context, bail, Result};
use crate::error::PistonError;

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
        println!("Directory emptied: {:?}", dir_path);
        Ok(())
    }
}
