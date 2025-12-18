use cargo_metadata::{Metadata, MetadataCommand, DependencyKind};
use std::path::PathBuf;
use std::env;
use std::fs::create_dir_all;
use std::process::{Command, Stdio};
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
    app_name: Option<String>,
}

impl LinuxBuilder {
    pub fn start(release: bool, target: String, cwd: PathBuf, cargo_path: String) -> Result<(), PistonError> {
    println!("building for linux");
    let mut op = LinuxBuilder::new(release, target, cwd, cargo_path)?;
    //TODO check for signing certificate & sign?
    //>>prebuild
    op.pre_build();

    //>>build
    op.build();

    //>>Postbuild
    op.post_build();

    Ok(())
    }

    fn new(release: bool, target: String, cwd: PathBuf, cargo_path: String) -> Result<Self, PistonError> {
        println!("creating LinuxBuilder: release: {:?}, target: {:?}, cwd: {:?}", release, target.to_string(), cwd);
        //parse cargo.toml
        let metadata: Metadata = MetadataCommand::new()
            .current_dir(cwd.clone())
            .exec()
            .map_err(|e| PistonError::CargoParseError(e.to_string()))?;

        let mut icon_path: Option<String> = None;
        let mut app_name: Option<String> = None;
        // Read standard fields from the first package
        if let Some(package) = metadata.root_package() {
            println!("Package name: {}", package.name);
            app_name = Some(package.name.to_string());
            println!("Version: {}", package.version);
            // Read custom [package.metadata] keys (if present)
            if let serde_json::Value::Object(meta) = &package.metadata {
                if let Some(value) = meta.get("icon_path") {
                    if let serde_json::Value::String(s) = value {
                        icon_path = Some(s.to_string());
                    }
                }
            }
        } else {
            println!("No packages found in Cargo.toml");
        }
        //parse the path to zigbuild if building on Macos
        let mut zigbuild_path: Option<String> = None;
        //MACOS ONLY
        if std::env::consts::OS == "macos"{
            //parse zigbuild path from .env
            dotenv::dotenv().ok();
            zigbuild_path = Some(env::var("zigbuild_path").map_err(|e| PistonError::ZigbuildMissingError(e.to_string()))?);
        }   
        
        Ok(LinuxBuilder{release: release, target: target.to_string(), cwd: cwd, output_path: None, icon_path: icon_path, cargo_path: cargo_path, zigbuild_path: zigbuild_path, app_name: app_name})
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
        if path.exists() && path.is_dir(){
            Helper::empty_directory(path)?
        }
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
        //MACOS HOST ONLY
        //TODO zigbuild linker is busted here, see ramp gui...figure out why it has a .env(new path) pass
        if std::env::consts::OS == "macos"{
            let output = Command::new("bash")
                .arg("-c")
                .arg(format!("{} {}", self.zigbuild_path.as_ref().unwrap(), &cargo_cmd))
                .current_dir(self.cwd.clone())
                //TODO is this the culprit for the linker error?
                .env("PATH", self.cwd.clone())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output();
                //LINUX HOST
        }else{
            let output = Command::new("bash")
            .arg("-c")
            .arg(&cargo_cmd)
            .current_dir(self.cwd.clone())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output();
        }
        Ok(())
    }

    fn post_build(&mut self) -> Result<(), PistonError>{
        println!("post build for linux");
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