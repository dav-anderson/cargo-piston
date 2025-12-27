use std::path::PathBuf;
use std::collections::HashMap;
use cargo_metadata::{Metadata, MetadataCommand};
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
        op.pre_build()?;

        //>>build
        op.build()?;

        //>>Postbuild
        op.post_build()?;

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

     fn pre_build(&mut self) -> Result <(), PistonError>{
        println!("Pre build for macos");
        println!("building the dynamic app bundle");
        // let cwd: PathBuf = self.cwd.clone();
        // println!("working dir: {:?}", cwd);
        // let rel_output: PathBuf = if self.release {
        //     "target/release/windows".into()
        // }else {"target/debug/windows".into()};
        // self.output_path = Some(cwd.join(&rel_output));
        // println!("windows dir: {:?}", self.output_path);
        // //empty the target directory if it exists
        // if self.output_path.as_ref().is_none() {
        //     return Err(PistonError::Generic("output path not provided".to_string()))
        // }
        // let path = self.output_path.as_ref().unwrap().as_path();
        // if path.exists() && path.is_dir(){
        //     Helper::empty_directory(path)?
        // }
        // //create the target directory
        // create_dir_all(path).map_err(|e| PistonError::CreateDirAllError {
        // path: self.output_path.as_ref().unwrap().to_path_buf(),
        // source: e,
        // })?;
        // let rc_path: PathBuf = self.output_path.as_ref().unwrap().join("app.rc");
        // let rc_icon: &PathBuf = &rel_output.join("windows_icon.ico");
        // let content = format!("IDI_ICON1 ICON \"{}\"", rc_icon.display());
        // //create the app.rc file
        // write(&rc_path, content.as_bytes()).map_err(|e| PistonError::WriteFileError(e.to_string()))?;
        // println!("created {:?} with content: {}", rc_path, content);   
        // //TODO add a winres config check to the cargo.toml for app naming...or maybe just automate this?
        // //[package.metadata.winres]
        // //OriginalFilename = "<appname>.exe"
        // //if icon path was provided...embed
        // if !self.icon_path.is_none() && self.embed_resources_ok{
        //     println!("icon path provided and embed resources installed, configuring icon");
        //     //convert the .png at icon_path to a .ico which resides in the app bundle
        //     let icon_output: PathBuf = cwd.join(rc_icon);
        //     println!("icon output path: {}", icon_output.display());
        //     let img_path_clone = self.icon_path.clone().unwrap();
        //     println!("image path clone: {}", &img_path_clone);
        //     let img_path = Path::new(&img_path_clone);
        //     println!("image path as path: {}", &img_path.display());
        //     //open the image
        //     let img = image::open(img_path).map_err(|e| PistonError::OpenImageError {
        //     path: img_path.to_path_buf(),
        //     source: e,
        //     })?;
        //     println!("image opened");
        //     // Resize to the specified size
        //     let resized = imageops::resize(&img, 64, 64, imageops::FilterType::Lanczos3);
        //     println!("image resized");
        //     let resized_img = DynamicImage::ImageRgba8(resized);
        //     println!("image converted");
        //     //create the image file
        //     let file = std::fs::File::create(icon_output.clone()).map_err(|e| PistonError::CreateFileError {
        //         path: icon_output.clone().to_path_buf(),
        //         source: e,
        //     })?;
        //     //write the image file
        //     let mut writer = std::io::BufWriter::new(file);
        //     println!("new image file written");
        //     //encode the image file
        //     let encoder = image::codecs::ico::IcoEncoder::new(&mut writer);
        //     encoder.write_image(
        //             resized_img.as_bytes(),
        //             64,
        //             64,
        //             image::ExtendedColorType::Rgba8,
        //     ).map_err(|e| PistonError::WriteImageError(e))?;
        //     println!("Converted {} to ICO ({}x{}) and saved as {}", self.icon_path.as_ref().unwrap(), 64, 64, icon_output.display());
        //     let build_path: PathBuf = cwd.join("build.rs");
        //     //if a build.rs file exists, first remove it.
        //     if build_path.exists() {
        //         remove_file(&build_path).map_err(|e| PistonError::RemoveFileError {
        //             path: build_path.clone().to_path_buf(),
        //             source: e,
        //         })?;
        //     }
        //     //populate the build.rs content
        //     let build_content = format!(
        //         r#"
        //         use std::io;

        //         fn main() {{
        //             if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" && std::path::Path::new("{}").exists() {{
        //                 embed_resource::compile("app.rc", embed_resource::NONE)
        //                 .manifest_optional();
        //             }}
        //     }}
            
        //         "#,
        //         &icon_output.display()
        //     );
        //     //Generate a build.rs file
        //     let mut build_file = File::create(&build_path).map_err(|e| PistonError::CreateFileError {
        //         path: build_path.clone().to_path_buf(),
        //         source: e
        //     })?;
        //     //write the file and flush the buffer
        //     build_file.write_all(build_content.as_bytes()).map_err(|e| PistonError::WriteFileError(e.to_string()))?;
        //     build_file.flush().map_err(|e| PistonError::FileFlushError(e.to_string()))?;
        //     println!("Created Build.rs at {}", &build_path.display());
        // }
        println!("done configuring MacOS bundle");
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
        if std::env::consts::OS != "macos"{
            println!("error cannot run mac on linux");
            // return Err(PistonError::UnsupportedOSError{os: std::env::consts::OS.to_string(), target: target})
        }
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