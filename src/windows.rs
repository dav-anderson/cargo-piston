use cargo_metadata::{Metadata, MetadataCommand, DependencyKind};
use anyhow::{Context, bail, Result};
use std::env;
use std::path::{Path, PathBuf};
use std::fs::{create_dir_all, write, remove_file};
use std::io::Write;
use std::fs::File;
use image::{self, imageops, DynamicImage, ImageEncoder};
use crate::helper::Helper;


pub struct WindowsBuilder {
    release: bool,
    target: String,
    cwd: PathBuf,
    output_path: Option<PathBuf>
}

impl WindowsBuilder {
    pub fn start(release: bool, target: String, cwd: PathBuf) {
    println!("Building for Windows");
    let mut op = WindowsBuilder::new(release, target, cwd);
    //TODO check for signing certificate
    op.pre_build();

    //>>build
    op.build();

    //>>Postbuild
    //TODO move binary to the app bundle and sign
    op.post_build();
    }

    fn new(release: bool, target: String, cwd: PathBuf) -> Self {
        println!("creating windowsBuilder: release: {:?}, target: {:?}, cwd: {:?}", release, target.to_string(), cwd);
        WindowsBuilder{release: release, target: target.to_string(), cwd: cwd, output_path: None}
    }

    fn pre_build(&mut self) -> Result<()>{
        //parse cargo.toml
        let metadata: Metadata = match MetadataCommand::new()
            .current_dir(self.cwd.clone())
            .exec() {
                Ok(md) => md,
                Err(_) => bail!("error parsing cargo toml")
        };

        // check if embed resources is installed
        let embed_resources_ok: bool = if let Some(root_pkg) = metadata.root_package() {
            root_pkg.dependencies.iter().any(|dep| dep.name == "embed-resource" && dep.kind == DependencyKind::Build)
        } else {
            false
        };
        println!("Embed Resources Installed: {}", embed_resources_ok);
        // Read standard fields from the first package
        if let Some(package) = metadata.root_package() {
            println!("Package name: {}", package.name);
            println!("Version: {}", package.version);
            // Read custom [package.metadata] keys (if present)
            if let serde_json::Value::Object(meta) = &package.metadata {
                println!("testing***************");
                //parse icon_path from the cargo.toml
                if let Some(icon_path) = meta.get("icon_path") {
                    println!("icon_path: {}", icon_path);
                    self.configure_bundle(Some(icon_path.to_string()), embed_resources_ok);
                }else {
                    println!("no icon path found");
                    self.configure_bundle(None, embed_resources_ok);
                }
            }
        } else {
            println!("No packages found in Cargo.toml");
        }
        Ok(())
    }

    fn configure_bundle(&mut self, icon_path: Option<String>, embed_resources_ok: bool) -> Result <()>{
        println!("building the dynamic app bundle");
        let cwd: PathBuf = self.cwd.clone();
        println!("working dir: {:?}", cwd);
        let rel_output: PathBuf = if self.release {
            "target/release/windows".into()
        }else {"target/debug/windows".into()};
        self.output_path = Some(cwd.join(&rel_output));
        println!("windows dir: {:?}", self.output_path);
        //empty the target directory if it exists
        let path = self.output_path.as_ref().unwrap().as_path();
        if path.exists() && path.is_dir(){
            Helper::empty_directory(path)?
        }
        //create the target directory
        create_dir_all(self.output_path.as_ref().unwrap())?;
        let rc_path: PathBuf = self.output_path.as_ref().unwrap().join("app.rc");
        let rc_icon: &PathBuf = &rel_output.join("windows_icon.ico");
        let content = format!("IDI_ICON1 ICON \"{}\"", rc_icon.display());
        //create the app.rc file
        write(&rc_path, content.as_bytes())?;
        println!("created {:?} with content: {}", rc_path, content);   
        //TODO open question: Will the App.rc compiling break the bundle if the user does not provide an icon?
        println!("Icon path: {}", &icon_path.clone().unwrap());
        //if no icon was provided
        if !icon_path.is_none() && embed_resources_ok{
            println!("icon path provided and embed resources installed, configuring icon");
            //TODO convert the .png at the icon_path to a .ico which resides in the app bundle
            let icon_output: PathBuf = cwd.join(rc_icon);
            println!("icon output path: {}", icon_output.display());
            //TODO SOMETHING IS BREAKING BEYOND THIS LINE AND NOT GETTING HANDLED
            let img = image::open(Path::new(&icon_path.clone().unwrap()))?;
            // Resize to the specified size
            let resized = imageops::resize(&img, 64, 64, imageops::FilterType::Lanczos3);
            let resized_img = DynamicImage::ImageRgba8(resized);
            let file = std::fs::File::create(icon_output.clone())?;
            let mut writer = std::io::BufWriter::new(file);
            let encoder = image::codecs::ico::IcoEncoder::new(&mut writer);
            encoder
                .write_image(
                    resized_img.as_bytes(),
                    64,
                    64,
                    image::ExtendedColorType::Rgba8,
                )?;
            println!("Converted {} to ICO ({}x{}) and saved as {}",icon_path.unwrap(), 64, 64, icon_output.display());
            let build_path: PathBuf = cwd.join("build.rs");
            //if a build.rs file exists, first remove it.
            if build_path.exists() {
                remove_file(&build_path).context("failed to remove existing build.rs")?;
            }
            //populate the build.rs content
            let build_content = format!(
                r#"
                use std::io;

                fn main() {{
                    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" && std::path::Path::new("{}").exists() {{
                        embed_resource::compile("app.rc", embed_resource::NONE)
                        .manifest_optional();
                    }}
            }}
            
                "#,
                &icon_output.display()
            );
            //Generate a build.rs file
            let mut build_file = File::create(&build_path)?;
            build_file.write_all(build_content.as_bytes())?;
            build_file.flush()?;
            println!("Created Build.rs at {}", &build_path.display());
        }
        println!("done configuring Windows bundle");
        Ok(())
        
    }

    fn build(&self) -> Result<()> {
        println!("building");
        //TODO build the app binary
        Ok(())
    }

    fn post_build(&self) -> Result<()>{
        println!("post building");
        //TODO move the app binary to the proper location
        Ok(())
    }
}






