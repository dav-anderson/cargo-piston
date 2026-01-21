use std::path::{Path};
use std::fs;
use std::fs::{read_dir, remove_dir_all, remove_file};
use std::process::Command;
use image::imageops;
use std::path::PathBuf;
use cargo_metadata::{Metadata, TargetKind};
use serde_json::Value;
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

    pub fn get_or_err<'a>(map: &'a HashMap<String, String>, key: &str) -> Result<&'a String, PistonError> {
        map.get(key).ok_or(PistonError::AndroidConfigError(format!("key '{}' not found in .env", key)))
    }

    pub fn get_host_platform(ndk_path: &str) -> Result<String, PistonError> {
        let prebuilt_path = PathBuf::from(ndk_path).join("toolchains/llvm/prebuilt");
        
        let entries = fs::read_dir(&prebuilt_path)
            .map_err(|e| PistonError::BuildError(format!("Failed to read prebuilt dir: {}", e)))?;
        
        let mut host_dirs = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| PistonError::BuildError(format!("Dir entry error: {}", e)))?;
            if entry.file_type().map_err(|e| PistonError::BuildError(format!("File type error: {}", e)))?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    host_dirs.push(name.to_string());
                }
            }
        }
        
        if host_dirs.is_empty() {
            return Err(PistonError::BuildError("No host platform dir found in NDK prebuilt".to_string()));
        } else if host_dirs.len() > 1 {
            // Warn or error; for now, take the first
            eprintln!("Warning: Multiple host dirs found; using the first: {}", host_dirs[0]);
        }
        
        Ok(host_dirs[0].clone())
    }

    pub fn get_build_tools_version(sdk_path: &str) -> Result<String, PistonError> {
        let prebuilt_path = PathBuf::from(sdk_path).join("build-tools");
        
        let entries = fs::read_dir(&prebuilt_path)
            .map_err(|e| PistonError::BuildError(format!("Failed to read prebuilt dir: {}", e)))?;
        
        let mut host_dirs = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| PistonError::BuildError(format!("Dir entry error: {}", e)))?;
            if entry.file_type().map_err(|e| PistonError::BuildError(format!("File type error: {}", e)))?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    host_dirs.push(name.to_string());
                }
            }
        }
        
        if host_dirs.is_empty() {
            return Err(PistonError::BuildError("No build tools version found in SDK prebuilt".to_string()));
        } else if host_dirs.len() > 1 {
            // Warn or error; for now, take the first
            eprintln!("Warning: Multiple host dirs found; using the first: {}", host_dirs[0]);
        }
        
        Ok(host_dirs[0].clone())
    }

    pub fn get_lib_name(metadata: &Metadata) -> Result<String, PistonError> {
        let root_package = metadata.root_package()
            .ok_or(PistonError::CargoParseError("no root package found in metadata".to_string()))?;

        // Default to package.name with hyphens replaced by underscores (Cargo's convention for lib outputs)
        let mut lib_name = root_package.name.replace("-", "_");

        // If [lib] name is overridden, find it in targets (for cdylib)
        let target_type: TargetKind = "cdylib".into();
        for target in &root_package.targets {
            if target.kind.iter().any(|k| k == &target_type) {
                lib_name = target.name.clone();
                break;
            }
        }
        Ok(lib_name)
    }

    pub fn get_icon_path(metadata: &Metadata) -> Option<String> {
        metadata.root_package()
            .and_then(|pkg| pkg.metadata.get("icon_path"))
            .and_then(Value::as_str)
            .map(|s| s.to_string())
    }

    pub fn get_app_name(metadata: &Metadata) -> Result<String, PistonError> {
        metadata.root_package()
            .map(|pkg| pkg.name.to_string())
            .ok_or(PistonError::CargoParseError("app_name not found in [package]".to_string()))
    }

    pub fn get_app_version(metadata: &Metadata) -> Result<String, PistonError> {
        metadata.root_package()
            .map(|pkg| pkg.version.to_string())
            .ok_or(PistonError::CargoParseError("app_version not found in [package]".to_string()))
    }
}
