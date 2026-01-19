use std::path::{ Path, PathBuf };
use std::collections::HashMap;
use cargo_metadata::{ Metadata, MetadataCommand };
use std::fs::{ File, create_dir_all, copy, remove_file};
use std::fs;
use std::io::{ Write, BufWriter };
use std::process::{ Command, Stdio };
use serde::Deserialize;
use serde_json::Value;
use crate::Helper;
use crate::PistonError;

//TODO reverse engineer cargo-apk


//     fn link_manifest_and_resources(&self, compiled_res: &Path, base_dir: &Path) -> Result<(), PistonError> {
//         let aapt2_path = self.sdk_path.join(format!("build-tools/{}/aapt2", self.build_tools_version));
//         let android_jar = self.sdk_path.join(format!("platforms/android-{}/android.jar", self.target_sdk_version));
//         let manifest_path = self.build_dir.join("AndroidManifest.xml");
//         let res_arg = if compiled_res.exists() { format!(" {}", compiled_res.display()) } else { String::new() };

//         let link_command = format!(
//             "{} link --proto-format -o {} --manifest {} -I {} {}",
//             aapt2_path.display(),
//             base_dir.display(),
//             manifest_path.display(),
//             android_jar.display(),
//             res_arg
//         );

//         Command::new("bash")
//             .arg("-c")
//             .arg(&link_command)
//             .current_dir(&self.build_dir)
//             .env("ANDROID_HOME", self.sdk_path.to_str().unwrap())
//             .stdout(Stdio::inherit())
//             .stderr(Stdio::inherit())
//             .output()
//             .map_err(|e| PistonError::BuildError(format!("aapt2 link failed: {}", e)))?;

//         Ok(())
//     }

//     fn add_lib(&self, base_dir: &Path, artifact_name: &str, target_triple: &str) -> Result<(), PistonError> {
//         let abi = match target_triple {
//             "aarch64-linux-android" => "arm64-v8a",
//             // Add more mappings
//             _ => return Err(PistonError::BuildError("Unsupported target".to_string())),
//         };
//         let lib_dir = base_dir.join("lib").join(abi);
//         fs::create_dir_all(&lib_dir)?;

//         let so_path = self.build_dir.parent().unwrap().join(target_triple).join(if self.is_debug { "debug" } else { "release" }).join(format!("lib{}.so", artifact_name));  // Adjust path
//         fs::copy(&so_path, lib_dir.join(format!("lib{}.so", artifact_name)))
//             .map_err(|e| PistonError::BuildError(format!("Failed to copy .so: {}", e)))?;

//         Ok(())
//     }

//     fn zip_base(&self, base_dir: &Path, base_zip: &Path) -> Result<(), PistonError> {
//         let zip_command = format!(
//             "cd {} && zip -r ../base.zip *",
//             base_dir.display()
//         );

//         Command::new("bash")
//             .arg("-c")
//             .arg(&zip_command)
//             .current_dir(&self.build_dir)
//             .stdout(Stdio::inherit())
//             .stderr(Stdio::inherit())
//             .output()
//             .map_err(|e| PistonError::BuildError(format!("Zip failed: {}", e)))?;

//         Ok(())
//     }

//     fn build_bundle(&self, base_zip: &Path, aab_path: &Path) -> Result<(), PistonError> {
//         let bundle_command = format!(
//             "java -jar {} build-bundle --modules={} --output={}",
//             self.bundletool_jar_path.display(),
//             base_zip.display(),
//             aab_path.display()
//         );

//         Command::new("bash")
//             .arg("-c")
//             .arg(&bundle_command)
//             .current_dir(&self.build_dir)
//             .env("JAVA_HOME", self.java_path.to_str().unwrap())
//             .stdout(Stdio::inherit())
//             .stderr(Stdio::inherit())
//             .output()
//             .map_err(|e| PistonError::BuildError(format!("bundletool failed: {}", e)))?;

//         Ok(())
//     }

//     fn sign_aab(&self, aab_path: &Path) -> Result<(), PistonError> {
//         let profile_name = if self.is_debug { "dev" } else { "release" };
//         let keystore_env = format!("CARGO_APK_{}_KEYSTORE", profile_name.to_uppercase());
//         let password_env = format!("{}_PASSWORD", keystore_env);

//         let keystore_path = std::env::var_os(&keystore_env).map(PathBuf::from);
//         let password = std::env::var(&password_env).ok();

//         let signing_key = match (keystore_path, password) {
//             Some(path), Some(pass) => (path, pass),
//             _ if self.is_debug => {
//                 // Fall back to default debug key (mirrors cargo-apk; assume path from NDK or SDK)
//                 let default_key = self.sdk_path.join("debug.keystore");  // Generate if needed with keytool
//                 if !default_key.exists() {
//                     self.generate_debug_key(&default_key)?;
//                 }
//                 (default_key, "android".to_string())  // Default pass
//             }
//             _ => return Err(PistonError::BuildError("Missing release key".to_string())),
//         };

//         let apksigner_path = self.sdk_path.join(format!("build-tools/{}/apksigner", self.build_tools_version));

//         let sign_command = format!(
//             "{} sign --ks {} --ks-key-alias androiddebugkey --ks-pass pass:{} {}",
//             apksigner_path.display(),
//             signing_key.0.display(),
//             signing_key.1,
//             aab_path.display()
//         );

//         Command::new("bash")
//             .arg("-c")
//             .arg(&sign_command)
//             .current_dir(&self.build_dir)
//             .stdout(Stdio::inherit())
//             .stderr(Stdio::inherit())
//             .output()
//             .map_err(|e| PistonError::BuildError(format!("Signing failed: {}", e)))?;

//         Ok(())
//     }

//     fn generate_debug_key(&self, key_path: &Path) -> Result<(), PistonError> {
//         let keytool_command = format!(
//             "keytool -genkeypair -v -keystore {} -alias androiddebugkey -keyalg RSA -keysize 2048 -validity 10000 -dname \"CN=Android Debug,O=Android,C=US\" -storepass android -keypass android",
//             key_path.display()
//         );

//         Command::new("bash")
//             .arg("-c")
//             .arg(&keytool_command)
//             .env("JAVA_HOME", self.java_path.to_str().unwrap())
//             .stdout(Stdio::inherit())
//             .stderr(Stdio::inherit())
//             .output()
//             .map_err(|e| PistonError::BuildError(format!("keytool failed: {}", e)))?;

//         Ok(())
//     }

//     // Helper for recursive copy (minimum impl; expand as needed)
//     fn copy_dir_recursively(&self, src: &Path, dest: &Path) -> Result<(), PistonError> {
//         for entry in fs::read_dir(src)? {
//             let entry = entry?;
//             let ty = entry.file_type()?;
//             if ty.is_dir() {
//                 self.copy_dir_recursively(&entry.path(), &dest.join(entry.file_name()))?;
//             } else {
//                 fs::copy(entry.path(), dest.join(entry.file_name()))
//                     .map_err(|e| PistonError::BuildError(format!("Copy failed: {}", e)))?;
//             }
//         }
//         Ok(())
//     }
// }

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

#[derive(Deserialize, Default, Debug)]
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
    build_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    icon_path: Option<String>,
    cargo_path: String,
    app_name: Option<String>,
    app_version: Option<String>,
    manifest: AndroidManifest,
    ndk_path: String,
    sdk_path: String,
    java_path: String,
    resources: Option<String>,
    build_tools_version: String,
    bundletool_path: String,
    // assets: Option<PathBuf>,
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
        let cargo_path: String = env_vars.get("cargo_path").cloned().unwrap_or("cargo".to_string());
        // let resources_path: Option<String> = env_vars.get("resources_path").cloned();
        let ndk_path: &String = Helper::get_or_err(&env_vars, "ndk_path")?;
        let sdk_path: &String = Helper::get_or_err(&env_vars, "sdk_path")?;
        let java_path: &String = Helper::get_or_err(&env_vars, "java_path")?;
        let bundletool_path: &String = Helper::get_or_err(&env_vars, "bundletool_path")?;
        let build_tools_version: String = Helper::get_build_tools_version(&sdk_path)?;
        println!("Cargo path determined: {}", &cargo_path);
        //parse cargo.toml
        let metadata: Metadata = MetadataCommand::new()
            .current_dir(cwd.clone())
            .exec()
            .map_err(|e| PistonError::CargoParseError(e.to_string()))?;

        let mut icon_path: Option<String> = None;
        let mut app_name: Option<String> = None;
        let mut app_version: Option<String> = None;
        let mut resources_path: Option<String> = None;
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
                if let Some(value) = meta.get("resources_path") {
                    if let serde_json::Value::String(s) = value {
                        resources_path = Some(s.to_string())
                    }
                }
            }
        } else {
            println!("No packages found in Cargo.toml");
        } 

        //generate androidmanifest.xml
        let manifest = AndroidManifest::build(&metadata, app_name.as_ref().unwrap().to_string())?;
        //manifest path is cwd/target/android/AndroidManifest.xml
        let manifest_path: PathBuf = cwd.join("target").join(if release { "release" } else { "debug" }).join("android");
        println!("manifest path: {:?}", manifest_path);
        //write AndroidManifest.xml to file
        manifest.write_to(&manifest_path.as_path())?;
        Ok(AndroidBuilder{
            release: release, 
            target: target.to_string(), 
            cwd: cwd,
            build_path: None, 
            output_path: None, 
            icon_path: icon_path, 
            cargo_path: cargo_path, 
            app_name: app_name, 
            app_version: app_version, 
            manifest: manifest, 
            ndk_path: ndk_path.to_string(), 
            sdk_path: sdk_path.to_string(), 
            java_path: java_path.to_string(),
            resources: resources_path,
            build_tools_version: build_tools_version,
            bundletool_path: bundletool_path.to_string()
        })
    }

    fn pre_build(&mut self) -> Result <(), PistonError>{
        println!("pre build for android");
        println!("building the dynamic app bundle");
        let cwd: PathBuf = self.cwd.clone();
        println!("working dir: {:?}", cwd);
        let app_name = self.app_name.as_ref().unwrap();
        let release = if self.release {"release"} else {"debug"};
        let parent_build_path: PathBuf = format!("target/{}/androidbuilder/android",release).into();
        let child_build_path: PathBuf = parent_build_path.join("app").join("src").join("main").join("res");
        //set the absolute build path
        let full_build_path = cwd.join(&child_build_path);
        let working_build_path = cwd.join(&parent_build_path);
        println!("working build path: {:?}", working_build_path);
        println!("full build path with children {:?}", full_build_path);
        self.build_path = Some(working_build_path.clone());
        //check for a valid build path
        if self.build_path.as_ref().is_none() {
            return Err(PistonError::Generic("build path not provided".to_string()))
        }
        //Empty the directory if it already exists
        let path = full_build_path.as_path();
        if path.exists() && path.is_dir(){
            Helper::empty_directory(path)?
        }
        //create the target directories
        create_dir_all(&full_build_path).map_err(|e| PistonError::CreateDirAllError {
        path: full_build_path.clone(),
        source: e,
        })?;
        //set the output path
        let partial_output_path: PathBuf = format!("target/{}/android", release).into();
        let output_path = cwd.join(&partial_output_path);
        println!("output path: {:?}", output_path);
        self.output_path = Some(output_path.clone());
        //check for valid output path
        if self.output_path.as_ref().is_none() {
            return Err(PistonError::Generic("output path not provided".to_string()))
        }
        //empty dir if it exists
        let path = output_path.as_path();
        if path.exists() && path.is_dir(){
            Helper::empty_directory(path)?
        }
        //create target dirs
        create_dir_all(self.output_path.as_ref().unwrap()).map_err(|e| PistonError::CreateDirAllError {
        path: self.output_path.as_ref().unwrap().to_path_buf(),
        source: e,
        })?;
        //establish absolute paths for  mipmap dirs
        let hdpi_path: PathBuf = full_build_path.join("mipmap-hdpi");
        let mdpi_path: PathBuf = full_build_path.join("mipmap-mdpi");
        let xhdpi_path: PathBuf = full_build_path.join("mipmap-xhdpi");
        let xxhdpi_path: PathBuf = full_build_path.join("mipmap-xxhdpi");
        let xxxhdpi_path: PathBuf = full_build_path.join("mipmap-xxxhdpi");
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

        println!("done preconfiguring Android build path");
        Ok(())
    }

    fn build(&mut self) -> Result <(), PistonError>{
        println!("building for android");
        //build the android .so with cargo
        self.build_so()?;
        //compile the resources directory
        let resources = self.compile_resources()?;
        //TODO Link manifest and resources (aapt2 link)
        // let base_dir = self.build_path.join("base");
        // fs::create_dir_all(&base_dir)?;
        // self.link_manifest_and_resources(&compiled_res, &base_dir)?;

        //TODO add assets if any (copy to base/asssets)
        //         if let Some(assets) = &self.assets {
//             let assets_dest = base_dir.join("assets");
//             fs::create_dir_all(&assets_dest)?;
//             // Assume recursive copy; implement or use walkdir if needed
//             self.copy_dir_recursively(assets, &assets_dest)?;
//         }

        //TODO add the .so lib for single lib/no recursion
        //         self.add_lib(&base_dir, artifact_name, target_triple)?;


        //TODO zip base module
        //         let base_zip = self.build_dir.join("base.zip");
//         self.zip_base(&base_dir, &base_zip)?;

        //TODO build AAB with bundletool? Can we just use cargo commands?
        //         let aab_path = self.build_dir.join(format!("{}.aab", self.aab_name));
//         self.build_bundle(&base_zip, &aab_path)?;

        //TODO sign AAB
        // self.sign_aab(&aab_path)?;

        Ok(())
    }

    fn post_build(&mut self) -> Result <(), PistonError>{
        println!("post build for android");
        //TDOD sign the completed AAB
        Ok(())
    }

    fn build_so(&mut self) -> Result<(), PistonError>{
        //build the .so with cargo
        let host_platform = Helper::get_host_platform(self.ndk_path.as_ref())?;
        //set linker
        let api_level = self.manifest.min_sdk_version.to_string();  // Assume min_sdk_version in your struct; fallback to "21"
        // For linker name: for aarch64-linux-android, it's target_triple + api_level + "-clang"
        let linker_name = if self.target == "armv7-linux-androideabi" {
            format!("armv7a-linux-androideabi{}-clang", api_level)
        } else {
            format!("{}{}-clang", self.target, api_level)
        };
        let ndk_path_buf = PathBuf::from(&self.ndk_path);
        let linker_path = ndk_path_buf.join("toolchains/llvm/prebuilt").join(&host_platform).join("bin").join(linker_name);
        let ar_path = ndk_path_buf.join("toolchains/llvm/prebuilt").join(&host_platform).join("bin").join("llvm-ar");
        // Check if paths exists
        if !linker_path.exists() {
            return Err(PistonError::BuildError(format!("Linker not found at {}", linker_path.display())));
        }
        if !ar_path.exists() {
            return Err(PistonError::BuildError(format!("AR not found at {}", ar_path.display())));
        }
        let target_upper = self.target.to_uppercase().replace("-", "_");
        let linker_env_key = format!("CARGO_TARGET_{}_LINKER", target_upper);
        let ar_env_key = format!("CARGO_TARGET_{}_AR", target_upper);
        let release = if self.release {"--release"} else {""};
        let cargo_command = format!("cargo build --target {}  {}", self.target, release); 
        //run the cargo build command
        Command::new("bash")
            .arg("-c")
            .arg(&cargo_command)
            .current_dir(self.build_path.as_ref().unwrap())
            .env("JAVA_HOME", self.java_path.clone())
            .env("ANDROID_HOME", self.sdk_path.clone())
            .env("NDK_HOME", self.ndk_path.clone())
            .env(&linker_env_key, linker_path.to_str().unwrap())
            .env(&ar_env_key, ar_path.to_str().unwrap())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| PistonError::BuildError(format!("Cargo build failed: {}", e)))?;
        Ok(())
    }

    fn compile_resources(&self) -> Result<PathBuf, PistonError> {
        println!("compiling resources");
        if let Some(res_str) = &self.resources {
            let res = PathBuf::from(res_str);
            let build_dir_path = self.build_path.as_ref().unwrap();
            let compiled_res = build_dir_path.join("compiled_resources.zip");
            let sdk = PathBuf::from(&self.sdk_path);
            let aapt2_path = sdk.join(format!("build-tools/{}/aapt2", self.build_tools_version));

            let compile_command = format!(
                "{} compile --dir {} -o {}",
                aapt2_path.display(),
                res.display(),
                compiled_res.display()
            );

            Command::new("bash")
                .arg("-c")
                .arg(&compile_command)
                .current_dir(&build_dir_path)
                .env("ANDROID_HOME", &self.sdk_path)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output()
                .map_err(|e| PistonError::BuildError(format!("aapt2 compile failed: {}", e)))?;
            println!("done compiling resources");
            Ok(compiled_res)
        } else {
            println!("no resources found");
            Ok(PathBuf::new())  // Empty if no resources
        }
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