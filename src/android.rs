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
        println!("building the dynamic app bundle");
        let cwd: PathBuf = self.cwd.clone();
        println!("working dir: {:?}", cwd);
        let app_name = self.app_name.as_ref().unwrap();
        let release = if self.release {"release"} else {"debug"};
        let bundle_path: PathBuf = if self.release {
            format!("target/{}/android/app/src/main/res",release).into()
        }else {
            format!("target/{}/android/app/src/main/res",release).into()
        };
        println!("bundle path: {:?}", bundle_path);
        //establish mipmap dirs
        let hdpi_path: PathBuf = bundle_path.join("mipmap-hdpi");
        let mdpi_path: PathBuf = bundle_path.join("mipmap-mdpi");
        let xhdpi_path: PathBuf = bundle_path.join("mipmap-xhdpi");
        let xxhdpi_path: PathBuf = bundle_path.join("mipmap-xxhdpi");
        let xxxhdpi_path: PathBuf = bundle_path.join("mipmap-xxxhdpi");
        println!("mipmap paths: hdpi: {:?}, mdpi: {:?}, xhdpi: {:?}, xxhdpi: {:?}, xxxhdpi: {:?}", hdpi_path, mdpi_path, xhdpi_path, xxhdpi_path, xxxhdpi_path);
        //set the output path
        self.output_path = Some(cwd.join(&bundle_path));
        //check for a valid output path
        if self.output_path.as_ref().is_none() {
            return Err(PistonError::Generic("output path not provided".to_string()))
        }
        //Empty the directory if it already exists
        let path = bundle_path.as_path();
        if path.exists() && path.is_dir(){
            Helper::empty_directory(path)?
        }
        // //create the target directories
        create_dir_all(self.output_path.as_ref().unwrap()).map_err(|e| PistonError::CreateDirAllError {
        path: self.output_path.as_ref().unwrap().to_path_buf(),
        source: e,
        })?;


        //TODO
        //create mipmap-hdpi mipmap-mdpi mipmap-xhdpi mipmap-xxhdpi mipmap-xxxhdpi
        //convert icon to various mipmaps

        //reverse engineer cargo-apk


        // //create binary directories
        // create_dir_all(macos_path).map_err(|e| PistonError::CreateDirAllError {
        //     path: self.output_path.as_ref().unwrap().to_path_buf(),
        //     source: e,
        // })?;
        // //establish app icon target path ~/macos/release/Appname.app/Contents/Resources/macos_icon.icns
        // let icon_path: PathBuf = res_path.join("macos_icon.icns");
        // //establish Info.plist path ~/macos/release/Appname.app/Contents/Info.plist
        // let plist_path: PathBuf = partial_path.join("Info.plist");
        // println!("plist path: {:?}", plist_path);
        // //if a plist file exists, first remove it.
        // if plist_path.exists() {
        //     remove_file(&plist_path).map_err(|e| PistonError::RemoveFileError {
        //         path: plist_path.clone().to_path_buf(),
        //         source: e,
        //     })?;
        // }
        // //create a new Info.plist file
        // let mut plist_file = File::create(&plist_path).map_err(|e| PistonError::CreateFileError {
        //         path: plist_path.clone().to_path_buf(),
        //         source: e,
        //     })?;
        // //populate the Info.plist file
        // let plist_content = format!(
        //     r#"
        //     <?xml version="1.0" encoding="UTF-8"?>
        //     <!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN"
        //     "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
        //     <plist version="1.0">
        //     <dict>
        //         <key>CFBundleName</key>
        //         <string>{}</string>
        //         <key>CFBundleExecutable</key>
        //         <string>{}</string>
        //         <key>CFBundleIconFile</key>
        //         <string>macos_icon</string>
        //         <key>CFBundleVersion</key>
        //         <string>{}</string>
        //     </dict>
        //     </plist>
        //     "#,
        //     &capitalized,
        //     &self.app_name.as_ref().unwrap(),
        //     &self.app_version.as_ref().unwrap(),
        // );
        // plist_file.write_all(plist_content.as_bytes()).map_err(|e| PistonError::WriteFileError(e.to_string()))?;;
        // println!("Info.plist created");
        // //if icon path was provided...convert
        // if !self.icon_path.is_none(){
        //     println!("icon path provided, configuring icon");
        //     //convert the .png at icon_path to an .icns which resides in the app bundle
        //     println!("icon output path: {}", icon_path.display());
        //     let img_path_clone = self.icon_path.clone().unwrap();
        //     println!("image path clone: {}", &img_path_clone);
        //     let img_path = Path::new(&img_path_clone);
        //     println!("image path as path: {}", &img_path.display());
        //     let macos_icon = Command::new("sips")
        //         .args(["-s", "format", "icns", &img_path_clone, "--out", &icon_path.display().to_string()])
        //         .output()
        //         .map_err(|e| PistonError::MacOSIconError {
        //             input_path: img_path.to_path_buf(),
        //             output_path: icon_path,
        //             source: e,
        //         })?;
        //     println!("done configuring macos icon");
        // }
        // println!("done configuring MacOS bundle");
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