use cargo_metadata::{Metadata, MetadataCommand};
use anyhow::{bail, Result};
use std::env;


pub struct WindowsBuilder {
    release: bool,
    target: String,
}

impl WindowsBuilder {
    pub fn start(release: bool, target: String) {
    println!("Building for Windows");
    let op = WindowsBuilder::new(release, target);
    //>>prebuild
    //setup the app bundle
    //read icon path and configure icon
    //-check for signing certificate
    op.pre_build();

    //>>build
    //op.build()

    //>>Postbuild
    //move binary to the app bundle and sign
    //op.post_build()
    }

    fn new(release: bool, target: String) -> Self {
        WindowsBuilder{release: release, target: target.to_string()}
    }

    fn pre_build(&self) -> Result<()>{
        // Parse local Cargo.toml from current dir
        let cwd = match env::current_dir(){
            Ok(cwd) => cwd,
            Err(_) => bail!("error parsing cargo.toml")
        };
        let metadata = MetadataCommand::new()
            .current_dir(cwd)
            .exec()?;
        // Read standard fields from the first package
        if let Some(package) = metadata.packages.first() {
            println!("Package name: {}", package.name);
            println!("Version: {}", package.version);
            // Read dependencies (example: check if "clap" is a dep)
            if let Some(dep) = package.dependencies.iter().find(|d| d.name == "clap") {
                println!("Clap dependency version req: {}", dep.req);
            }
            // Read custom [package.metadata] keys (if present)
            if let serde_json::Value::Object(meta) = &package.metadata {
                if let Some(icon_path) = meta.get("icon_path") {
                    println!("Custom icon_path: {}", icon_path);
                }
            }
        } else {
            println!("No packages found in Cargo.toml");
        }
        Ok(())
    }
}






