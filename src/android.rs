use std::path::{ Path, PathBuf };
use std::collections::HashMap;
use cargo_metadata::{ Metadata, MetadataCommand };
use std::fs::{ File, create_dir_all, copy, remove_file};
use std::io::{ Write, BufWriter };
use std::process::{ Command, Stdio };
use serde::Deserialize;
use serde_json::Value;
use crate::Helper;
use crate::PistonError;

#[derive(Deserialize, Default)]
struct AndroidMetadata {
    #[serde(default)]
    package: Option<String>,
    #[serde(default)]
    version_code: Option<u32>,
    #[serde(default)]
    version_name: Option<String>,
    #[serde(default)]
    min_sdk_version: Option<u32>,
    #[serde(default)]
    target_sdk_version: Option<u32>,
    #[serde(default)]
    label: Option<String>
}

#[derive(Deserialize, Default)]
struct AndroidManifest {
    package: String,
    version_code: u32,
    version_name: String,
    min_sdk_version: u32,
    target_sdk_version: u32,
    app_label: String,
    app_name: String,
    icon: String,
}

impl AndroidManifest {
    pub fn build(metadata: &Metadata, app_name: String) -> Result<Self, PistonError> {
        let package = metadata.root_package()
            .ok_or_else(|| PistonError::ParseManifestError("No Root package found in metadata".to_string()))?;
        let crate_name = package.name.clone();
        //extract [package.metadata.android] as JSON values
        let android_meta_value: Value = package.metadata
            .get("android")
            .cloned()
            .unwrap_or(Value::Object(Default::default()));
        //Deserialize to structured metadata
        let android_meta: AndroidMetadata = serde_json::from_value(android_meta_value)
            .map_err(|e| PistonError::ParseManifestError(format!("Failed to deserialize [package.metadata.android: {}]", e)))?;
        //Build the manifest with extracted values or defaults
        let mut manifest = Self::default();
        manifest.package = android_meta.package
            .unwrap_or(format!("com.example.{}", crate_name));
        manifest.version_code = android_meta.version_code
            .unwrap_or(1);
        manifest.version_name = android_meta.version_name
            .unwrap_or("1.0".to_string());
        manifest.min_sdk_version = android_meta.min_sdk_version
            .unwrap_or(21);
        manifest.target_sdk_version = android_meta.target_sdk_version
            .unwrap_or(34);
        manifest.app_label = android_meta.label
            .unwrap_or(format!("{} App", crate_name));
        manifest.app_name = app_name;
        manifest.icon = "@mipmap/ic_launcher".to_string();

        Ok(manifest)
    }

    pub fn to_xml(&self) -> String {
        let icon_attr = format!(r#" android:icon={}""#, Self::escape_xml(&self.icon));

        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
            <manifest xmlns:android="http://schemas.android.com/apk/res/android"
                package="{package}"
                android:versionCode="{version_code}"
                android:versionName="{version_name}">

                <uses-sdk android:minSdkVersion="{min_sdk}" android:targetSdkVersion="{target_sdk}" />

                <application android:label="{label}" android:hasCode="false"{icon_attr}>
                    <activity android:name="android.app.NativeActivity"
                        android:label="{label}"
                        android:exported="true">
                        <meta-data android:name="android.app.lib_name" android:value="{app_name}" />
                        <intent-filter>
                            <action android:name="android.intent.action.MAIN" />
                            <category android:name="android.intent.category.LAUNCHER" />
                        </intent-filter>
                    </activity>
                </application>
            </manifest>"#,
            package = Self::escape_xml(&self.package),
            version_code = self.version_code,
            version_name = Self::escape_xml(&self.version_name),
            min_sdk = self.min_sdk_version,
            target_sdk = self.target_sdk_version,
            label = Self::escape_xml(&self.app_label),
            app_name = Self::escape_xml(&self.app_name),  // Using app_name for lib_name in meta-data
            icon_attr = icon_attr,
        )
    }

    pub fn write_to(&self, dir: &Path) -> Result<(), PistonError> {
        println!("writing manifest...");
        let path = dir.join("AndroidManifest.xml");
        println!("Manifest path: {:?}", path);
        let file = File::create(&path)
            .map_err(|e| PistonError::CreateManifestError(format!("Failed to create manifest file: {}", e)))?;
        
        let mut writer = BufWriter::new(file);
        writer.write_all(self.to_xml().as_bytes())
            .map_err(|e| PistonError::WriteManifestError(format!("Failed to write manifest file: {}", e)))?;

        Ok(())
    }

    pub fn escape_xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
}

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
        //set the absolute output path
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
        //establish absolute paths for  mipmap dirs
        let hdpi_path: PathBuf = cwd.join(&bundle_path).join("mipmap-hdpi");
        let mdpi_path: PathBuf = cwd.join(&bundle_path).join("mipmap-mdpi");
        let xhdpi_path: PathBuf = cwd.join(&bundle_path).join("mipmap-xhdpi");
        let xxhdpi_path: PathBuf = cwd.join(&bundle_path).join("mipmap-xxhdpi");
        let xxxhdpi_path: PathBuf = cwd.join(&bundle_path).join("mipmap-xxxhdpi");
        println!("mipmap paths: hdpi: {:?}, mdpi: {:?}, xhdpi: {:?}, xxhdpi: {:?}, xxxhdpi: {:?}", hdpi_path, mdpi_path, xhdpi_path, xxhdpi_path, xxxhdpi_path);
        //create mipmap dirs
        create_dir_all(&hdpi_path).map_err(|e| PistonError::CreateDirAllError {
        path: hdpi_path.clone(),
        source: e,
        })?;
        create_dir_all(&mdpi_path).map_err(|e| PistonError::CreateDirAllError {
        path: mdpi_path.clone(),
        source: e,
        })?;
        create_dir_all(&xhdpi_path).map_err(|e| PistonError::CreateDirAllError {
        path: xhdpi_path.clone(),
        source: e,
        })?;
        create_dir_all(&xxhdpi_path).map_err(|e| PistonError::CreateDirAllError {
        path: xxhdpi_path.clone(),
        source: e,
        })?;
        create_dir_all(&xxxhdpi_path).map_err(|e| PistonError::CreateDirAllError {
        path: xxxhdpi_path.clone(),
        source: e,
        })?;
        //convert icon to various mipmaps
        let hdpi_target: PathBuf = hdpi_path.join("ic_launcher.png");
        Helper::resize_png(&self.icon_path.as_ref().unwrap(), &hdpi_target.display().to_string(), 48, 48)?;
        let mdpi_target: PathBuf = mdpi_path.join("ic_launcher.png");
        Helper::resize_png(&self.icon_path.as_ref().unwrap(), &mdpi_target.display().to_string(), 72, 72)?;
        let xhdpi_target: PathBuf = xhdpi_path.join("ic_launcher.png");
        Helper::resize_png(&self.icon_path.as_ref().unwrap(), &xhdpi_target.display().to_string(), 96, 96)?;
        let xxhdpi_target: PathBuf = xxhdpi_path.join("ic_launcher.png");
        Helper::resize_png(&self.icon_path.as_ref().unwrap(), &xxhdpi_target.display().to_string(), 144, 144)?;
        let xxxhdpi_target: PathBuf = xxxhdpi_path.join("ic_launcher.png");
        Helper::resize_png(&self.icon_path.as_ref().unwrap(), &xxxhdpi_target.display().to_string(), 192, 192)?;
        //parse cargotoml & generate androidmanifest xml
        let metadata: Metadata = MetadataCommand::new()
            .current_dir(cwd.clone())
            .exec()
            .map_err(|e| PistonError::CargoParseError(e.to_string()))?;
        let manifest = AndroidManifest::build(&metadata, self.app_name.as_ref().unwrap().to_string())?;
        //manifest path is cwd/target/android/AndroidManifest.xml
        let manifest_path: PathBuf = cwd.join("target").join(if self.release { "release" } else { "debug" }).join("android");
        println!("manifest path: {:?}", manifest_path);
        //write the manifest file
        manifest.write_to(&manifest_path.as_path())?;

        //TDOD proceed to aapt2 with generated AndroidManifest.xml path

        //TODO reverse engineer cargo-apk

        println!("done configuring Android bundle");
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