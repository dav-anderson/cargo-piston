use cargo_metadata::{Metadata, MetadataCommand, DependencyKind};
use std::env;
use std::path::{Path, PathBuf};
use std::fs::{create_dir_all, write, remove_file, copy};
use std::io::Write;
use std::fs::File;
use std::process::{Command, Stdio};
use serde_json::Value;
use image::{self, imageops, DynamicImage, ImageEncoder};
use crate::helper::Helper;
use crate::error::PistonError;

pub struct WindowsBuilder {
    release: bool,
    target: String,
    cwd: PathBuf,
    output_path: Option<PathBuf>,
    icon_path: Option<String>,
    embed_resources_ok: bool,
    cargo_path: String,
    app_name: Option<String>,
}

impl WindowsBuilder {
    pub fn start(release: bool, target: String, cwd: PathBuf, cargo_path: String) -> Result<(), PistonError> {
    println!("Building for Windows");
    let mut op = WindowsBuilder::new(release, target, cwd, cargo_path)?;
    //TODO check for signing certificate & sign?
    //>>prebuild
    op.pre_build()?;

    //>>build
    op.build()?;

    //>>Postbuild
    op.post_build()?;

    Ok(())
    }

    fn new(release: bool, target: String, cwd: PathBuf, cargo_path: String) -> Result<Self, PistonError> {
        println!("creating windowsBuilder: release: {:?}, target: {:?}, cwd: {:?}", release, target.to_string(), cwd);
        //parse cargo.toml
        let metadata: Metadata = MetadataCommand::new()
            .current_dir(cwd.clone())
            .exec()
            .map_err(|e| PistonError::CargoParseError(e.to_string()))?;

        // check if embed resources is installed
        let embed_resources_ok: bool = if let Some(root_pkg) = metadata.root_package() {
            root_pkg.dependencies.iter().any(|dep| dep.name == "embed-resource" && dep.kind == DependencyKind::Build)
        } else {
            false
        };
        println!("Embed Resources Installed: {}", embed_resources_ok);
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
        if app_name == None {
            return Err(PistonError::CargoParseError("Could not parse app name from cargo.toml".to_string()))
        }
        Ok(WindowsBuilder{release: release, target: target.to_string(), cwd: cwd, output_path: None, icon_path: icon_path, embed_resources_ok: embed_resources_ok, cargo_path: cargo_path, app_name: app_name})
    }

    fn pre_build(&mut self) -> Result <(), PistonError>{
        println!("building the dynamic app bundle");
        let cwd: PathBuf = self.cwd.clone();
        println!("working dir: {:?}", cwd);
        let rel_output: PathBuf = if self.release {
            "target/release/windows".into()
        }else {"target/debug/windows".into()};
        self.output_path = Some(cwd.join(&rel_output));
        println!("windows dir: {:?}", self.output_path);
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
        let rc_path: PathBuf = self.output_path.as_ref().unwrap().join("app.rc");
        let rc_icon: &PathBuf = &rel_output.join("windows_icon.ico");
        let content = format!("IDI_ICON1 ICON \"{}\"", rc_icon.display());
        //create the app.rc file
        write(&rc_path, content.as_bytes()).map_err(|e| PistonError::WriteFileError(e.to_string()))?;
        println!("created {:?} with content: {}", rc_path, content);   
        //TODO add a winres config check to the cargo.toml for app naming...or maybe just automate this?
        //[package.metadata.winres]
        //OriginalFilename = "<appname>.exe"
        //if icon path was provided...embed
        if !self.icon_path.is_none() && self.embed_resources_ok{
            println!("icon path provided and embed resources installed, configuring icon");
            //convert the .png at icon_path to a .ico which resides in the app bundle
            let icon_output: PathBuf = cwd.join(rc_icon);
            println!("icon output path: {}", icon_output.display());
            let img_path_clone = self.icon_path.clone().unwrap();
            println!("image path clone: {}", &img_path_clone);
            let img_path = Path::new(&img_path_clone);
            println!("image path as path: {}", &img_path.display());
            //open the image
            let img = image::open(img_path).map_err(|e| PistonError::OpenImageError {
            path: img_path.to_path_buf(),
            source: e,
            })?;
            println!("image opened");
            // Resize to the specified size
            let resized = imageops::resize(&img, 64, 64, imageops::FilterType::Lanczos3);
            println!("image resized");
            let resized_img = DynamicImage::ImageRgba8(resized);
            println!("image converted");
            //create the image file
            let file = std::fs::File::create(icon_output.clone()).map_err(|e| PistonError::CreateFileError {
                path: icon_output.clone().to_path_buf(),
                source: e,
            })?;
            //write the image file
            let mut writer = std::io::BufWriter::new(file);
            println!("new image file written");
            //encode the image file
            let encoder = image::codecs::ico::IcoEncoder::new(&mut writer);
            encoder.write_image(
                    resized_img.as_bytes(),
                    64,
                    64,
                    image::ExtendedColorType::Rgba8,
            ).map_err(|e| PistonError::WriteImageError(e))?;
            println!("Converted {} to ICO ({}x{}) and saved as {}", self.icon_path.as_ref().unwrap(), 64, 64, icon_output.display());
            let build_path: PathBuf = cwd.join("build.rs");
            //if a build.rs file exists, first remove it.
            if build_path.exists() {
                remove_file(&build_path).map_err(|e| PistonError::RemoveFileError {
                    path: build_path.clone().to_path_buf(),
                    source: e,
                })?;
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
            let mut build_file = File::create(&build_path).map_err(|e| PistonError::CreateFileError {
                path: build_path.clone().to_path_buf(),
                source: e
            })?;
            //write the file and flush the buffer
            build_file.write_all(build_content.as_bytes()).map_err(|e| PistonError::WriteFileError(e.to_string()))?;
            build_file.flush().map_err(|e| PistonError::FileFlushError(e.to_string()))?;
            println!("Created Build.rs at {}", &build_path.display());
        }
        println!("done configuring Windows bundle");
        Ok(())
        
    }

    fn build(&self) -> Result<(), PistonError> {
        println!("building");
        //build the binary for the specified target
        let cargo_args = format!("build --target {} {}", self.target, if self.release {"--release"} else {""});
        let cargo_cmd = format!("{} {}", self.cargo_path, cargo_args);
        let output = Command::new("bash")
            .arg("-c")
            .arg(&cargo_cmd)
            .current_dir(self.cwd.clone())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output();
        Ok(())
    }

    fn post_build(&self) -> Result<(), PistonError>{
        println!("post building");
        let binary_path = self.cwd.join("target").join(self.target.clone()).join(if self.release {"release"} else {"debug"}).join(format!("{}.exe", self.app_name.as_ref().unwrap()));
        let bundle_path = self.output_path.as_ref().unwrap().join(format!("{}.exe", self.app_name.as_ref().unwrap()));
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
        //output the proper location in the terminal for the user to see 
        println!("app bundle available at: {}", &bundle_path.display());
        Ok(())
    }
}






