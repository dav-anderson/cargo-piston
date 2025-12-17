use cargo_metadata::{Metadata, MetadataCommand, DependencyKind};
use std::path::PathBuf;
use crate::error::PistonError;


pub struct LinuxBuilder {
    release: bool,
    target: String,
    cwd: PathBuf,
    output_path: Option<PathBuf>,
    icon_path: Option<String>,
    cargo_path: String,
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
        Ok(LinuxBuilder{release: release, target: target.to_string(), cwd: cwd, output_path: None, icon_path: icon_path, cargo_path: cargo_path, app_name: app_name})
    }

    fn pre_build(&mut self) -> Result<(), PistonError>{
        println!("pre build for linux");
        Ok(())
    }

    fn build(&mut self) -> Result<(), PistonError>{
        println!("Build for linux");
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