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
    op.build();

    //>>Postbuild
    //move binary to the app bundle and sign
    op.post_build();
    }

    fn new(release: bool, target: String) -> Self {
        WindowsBuilder{release: release, target: target.to_string()}
    }

    fn pre_build(&self) -> Result<()>{

        // Parse local current working dir
        let cwd = match env::current_dir(){
            Ok(cwd) => cwd,
            Err(_) => bail!("error getting working directory")
        };
        //TODO create app bundle

        //parse cargo.toml
        let metadata = match MetadataCommand::new()
            .current_dir(cwd)
            .exec() {
                Ok(md) => md,
                Err(_) => bail!("error parsing cargo toml")
            };

        //TODO parse icon path from the cargo.toml

        //TODO convert .png to .ico and deliver it inside of windows app bundle

        //TODO create the app.rc pointing to the path to the .ico from the previous step

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
                }else {
                    println!("no icon path found")
                }
            }
        } else {
            println!("No packages found in Cargo.toml");
        }
        Ok(())
    }

    fn build(&self) -> Result<()> {
        println!("building");
        Ok(())
    }

    fn post_build(&self) -> Result<()>{
        println!("post building");
        Ok(())
    }
}






