use std::path::{Path};
use std::fs::{read_dir, remove_dir_all, remove_file};
use std::process::Command;
use image::imageops;
use std::path::PathBuf;
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

    pub fn capitalize_first(s: &str) -> String {
        match s.get(0..1) {
            None => String::new(),
            Some(first) => first.to_uppercase() + &s[1..],
        }
    }


    pub fn resize_png(input_name: &str, target_name: &str, width: u32, height: u32) -> Result<(), PistonError> {
    // Open the input PNG file
    let img = image::open(input_name).map_err(|e| PistonError::OpenImageError {
            path: PathBuf::from(input_name),
            source: e,
    })?;

    //remove target output if it exists
    if Path::new(&target_name).exists() {
        let output = Command::new("rm")
            .arg(&target_name)
            .output()
            .unwrap();
        if !output.status.success() {
            return Err(
                PistonError::Generic(format!("error removing the target: {}", target_name))
            );
        }
    }

    // Resize the image to the target resolution
    let resized_img = imageops::resize(&img, width, height, imageops::FilterType::Lanczos3);

    // Save the resized image to the target name
    resized_img.save(target_name).map_err(|e| {
        PistonError::SaveImageError(format!("Failed to save {}: {}", target_name, e))
    })?;

    println!(
        "Resized {} to {}x{} and saved as {}",
        input_name, width, height, target_name
    );
    Ok(())
}
}
