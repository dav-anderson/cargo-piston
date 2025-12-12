use std::path::{PathBuf, Path};
use std::fs::{read_dir, remove_dir_all, remove_file};
use anyhow::{Context, bail, Result};


pub struct Helper {

}


impl Helper {
    pub fn empty_directory(dir_path: &Path) -> Result<()>{
        println!("Emptying {:?}", dir_path);
        for entry in read_dir(dir_path).context("failed to read dir")? {
            let entry_path = entry?.path();
            if entry_path.is_dir() {
                remove_dir_all(&entry_path).context("failed to remove subdirectories")?;
            }else {
                remove_file(&entry_path).context("failed to remove file")?;
            }
        }
        println!("Directory emptied: {:?}", dir_path);
        Ok(())
    }
}
