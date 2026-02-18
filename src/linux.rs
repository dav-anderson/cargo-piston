use cargo_metadata::{Metadata, MetadataCommand};
use std::path::PathBuf;
use std::env;
use std::fs::{create_dir_all, copy};
use std::process::{Command, Stdio};
 use std::collections::HashMap;
use crate::error::PistonError;
use crate::helper::Helper;

pub struct LinuxBuilder {
    release: bool,
    target: String,
    cwd: PathBuf,
    output_path: Option<PathBuf>,
    icon_path: Option<String>,
    cargo_path: String,
    zigbuild_path: Option<String>,
    homebrew_path: Option<String>,
    app_name: String,
    key_path: Option<String>
}

impl LinuxBuilder {
    pub fn start(release: bool, target: String, cwd: PathBuf, env_vars: HashMap<String, String>) -> Result<(), PistonError> {
    println!("building for linux");
    let mut op = LinuxBuilder::new(release, target, cwd, env_vars)?;
    //TODO embed icon image?
    //>>prebuild
    op.pre_build()?;

    //>>build
    op.build()?;

    //>>Postbuild
    op.post_build()?;

    Ok(())
    }

    fn new(release: bool, target: String, cwd: PathBuf, env_vars: HashMap<String, String>) -> Result<Self, PistonError> {
        println!("creating LinuxBuilder: release: {:?}, target: {:?}, cwd: {:?}", release, target.to_string(), cwd);
        //parse env vars
        let cargo_path = env_vars.get("cargo_path").cloned().unwrap_or("cargo".to_string());
        let key_string = if release {"linux_release_keypath"} else {"linux_debug_keypath"};
        let key_path = env_vars.get(key_string).cloned();
        println!("Cargo path determined: {}", &cargo_path);
        //parse cargo.toml
        let metadata: Metadata = MetadataCommand::new()
            .current_dir(cwd.clone())
            .exec()
            .map_err(|e| PistonError::CargoParseError(e.to_string()))?;

        let icon_path = Helper::get_icon_path(&metadata);
        let app_name = Helper::get_app_name(&metadata)?;
        //parse the path to zigbuild if building on Macos
        let mut zigbuild_path: Option<String> = None;
        let mut homebrew_path: Option<String> = None;
        //MACOS ONLY
        if std::env::consts::OS == "macos"{
            //parse zigbuild & homebrew path from .env
            zigbuild_path = Some(env_vars.get("zigbuild_path").cloned().ok_or(PistonError::ZigbuildMissingError("Zigbuild key not found".to_string()))?);
            println!("Zigbuild path determined: {}", &zigbuild_path.clone().unwrap());
            homebrew_path = Some(env_vars.get("homebrew_path").cloned().ok_or(PistonError::HomebrewMissingError("Homebrew key not found".to_string()))?);
            println!("Homebrew path determined: {}", &homebrew_path.clone().unwrap());
        }   
        
        Ok(LinuxBuilder{release: release, target: target.to_string(), cwd: cwd, output_path: None, icon_path: icon_path, cargo_path: cargo_path, zigbuild_path: zigbuild_path, homebrew_path: homebrew_path, app_name: app_name, key_path: key_path})
    }

    fn pre_build(&mut self) -> Result<(), PistonError>{
        println!("pre build for linux");
        println!("building the dynamic app bundle");
        let cwd: PathBuf = self.cwd.clone();
        println!("working dir: {:?}", cwd);
        let rel_output: PathBuf = if self.release {
            "target/release/linux".into()
        }else {"target/debug/linux".into()};
        self.output_path = Some(cwd.join(&rel_output));
        println!("linux dir: {:?}", self.output_path);
        //empty the target directory if it exists
        if self.output_path.as_ref().is_none() {
            return Err(PistonError::Generic("output path not provided".to_string()))
        }
        let path = self.output_path.as_ref().unwrap().as_path();
        //empty the dir if it exists
        Helper::empty_directory(path)?;
        //create the target directory
        create_dir_all(path).map_err(|e| PistonError::CreateDirAllError {
        path: self.output_path.as_ref().unwrap().to_path_buf(),
        source: e,
        })?;
        println!("Finished Pre Build for Linux");
        Ok(())
    }

    fn build(&mut self) -> Result<(), PistonError>{
        println!("Building for linux");
        //build the binary for the specified target
        let cargo_args = format!("build --target {} {}", self.target, if self.release {"--release"} else {""});
        let cargo_cmd = format!("{} {}", self.cargo_path, cargo_args);
        let current_path = env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", self.homebrew_path.as_ref().unwrap(), current_path);
        //MACOS HOST ONLY
        if std::env::consts::OS == "macos"{
            println!("Building for Linux on Macos using Zig linker");
            Command::new("bash")
                .arg("-c")
                .arg(format!("{} {}", self.zigbuild_path.as_ref().unwrap(), &cargo_args))
                .current_dir(self.cwd.clone())
                .env("PATH", new_path)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output()
                .map_err(|e| PistonError::BuildError(format!("Cargo build failed: {}", e)))?;

        //LINUX HOST
        }else{
            Command::new("bash")
                .arg("-c")
                .arg(&cargo_cmd)
                .current_dir(self.cwd.clone())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output()
                .map_err(|e| PistonError::BuildError(format!("Cargo build failed: {}", e)))?;
        }
        Ok(())
    }

    fn post_build(&mut self) -> Result<(), PistonError>{
        println!("post build for linux");
        let binary_path = self.cwd.join("target").join(self.target.clone()).join(if self.release {"release"} else {"debug"}).join(self.app_name.clone());
        let bundle_path = self.output_path.as_ref().unwrap().join(self.app_name.clone());
        //bundle path should be cwd + target + <target output> + <--release flag or None for debug> + <appname>.exe
        println!("binary path is: {}", &binary_path.display());
        println!("bundle path is: {}", &bundle_path.display());
        println!("copying binary to app bundle");
        //move the target binary into the app bundle at the proper location
        copy(&binary_path, &bundle_path).map_err(|e| PistonError::CopyFileError {
            input_path: binary_path.clone().to_path_buf(),
            output_path: bundle_path.clone().to_path_buf(),
            source: e,
        })?;
        //TODO check for valid key and sign
        
        //output the proper location in the terminal for the user to see 
        println!("app bundle available at: {}", &bundle_path.display());
        Ok(())
    }
}

struct LinuxRunner{
device: String, 
}

impl LinuxRunner{
    fn new() -> Self {
        println!("Running for Linux");
        LinuxRunner{device: "device".to_string()}
    }
}