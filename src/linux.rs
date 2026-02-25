use cargo_metadata::{Metadata, MetadataCommand};
use std::path::PathBuf;
use std::env;
use std::io::Write;
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
    gpg_path: Option<String>,
    zigbuild_path: Option<String>,
    homebrew_path: Option<String>,
    app_name: String,
    key_id: Option<String>,
    key_pass: Option<String>,
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
        let cargo_path: String = env_vars.get("cargo_path").cloned().unwrap_or("cargo".to_string());
        let gpg_path: Option<String> = env_vars.get("gpg_path").cloned();
        let key_id: Option<String> = env_vars.get("linux_gpg_key_id").cloned();
        let key_pass: Option<String> = env_vars.get("linux_gpg_key_pass").cloned();
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
        
        Ok(LinuxBuilder{release: release, target: target.to_string(), cwd: cwd, output_path: None, icon_path: icon_path, cargo_path: cargo_path, gpg_path: gpg_path, zigbuild_path: zigbuild_path, homebrew_path: homebrew_path, app_name: app_name, key_id: key_id, key_pass: key_pass})
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
        //check for valid key and sign
        if GPGSigner::gpg_valid(self.key_id.clone(), self.gpg_path.clone()){
            println!("key is valid");
            //sign the bundle with gpg
            let sign = GPGSigner::gpg_sign(self.key_id.clone(), self.key_pass.clone(), self.gpg_path.clone(), &bundle_path);
            println!("{}", sign);
        }
        //output the proper location in the terminal for the user to see 
        println!("app bundle available at: {}", &bundle_path.display());
        Ok(())
    }
}

pub struct LinuxRunner{
release: bool,
cwd: PathBuf,
cargo_path: String,
}

impl LinuxRunner {

    pub fn start(release: bool, cwd: PathBuf, env_vars: HashMap<String, String>) -> Result<(), PistonError> {
        println!("Initializing runner for Linux");
        let mut op = LinuxRunner::new(release, cwd, env_vars)?;

        op.run()?;

        Ok(())
    }
    fn new(release: bool, cwd: PathBuf, env_vars: HashMap<String, String>) -> Result<Self, PistonError> {
        println!("Creating Linux Runner: release flag: {:?}, cwd: {:?}", release, cwd);
        //parse env vars
        let cargo_path = env_vars.get("cargo_path").cloned().unwrap_or("cargo".to_string());

        Ok(LinuxRunner{release: release, cwd: cwd, cargo_path: cargo_path})
        
    }

    fn run(&mut self) -> Result<(), PistonError> {
        println!("Running for Linux");
        //Run the binary for Linux
        let cargo_args = format!("run {}", if self.release {"--release"} else {""});
        let cargo_cmd = format!("{} {}", self.cargo_path, cargo_args);
        Command::new("bash")
            .arg("-c")
            .arg(&cargo_cmd)
            .current_dir(self.cwd.clone())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| PistonError::BuildError(format!("Cargo Run failed: {}", e)))?;
        Ok(())
    }
}


struct GPGSigner;

impl GPGSigner{
        fn gpg_valid(key_id: Option<String>, gpg_bin: Option<String>) -> bool {
        if key_id.is_none() {
            return false
        }else if gpg_bin.is_none() {
            return false
        }
        let output = Command::new(gpg_bin.unwrap())
            .arg("--list-keys")
            .arg(key_id.unwrap())
            .output();

        match output {
            Ok(o) if o.status.success() => true,
            _=> false,
        }
    }

    fn gpg_sign(key_id: Option<String>, key_pass: Option<String>, gpg_path: Option<String>, bundle_path: &PathBuf) -> String{
        //prepare signature path: <binary>.asc
        let mut sig_path = bundle_path.clone();
        sig_path.set_extension("asc");

        //build the gpg command
        let mut cmd = Command::new(gpg_path.unwrap());

        //handle the passphrase
        cmd.arg("--batch");
        cmd.arg("--no-tty");
        cmd.arg("--yes");
        cmd.arg("--pinentry-mode").arg("loopback");
        cmd.arg("--passphrase-fd").arg("0");

        //construct the signing command
        cmd.arg("--armor")
            .arg("--output")
            .arg(&sig_path)
            .arg("-u")
            .arg(&key_id.unwrap())
            .arg("--detach-sig")
            .arg("--verbose")
            .arg(&bundle_path.display().to_string())
            .stdin(Stdio::piped());



        //spawn the process
        let mut child = match cmd.spawn(){
            Ok(c) => c,
            Err(..) => return "Error spawning child process, GPG signing failed".to_string()
        };

        //write passphrase to stdin
        if let Some(pass) = key_pass {
            if let Some(mut stdin) = child.stdin.take() {
                match stdin.write(pass.as_bytes()) {
                    Ok(res) => res,
                    Err(..) => return "Error writing passphrase to bytes, GPG signing failed".to_string()
                };
            }
        }

        let output = match child.wait_with_output() {
            Ok(c) => c,
            Err(..) => return "Error with gpg child output spawn, GPG signing failed".to_string()
        };
        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return format!("GPG signing failed: {}", err_msg).into();
        }

        return format!("successfully signed {} with signature at {:?}", bundle_path.display(), sig_path.display());
        
    }
}