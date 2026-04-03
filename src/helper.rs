use std::path::{Path};
use std::fs;
use std::fs::{copy, read_dir, remove_dir_all, remove_file, create_dir_all};
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
    pub fn empty_directory(tgt_path: &Path, preserve: &[&str]) -> Result<(), PistonError>{
        if !tgt_path.exists() || !tgt_path.is_dir() {
            return Ok(())
        }
        
        println!("🧹 Cleaning :{} (preserving: {:?})", tgt_path.display(), preserve);

        for entry in read_dir(tgt_path)
            .map_err(|e| PistonError::ReadDirError { path: tgt_path.to_path_buf(), source: e })?
        {
            let entry = entry.map_err(|e| PistonError::ReadDirError { path: tgt_path.to_path_buf(), source: e })?;
            let entry_path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();

            // Skip any directory we want to preserve
            if entry_path.is_dir() && preserve.contains(&name.as_str()) {
                println!("   Preserving: {}", name);
                continue;
            }

            if entry_path.is_dir() {
                remove_dir_all(&entry_path)
                    .map_err(|e| PistonError::RemoveSubdirError { path: entry_path.clone(), source: e })?;
            } else {
                remove_file(&entry_path)
                    .map_err(|e| PistonError::RemoveFileError { path: entry_path.clone(), source: e })?;
            }
        }
        Ok(())
    }

    pub fn sync_assets(src: &Path, tgt: &Path) -> Result<(), PistonError> {
        println!("attempting to sync assets dir: {} with destination: {}", src.display(), tgt.display());
        if !src.exists() {
            println!("⚠️  Assets source not found at {:?} — removing target if it exists", src);
            if tgt.exists() {
                remove_dir_all(tgt).map_err(|e| PistonError::RemoveSubdirError { path: tgt.to_path_buf(), source: e })?;
            }
            return Ok(());
        }

        create_dir_all(tgt)
            .map_err(|e| PistonError::Generic(format!("Failed to create base/assets: {}", e)))?;

        println!("📦 Syncing assets: {:?} → {:?}", src, tgt);

        Self::copy_updated_files(src, tgt)?;
        Self::remove_stale_files(src, tgt)?;

        println!("✅ Assets synced (only changed files were updated)");
        Ok(())
    }

    //copy new or newer files
    fn copy_updated_files(src: &Path, dst: &Path) -> Result<(), PistonError> {
        for entry in read_dir(src)
            .map_err(|e| PistonError::ReadDirError { path: src.to_path_buf(), source: e })?
        {
            let entry = entry.map_err(|e| PistonError::ReadDirError { path: src.to_path_buf(), source: e })?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                fs::create_dir_all(&dst_path)
                    .map_err(|e| PistonError::Generic(format!("Failed to create asset subdir {}: {}", dst_path.display(), e)))?;

                Self::copy_updated_files(&src_path, &dst_path)?;
            } else {
                let needs_copy = match (src_path.metadata(), dst_path.metadata()) {
                    (Ok(src_meta), Ok(dst_meta)) => {
                        src_meta.modified().map_err(|e| {
                            PistonError::Generic(format!("Failed to get modified time of source {}: {}", src_path.display(), e))
                        })? > dst_meta.modified().map_err(|e| {
                            PistonError::Generic(format!("Failed to get modified time of destination {}: {}", dst_path.display(), e))
                        })?
                    }
                    (Ok(_), Err(_)) => true,           // destination doesn't exist
                    (Err(e), _) => {
                        return Err(PistonError::Generic(format!(
                            "Failed to read metadata of source {}: {}", src_path.display(), e
                        )));
                    }
                };

                if needs_copy {
                    copy(&src_path, &dst_path)
                        .map_err(|e| PistonError::Generic(format!(
                            "Failed to copy asset {} → {}: {}", 
                            src_path.display(), dst_path.display(), e
                        )))?;
                }
            }
        }
        Ok(())
    }

    //delete files in target that no longer exist in source
    fn remove_stale_files(src: &Path, dst: &Path) -> Result<(), PistonError> {
        for entry in read_dir(dst)
            .map_err(|e| PistonError::ReadDirError { path: dst.to_path_buf(), source: e })?
        {
            let entry = entry.map_err(|e| PistonError::ReadDirError { path: dst.to_path_buf(), source: e })?;
            let dst_path = entry.path();
            let corresponding_src = src.join(entry.file_name());

            if dst_path.is_dir() {
                if !corresponding_src.exists() {
                    remove_dir_all(&dst_path)
                        .map_err(|e| PistonError::RemoveSubdirError { path: dst_path.clone(), source: e })?;
                } else {
                    Self::remove_stale_files(&corresponding_src, &dst_path)?;
                }
            } else if !corresponding_src.exists() {
                remove_file(&dst_path)
                    .map_err(|e| PistonError::RemoveFileError { path: dst_path.clone(), source: e })?;
            }
        }
        Ok(())
    }

    // pub fn copy_dir_all(input: &Path, output: &Path) -> Result<(), PistonError> {
    //     if !output.exists() {
    //         create_dir_all(&output)
    //             .map_err(|e| PistonError::CreateDirAllError {
    //                 path: output.to_path_buf(),
    //                 source: e,
    //             })?;
    //     }
    //     let entries = read_dir(input).map_err(|e| PistonError::ReadDirError {
    //         path: input.to_path_buf(),
    //         source: e,
    //     })?;
    //     for entry in entries {
    //         let entry = entry.map_err(|e| PistonError::ReadDirError {
    //             path: input.to_path_buf(),
    //             source: e,
    //         })?;
    //         let input_path = entry.path();
    //         let output_path = output.join(entry.file_name());

    //         if entry.file_type()
    //             .map_err(|e| PistonError::Generic(format!("Failed to get file type of: {}, error: {}", input_path.display(), e.to_string())))?
    //             .is_dir() {
    //             Self::copy_dir_all(&input_path, &output_path)?;
    //         }else {
    //             copy(&input_path, &output_path)
    //                 .map_err(|e| PistonError::CopyFileError {
    //                     input_path: input_path,
    //                     output_path: output_path,
    //                     source: e,
    //                 })?;
    //         }
    //     }
    //     Ok(())
    // }

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

    pub fn get_assets_path(metadata: &Metadata) -> String {
        metadata.root_package()
            .and_then(|pkg| pkg.metadata.get("assets_path"))
            .and_then(Value::as_str)
            .map(|s| s.to_string())
            .unwrap_or("".to_string())
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

    pub fn get_bundle_id(metadata: &Metadata, app_name: &str) -> String {
        let default = format!("com.piston.{}", app_name);

        metadata
            .root_package()
            .and_then(|pkg| pkg.metadata.get("ios"))
            .and_then(|ios| ios.get("bundle_id"))
            .and_then(|id| id.as_str())
            .map(|s| s.to_string())
            .unwrap_or(default)
    }

    pub fn get_min_os(metadata: &Metadata) -> f32 {
        let default: f32 = 17.5;

        metadata
            .root_package()
            .and_then(|pkg| pkg.metadata.get("ios"))
            .and_then(|ios| ios.get("min_os_version"))
            .and_then(|min| min.as_f64())
            .map(|val| val as f32)
            .unwrap_or(default)
    }

}
