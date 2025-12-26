use std::path::PathBuf;
use std::collections::HashMap;
use cargo_metadata::{Metadata, MetadataCommand, DependencyKind};
use crate::PistonError;

pub struct MacOSBuilder {
    release: bool,
    target: String,
    cwd: PathBuf,
    output_path: Option<PathBuf>,
    icon_path: Option<String>,
    cargo_path: String,
    app_name: Option<String>,
}

impl MacOSBuilder {
    pub fn start(release: bool, target: String, cwd: PathBuf, env_vars: HashMap<String, String>) -> Result<(), PistonError> {
        println!("building for MacOS");
        //check operating system (requires MacOS)
        if std::env::consts::OS != "macos"{
            return Err(PistonError::UnsupportedOSError{os: std::env::consts::OS.to_string(), target: target})
        }
        let mut op = MacOSBuilder::new(release, target, cwd, env_vars)?;
        //TODO check for signing certificate & sign?
        //>>prebuild
        op.pre_build();

        //>>build
        op.build();

        //>>Postbuild
        op.post_build();

        Ok(())
    }

fn new(release: bool, target: String, cwd: PathBuf, env_vars: HashMap<String, String>) -> Result<Self, PistonError> {
        println!("creating MacOSBuilder: release: {:?}, target: {:?}, cwd: {:?}", release, target.to_string(), cwd);
        //parse env vars
        let cargo_path = env_vars.get("cargo_path").cloned().unwrap_or("cargo".to_string());
        println!("Cargo path determined: {}", &cargo_path);
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
        Ok(MacOSBuilder{release: release, target: target.to_string(), cwd: cwd, output_path: None, icon_path: icon_path, cargo_path: cargo_path, app_name: app_name})
    }

    fn pre_build(&mut self) -> Result<(), PistonError>{
        println!("pre build for macos");
        Ok(())
    }

    fn build(&mut self) -> Result<(), PistonError>{
        println!("build for macos");
        Ok(())
    }

    fn post_build(&mut self) -> Result<(), PistonError>{
        println!("post build for macos");
        Ok(())
    }
}

struct MacOSRunner{
device: String, 
}

impl MacOSRunner {

    pub fn start() -> Result<(), PistonError> {
        println!("running for MacOS");
        // let device = "device";
        // let mut op = MacOSRunner::new(device)?;
        //TODO check for signing certificate & sign?
        //>>run
        // op.run();

        Ok(())
    }
    fn new() -> Self {
        println!("Running for MacOS");

        MacOSRunner{device: "device".to_string()}
    }
}