use std::path::{ Path, PathBuf };
use std::collections::HashMap;
use cargo_metadata::{ Metadata, MetadataCommand };
use std::fs::{ File, create_dir_all, copy, remove_file};
use std::io::Write;
use std::process::{ Command, Stdio };
use crate::Helper;
use crate::PistonError;

pub struct AndroidBuilder {
    release: bool,
    target: String,
    cwd: PathBuf,
    output_path: Option<PathBuf>,
    icon_path: Option<String>,
    cargo_path: String,
    app_name: Option<String>,
    app_version: Option<String>
}

impl AndroidBuilder {
    pub fn start(release: bool, target: String, cwd: PathBuf, env_vars: HashMap<String, String>) -> Result<(), PistonError>{
    println!("building for android");
    let mut op = AndroidBuilder::new(release, target, cwd, env_vars)?;

    //>>prebuild
    //TODO check for signing certificate
    op.pre_build()?;

    //>>build
    op.build()?;

    //>>Postbuild
    op.post_build()?;

    Ok(())
    }

    fn new(release: bool, target: String, cwd: PathBuf, env_vars: HashMap<String, String>) -> Result<Self, PistonError> {
        println!("creating AndroidBuilder: release: {:?}, target: {:?}, cwd: {:?}", release, target.to_string(), cwd);
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
        let mut app_version: Option<String> = None;
        // Read standard fields from the first package
        if let Some(package) = metadata.root_package() {
            println!("Package name: {}", package.name);
            app_name = Some(package.name.to_string());
            println!("Version: {}", package.version);
            app_version = Some(package.version.to_string());
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
        Ok(AndroidBuilder{release: release, target: target.to_string(), cwd: cwd, output_path: None, icon_path: icon_path, cargo_path: cargo_path, app_name: app_name, app_version: app_version})
    }

    fn pre_build(&mut self) -> Result <(), PistonError>{
        println!("pre build for android");
        Ok(())
    }

    fn build(&mut self) -> Result <(), PistonError>{
        println!("building for android");
        Ok(())
    }

    fn post_build(&mut self) -> Result <(), PistonError>{
        println!("post build for android");
        Ok(())
    }

}



struct AndroidRunner{
device: String, 
}

impl AndroidRunner{
    fn new() -> Self{
        println!("running for android");

        AndroidRunner{device: "device".to_string()}
    }
}