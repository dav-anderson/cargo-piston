use std::path::{ Path, PathBuf };
use std::collections::HashMap;
use cargo_metadata::{ Metadata, MetadataCommand };
use std::fs::{ File, create_dir_all, copy, rename, remove_file};
use std::io::{ Write, BufWriter };
use std::process::{ Command, Stdio };
use serde::Deserialize;
use serde_json::Value;
use crate::Helper;
use crate::PistonError;

//TODO build out intent filters with more robust cargo.toml parameters  
//TODO reverse engineer cargo-apk

//TODO derive APK from aab


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
//             .current_dir(&self.build_path)
//             .stdout(Stdio::inherit())
//             .stderr(Stdio::inherit())
//             .output()
//             .map_err(|e| PistonError::BuildError(format!("Signing failed: {}", e)))?;

//         Ok(())
//     }

//     fn generate_debug_key(&self, key_id: &Path) -> Result<(), PistonError> {
//         let keytool_command = format!(
//             "keytool -genkeypair -v -keystore {} -alias androiddebugkey -keyalg RSA -keysize 2048 -validity 10000 -dname \"CN=Android Debug,O=Android,C=US\" -storepass android -keypass android",
//             key_id.display()
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
    pub fn build(metadata: &Metadata, app_name: &String) -> Result<Self, PistonError> {
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
        manifest.app_name = app_name.to_string();
        manifest.icon = "@mipmap/ic_launcher".to_string();

        Ok(manifest)
    }

    pub fn to_xml(&self) -> String {
        let icon_attr = format!(r#" android:icon="{}""#, Self::escape_xml(&self.icon));

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
        // let file_path = dir.join("AndroidManifest.xml");
        println!("writing {:?}...", dir);
        //empty dir if exists
        // Helper::empty_directory(&dir)?;
        // create_dir_all(&dir).map_err(|e| PistonError::CreateDirAllError {
        // path: dir.to_path_buf(),
        // source: e,
        // })?;
        let file = File::create(&dir)
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
    build_path: PathBuf,
    output_path: Option<PathBuf>,
    icon_path: Option<String>,
    cargo_path: String,
    gpg_path: Option<String>,
    app_name: String,
    lib_name: String,
    app_version: String,
    manifest: AndroidManifest,
    manifest_path: PathBuf,
    ndk_path: String,
    sdk_path: String,
    java_path: String,
    resources: PathBuf,
    build_tools_version: String,
    bundletool_path: String,
    key_id: Option<String>,
    key_pass: Option<String>,
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
        let gpg_path: Option<String> = env_vars.get("gpg_path").cloned();
        let ndk_path: &String = Helper::get_or_err(&env_vars, "ndk_path")?;
        let sdk_path: &String = Helper::get_or_err(&env_vars, "sdk_path")?;
        let java_path: &String = Helper::get_or_err(&env_vars, "java_path")?;
        let bundletool_path: &String = Helper::get_or_err(&env_vars, "bundletool_path")?;
        let build_tools_version: String = Helper::get_build_tools_version(&sdk_path)?;
        let key_id: Option<String> = env_vars.get("android_gpg_key_id").cloned();
        let key_pass: Option<String> = env_vars.get("android_gpg_key_pass").cloned();
        println!("Cargo path determined: {}", &cargo_path);
        //parse cargo.toml
        let metadata: Metadata = MetadataCommand::new()
            .current_dir(cwd.clone())
            .exec()
            .map_err(|e| PistonError::CargoParseError(e.to_string()))?;

        let lib_name = Helper::get_lib_name(&metadata)?;
        let icon_path = Helper::get_icon_path(&metadata);
        let app_name = Helper::get_app_name(&metadata)?;
        let app_version = Helper::get_app_version(&metadata)?;
        //generate androidmanifest.xml
        let manifest = AndroidManifest::build(&metadata, &app_name)?;
        let build_path: PathBuf = cwd.join("target").join(if release {"release"} else {"debug"}).join("androidbuilder").join("android");
        //manifest path is cwd/target/<release>/androidbuilder/android/manifest
        let manifest_path: PathBuf = build_path.join("AndroidManifest.xml");
        let resources_path: PathBuf = build_path.join("app").join("src").join("main").join("res");
        println!("build path: {:?}", build_path);
        //write AndroidManifest.xml to file
        manifest.write_to(&manifest_path.as_path())?;
        Ok(AndroidBuilder{
            release: release, 
            target: target.to_string(), 
            cwd: cwd,
            build_path: build_path, 
            output_path: None, 
            icon_path: icon_path, 
            cargo_path: cargo_path,
            gpg_path: gpg_path,
            app_name: app_name, 
            lib_name: lib_name,
            app_version: app_version, 
            manifest: manifest, 
            manifest_path: manifest_path,
            ndk_path: ndk_path.to_string(), 
            sdk_path: sdk_path.to_string(), 
            java_path: java_path.to_string(),
            resources: resources_path,
            build_tools_version: build_tools_version,
            bundletool_path: bundletool_path.to_string(),
            key_id: key_id,
            key_pass: key_pass,
        })
    }

    fn pre_build(&mut self) -> Result <(), PistonError>{
        println!("pre build for android");
        println!("building the dynamic app bundle");
        let cwd: PathBuf = self.cwd.clone();
        println!("working dir: {:?}", cwd);
        let release = if self.release {"release"} else {"debug"};
        //set the absolute build path
        println!("working build path: {:?}", self.build_path);
        println!("full build path with children {:?}", self.resources);
        let path = self.resources.as_path();
        //Empty the directory if it already exists
        Helper::empty_directory(path)?;
        //create the target directories
        create_dir_all(&self.resources).map_err(|e| PistonError::CreateDirAllError {
        path: self.resources.clone(),
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
        let path = output_path.as_path();
        //empty dir if it exists
        Helper::empty_directory(path)?;
        //create target dirs
        create_dir_all(self.output_path.as_ref().unwrap()).map_err(|e| PistonError::CreateDirAllError {
        path: self.output_path.as_ref().unwrap().to_path_buf(),
        source: e,
        })?;
        //establish absolute paths for  mipmap dirs
        let hdpi_path: PathBuf = self.resources.join("mipmap-hdpi");
        let mdpi_path: PathBuf = self.resources.join("mipmap-mdpi");
        let xhdpi_path: PathBuf = self.resources.join("mipmap-xhdpi");
        let xxhdpi_path: PathBuf = self.resources.join("mipmap-xxhdpi");
        let xxxhdpi_path: PathBuf = self.resources.join("mipmap-xxxhdpi");
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
        //Link manifest and resources (aapt2 link)
        let base_dir = self.build_path.join("base");
        //empty the dir if it exists
        Helper::empty_directory(&base_dir)?;
        create_dir_all(&base_dir).map_err(|e| PistonError::CreateDirAllError {
        path: base_dir.clone(),
        source: e,
        })?;
        self.link_manifest_and_resources(&resources, &base_dir)?;

        //TODO add assets if any (copy to base/asssets)
        // if let Some(assets) = &self.assets {
//             let assets_dest = base_dir.join("assets");
//             fs::create_dir_all(&assets_dest)?;
//             // Assume recursive copy; implement or use walkdir if needed
//             self.copy_dir_recursively(assets, &assets_dest)?;
//         }

        //add the .so lib for a single lib
        self.add_lib(&base_dir, self.target.as_ref())?;
        //zip base module
        let base_zip = self.build_path.join("base.zip");
        self.zip_base(&base_dir, &base_zip)?;
        //build AAB with bundletool
        let aab_path = self.build_path.join(format!("{}.aab", self.app_name));
        self.build_bundle(&base_zip, &aab_path)?;

        println!("Success in building Android App Bundle. Bundle is available at: {:?}", aab_path);

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
        println!("building the .so library");
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
        let cargo_command = format!("cargo build --target {}  {} --lib", self.target, release); 
        //run the cargo build command
        Command::new("bash")
            .arg("-c")
            .arg(&cargo_command)
            .current_dir(self.build_path.clone())
            .env("JAVA_HOME", self.java_path.clone())
            .env("ANDROID_HOME", self.sdk_path.clone())
            .env("NDK_HOME", self.ndk_path.clone())
            .env(&linker_env_key, linker_path.to_str().unwrap())
            .env(&ar_env_key, ar_path.to_str().unwrap())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| PistonError::BuildError(format!("Cargo build failed: {}", e)))?;
        println!("finished building .so library");
        Ok(())
    }

    fn compile_resources(&self) -> Result<PathBuf, PistonError> {
        println!("compiling resources at {:?}", &self.resources);
        //remove compiled_resources.zip if it exists
        let compiled_res = self.build_path.join("compiled_resources.zip");
        if compiled_res.exists() {
            remove_file(&compiled_res).map_err(|e| PistonError::RemoveFileError {
                path: compiled_res.clone().to_path_buf(),
                source: e,
            })?;
            println!("removed compiled_resources.zip at: {:?}", compiled_res);
        }
        let sdk = PathBuf::from(&self.sdk_path);
        let aapt2_path = sdk.join(format!("build-tools/{}/aapt2", self.build_tools_version));

        let compile_command = format!(
            "{} compile --dir {} -o {}",
            aapt2_path.display(),
            self.resources.display(),
            compiled_res.display()
        );

        Command::new("bash")
            .arg("-c")
            .arg(&compile_command)
            .current_dir(self.build_path.clone())
            .env("ANDROID_HOME", self.sdk_path.clone())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| PistonError::BuildError(format!("aapt2 compile failed: {}", e)))?;
        println!("done compiling resources");
        Ok(compiled_res)
    }

    fn link_manifest_and_resources(&self, compiled_res: &Path, base_dir: &Path) -> Result<(), PistonError> {
        let aapt2_path: PathBuf = PathBuf::from(self.sdk_path.clone()).join(format!("build-tools/{}/aapt2", self.build_tools_version));
        let android_jar: PathBuf = PathBuf::from(self.sdk_path.clone()).join(format!("platforms/android-{}/android.jar", self.manifest.target_sdk_version));
        
        let res_arg = if compiled_res.exists() { format!(" {}", compiled_res.display()) } else { String::new() };
        println!("linking manifest & resources");
        let link_command = format!(
            "{} link --proto-format --output-to-dir -o {} --manifest {} -I {} {}",
            aapt2_path.display(),
            base_dir.display(),
            self.manifest_path.display(),
            android_jar.display(),
            res_arg
        );

        println!("link command: {}", link_command);

        Command::new("bash")
            .arg("-c")
            .arg(&link_command)
            .current_dir(&self.build_path)
            .env("ANDROID_HOME", self.sdk_path.clone())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| PistonError::ProtoLinkError(format!("aapt2 link failed: {}", e)))?;

        //TODO THIS IS BUSTED AND PROBABLY UNNCESSARY FIX HIGHER UP LOGIC FLOW Ensure proto manifest is in a manifest/ subdir
        let proto_manifest_root = base_dir.join("AndroidManifest.xml");
        if proto_manifest_root.exists() {
            let manifest_dir = base_dir.join("manifest");
            //empty the dir if it exists
            Helper::empty_directory(&manifest_dir)?;
            create_dir_all(&manifest_dir)
                .map_err(|e| PistonError::CreateDirAllError {
                    path: manifest_dir.clone(),
                    source: e,
                })?;
            rename(&proto_manifest_root, manifest_dir.join("AndroidManifest.xml"))
                .map_err(|e| PistonError::RenameFileError {
                    path: proto_manifest_root.clone(),
                    source: e,
                })?;
        } else {
            return Err(PistonError::ProtoLinkError("Proto AndroidManifest.xml not generated and linked by AAPT2".to_string()))
        }
        
        println!("done linking manifest and resources");

        Ok(())
    }

    fn add_lib(&self, base_dir: &Path, target: &str) -> Result<(), PistonError> {
        println!("adding .so library to base directory");
        let abi = match target {
            "aarch64-linux-android" => "arm64-v8a",
            //Add more mappings here as required if updating android support for other outputs
            _ => return Err(PistonError::UnsupportedTargetError(format!("Unsupported target {}", target).to_string())),
        };
        let lib_dir = base_dir.join("lib").join(abi);
        Helper::empty_directory(&lib_dir)?;
        create_dir_all(&lib_dir).map_err(|e| PistonError::CreateDirAllError {
        path: lib_dir.clone(),
        source: e,
        })?;

        let lib_file = format!("lib{}.so", self.lib_name);
        println!("library file name: {}", lib_file);

        let so_path = self.cwd.join("target").join(target).join(if self.release { "release" } else { "debug" }).join(&lib_file);
        println!(".so path: {:?}", so_path);
        copy(&so_path, lib_dir.join(&lib_file))
            .map_err(|e| PistonError::BuildError(format!("Failed to copy .so: {}", e)))?;

        Ok(())
    }

    fn zip_base(&self, base_dir: &Path, base_zip: &Path) -> Result<(), PistonError> {
        let zip_path = self.build_path.join("base.zip");
        if zip_path.exists() {
            println!("removing stale zip");
            remove_file(&zip_path).map_err(|e| PistonError::RemoveFileError {
                path: zip_path.clone().to_path_buf(),
                source: e,
            })?;
            println!("removed old zip from: {:?}", zip_path);
        }
        println!("zipping base dir");
        let zip_command = format!(
            "cd {} && zip -r ../base.zip *",
            base_dir.display()
        );

        Command::new("bash")
            .arg("-c")
            .arg(&zip_command)
            .current_dir(&self.build_path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| PistonError::BuildError(format!("Zip failed: {}", e)))?;

        Ok(())
    }

    fn build_bundle(&self, base_zip: &Path, aab_path: &Path) -> Result<(), PistonError> {
        println!("building .aab bundle with bundletool");
        if aab_path.exists() {
            remove_file(&aab_path).map_err(|e| PistonError::RemoveFileError {
                path: aab_path.to_path_buf(),
                source: e,
            })?;
            println!("removed .aab at: {:?}", aab_path);
        }
        let bundle_command = format!(
            "java -jar {} build-bundle --modules={} --output={}",
            self.bundletool_path,
            base_zip.display(),
            aab_path.display()
        );

        Command::new("bash")
            .arg("-c")
            .arg(&bundle_command)
            .current_dir(&self.build_path)
            .env("JAVA_HOME", self.java_path.clone())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| PistonError::BuildError(format!("bundletool failed: {}", e)))?;
        println!("finished .aab bulding bundle with bundletool");

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

        //TODO run androidbuilder
        //TODO extract .apk from completed aab provided by androidbuilder
        //TODO stream install the extracted .apk to the target device
    }
}