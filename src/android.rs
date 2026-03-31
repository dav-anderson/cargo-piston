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
use crate::devices::AndroidDevice;

//TODO build out intent filters with more robust cargo.toml parameters  

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
    pub fn build(metadata: &Metadata, app_name: &String, version: &String) -> Result<Self, PistonError> {
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
        manifest.version_name = android_meta.version_name.unwrap_or(version.to_string());
        manifest.min_sdk_version = android_meta.min_sdk_version
            .unwrap_or(21);
        manifest.target_sdk_version = android_meta.target_sdk_version
            .unwrap_or(34);
        manifest.app_label = android_meta.label
            .unwrap_or(format!("{}", crate_name));
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
    // cargo_path: String,
    key_path: String,
    key_pass: String,
    key_alias: String,
    app_name: String,
    lib_name: String,
    manifest: AndroidManifest,
    manifest_path: PathBuf,
    ndk_path: String,
    sdk_path: String,
    java_path: String,
    resources: PathBuf,
    build_tools_version: String,
    bundletool_path: String,
    device_target: Option<AndroidDevice>,
    // assets: Option<PathBuf>,
}

impl AndroidBuilder {
    pub fn start(release: bool, target: String, cwd: PathBuf, env_vars: HashMap<String, String>, device_target: Option<AndroidDevice>) -> Result<(PathBuf, String, String), PistonError>{
        println!("building for android");
        let mut op = AndroidBuilder::new(release, target, cwd, env_vars, device_target)?;

        //>>prebuild
        op.pre_build()?;

        //>>build
        let aab_path = op.build()?;

        //>>Postbuild
        op.post_build(aab_path)?;

        //return bundle output path and app name
        Ok((op.output_path.unwrap(), op.app_name, op.manifest.package))
    }

    fn new(release: bool, target: String, cwd: PathBuf, env_vars: HashMap<String, String>, device_target: Option<AndroidDevice>) -> Result<Self, PistonError> {
        println!("creating AndroidBuilder: release: {:?}, target: {:?}, cwd: {:?}", release, target.to_string(), cwd);
        //parse env vars
        // let cargo_path: String = env_vars.get("cargo_path").cloned().unwrap_or("cargo".to_string());
        let ndk_path: &String = Helper::get_or_err(&env_vars, "ndk_path")?;
        let sdk_path: &String = Helper::get_or_err(&env_vars, "sdk_path")?;
        let java_path: &String = Helper::get_or_err(&env_vars, "java_path")?;
        let bundletool_path: &String = Helper::get_or_err(&env_vars, "bundletool_path")?;
        let build_tools_version: String = Helper::get_build_tools_version(&sdk_path)?;
        //obtain default path for keystore
        let user_output = Command::new("whoami")
            .output()
            .map_err(|e| PistonError::WhoAmIError(format!("Failed to run 'whoami': {}", e)))?;

        if !user_output.status.success() {
            return Err(PistonError::WhoAmIError(format!("Failed to run 'whoami': {}", String::from_utf8_lossy(&user_output.stderr))))
        }
        let user = String::from_utf8_lossy(&user_output.stdout).trim().to_string();
        if user.is_empty() {
            return Err(PistonError::WhoAmIError(format!("Failed to obtain user id with whoami: {}", String::from_utf8_lossy(&user_output.stderr))))
        }
        let default_path = format!("/Users/{}/.android/release.keystore", user);
        //allow .env to override default key_path and key_pass and key_alias if it exists
        let key_path: String = env_vars.get("aab_release_key").cloned().unwrap_or(default_path);
        let key_pass: String = env_vars.get("aab_key_pass").cloned().unwrap_or("piston".to_string());
        let key_alias: String = env_vars.get("aab_key_alias").cloned().unwrap_or("release-key".to_string());
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
        let manifest = AndroidManifest::build(&metadata, &app_name, &app_version)?;
        let build_path: PathBuf = cwd.join("target").join(if release {"release"} else {"debug"}).join("android").join("androidbuilder");
        println!("build path: {:?}", build_path);
        //empty dirs all build_path
        Helper::empty_directory(build_path.as_path())?;
        //mkdir all build_path
        create_dir_all(&build_path).map_err(|e| PistonError::CreateDirAllError {
        path: build_path.clone(),
        source: e,
        })?;
        //manifest path is cwd/target/<release>/androidbuilder/android/manifest
        let manifest_path: PathBuf = build_path.join("AndroidManifest.xml");
        println!("manifest path: {}", manifest_path.display());
        let resources_path: PathBuf = build_path.join("app").join("src").join("main").join("res");
        //write AndroidManifest.xml to file
        manifest.write_to(&manifest_path.as_path())?;
        Ok(AndroidBuilder{
            release: release, 
            target: target.to_string(), 
            cwd: cwd,
            build_path: build_path, 
            output_path: None, 
            icon_path: icon_path, 
            // cargo_path: cargo_path,
            key_path: key_path,
            key_pass: key_pass,
            key_alias: key_alias,
            app_name: app_name, 
            lib_name: lib_name,
            manifest: manifest, 
            manifest_path: manifest_path,
            ndk_path: ndk_path.to_string(), 
            sdk_path: sdk_path.to_string(), 
            java_path: java_path.to_string(),
            resources: resources_path,
            build_tools_version: build_tools_version,
            bundletool_path: bundletool_path.to_string(),
            device_target: device_target,
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

    fn build(&mut self) -> Result <PathBuf, PistonError>{
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
        self.zip_base(&base_dir)?;
        //build AAB with bundletool
        let output_bind = self.output_path.clone().unwrap();
        let aab_path = output_bind.join(format!("{}.aab", self.app_name));
        self.build_bundle(&base_zip, &aab_path)?;

        println!("Success in building Android App Bundle. Bundle is available at: {:?}", aab_path);

        Ok(aab_path)
    }

    fn post_build(&mut self, aab_path: PathBuf) -> Result <(), PistonError>{
        println!("post build for android");
        // let bind = self.key_path.clone();
        //create a release key if none specified in .env and release flag is true
        let key_path_exists = Path::new(&self.key_path).to_path_buf().exists();
        let key_alias_exists = self.verify_key_alias()?;
        if self.release && (!key_path_exists || !key_alias_exists){
            //create a release key
            self.create_release_key()?;
        }else {
            println!("release key found at: {}", self.key_path);
        }
        //sign the completed AAB with release key if release flag is true
        if self.release{
            //sign the bundle
            self.sign_aab(aab_path)?;
        }
        //TODO if a device target is provided, check if the target device is provisioned
        if !self.device_target.is_none() {
            println!("");
            //NOTE: this feature will be implemented when Android adds requirements for provisioning 
        }
        Ok(())
    }

    fn build_so(&mut self) -> Result<(), PistonError>{
        println!("building the .so library");
        //build the .so with cargo
        let host_platform = Helper::get_host_platform(self.ndk_path.as_ref())?;
        //set linker
        let api_level = self.manifest.min_sdk_version.to_string();
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
        let builder = Command::new("bash")
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
        if !builder.status.success() {
            return Err(PistonError::BuildError(format!("Cargo build failed: {}", String::from_utf8_lossy(&builder.stderr))))
        }
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

    fn zip_base(&self, base_dir: &Path) -> Result<(), PistonError> {
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

    fn create_release_key(&self) -> Result<(), PistonError>{
        //proceed to key creation, state the reason for the user
        println!("creating release key at path: {} with the alias: {}", self.key_path, self.key_alias);
        //check if .android exists, if not create
        if let Some(parent) = Path::new(&self.key_path).parent() {
            create_dir_all(parent)
                .map_err(|e| PistonError::CreateDirAllError {
                    path: Path::new(&self.key_path).to_path_buf(),
                    source: e,
                })?;
        }

        //create release key with keytool
        let output = Command::new("keytool")
            .arg("-genkeypair")
            .arg("-v")
            .arg("-keystore").arg(self.key_path.clone())
            .arg("-storepass").arg(self.key_pass.clone())
            .arg("-keypass").arg(self.key_pass.clone())
            .arg("-alias").arg(self.key_alias.clone())
            .arg("-keyalg").arg("RSA")
            .arg("-keysize").arg("2048")
            .arg("-validity").arg("10000")
            .arg("-dname").arg("CN=Unknown, OU=Development, O=Unknown, L=Unknown, S=Unknown, C=US")
            .output()
            .map_err(|e| PistonError::KeyToolError(format!("Failed to generate release key with keytool: {}", e)))?;

        //TODO implement dynamic -dname params and update docs
        if !output.status.success() {
            return Err(PistonError::KeyToolError(format!("Failed to generate release key: {}", String::from_utf8_lossy(&output.stderr))))
        }

        println!("Release key successfully created at: {}", self.key_path);
        Ok(())
    }

    fn verify_key_alias(&self) -> Result<bool, PistonError> {
        //verify the key alias on record exists by querying the keystore
        let output = Command::new("keytool")
            .arg("-list")
            .arg("-v")
            .arg("-keystore").arg(self.key_path.clone())
            .arg("-storepass").arg(self.key_pass.clone())
            .output()
            .map_err(|e| PistonError::KeyToolError(format!("Failed to list keystore contents: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PistonError::KeyToolError(format!("Could not use keytool to list keystore contents: {}", stderr)))
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        //search for the alias on record in the keystore
        for line in stdout.lines() {
            if let Some(found) = line.strip_prefix("Alias name:") {
                if found.trim() == self.key_alias {
                    return Ok(true)
                } else {
                    return Ok(false)
                }
            }
        }
        Ok(false)

    }

    fn sign_aab(&self, aab_path: PathBuf) -> Result<(), PistonError> {
        //sign the AAB with key_path, key_pass, and key_alias on record
        let sdk = PathBuf::from(&self.sdk_path);
        let apksigner_path = sdk.join(format!("build-tools/{}/apksigner", self.build_tools_version));
        let api_level = self.manifest.min_sdk_version.to_string();

        let output = Command::new(&apksigner_path)
            .arg("sign")
            .arg("--ks").arg(self.key_path.clone())
            .arg("--ks-key-alias").arg(self.key_alias.clone())
            .arg("--ks-pass").arg(format!("pass:{}", self.key_pass.clone()))
            .arg("--key-pass").arg(format!("pass:{}", self.key_pass.clone()))
            .arg("--min-sdk-version").arg(&api_level)
            .arg(&aab_path)
            .output()
            .map_err(|e| PistonError::APKSignerError(format!("Error signing AAB: {}", e)))?;
        
        if !output.status.success() {
            return Err(PistonError::APKSignerError(format!("Error signing AAB: {}", String::from_utf8_lossy(&output.stderr))))
        }

        let output = Command::new("keytool")
            .arg("-printcert")
            .arg("-jarfile")
            .arg(&aab_path)
            .output()
            .map_err(|e| PistonError::Generic(format!("Error verifying signature: {}", e)))?;
        if !output.status.success() {
            return Err(PistonError::Generic(format!("Error verifying signature: {}", String::from_utf8_lossy(&output.stderr))))
        }
        println!("Signature verifcation: {:?}", output);
        println!("AAB: {} successfully signed for release", aab_path.display());
        Ok(())
    }

}

pub struct AndroidRunner{}

impl AndroidRunner{

    pub fn start(release: bool, cwd: PathBuf, env_vars: HashMap<String, String>, device: &AndroidDevice) -> Result<(), PistonError> {
        println!("Running for Android");
        let target_string = "aarch64-linux-android".to_string();
        let env_vars_bind = env_vars.clone();
        //build the app bundle
        let builder = AndroidBuilder::start(release, target_string, cwd.clone(), env_vars, Some(device.clone()))?;

        //deploy the app bundle to the target device
        AndroidRunner::deploy_usb(device.id.as_ref(), builder.0, builder.1, cwd.clone(), builder.2, env_vars_bind)?;

        Ok(())
    }

    fn deploy_usb(device_id: &str, output_path: PathBuf, app_name: String, cwd: PathBuf, package: String, env_vars: HashMap<String, String>) -> Result<(), PistonError> {
        println!("Deploying bundle at: {} to device: {}", output_path.display(), device_id);
        let aab_path = output_path.join(format!("{}.aab", app_name));
        let bundletool_path: &String = Helper::get_or_err(&env_vars, "bundletool_path")?;
        let java_path: &String = Helper::get_or_err(&env_vars, "java_path")?;
        let sdk_path: &String = Helper::get_or_err(&env_vars, "sdk_path")?;
        let adb_path: String = format!("{}/platform-tools/adb", sdk_path);
        let apk_path = cwd.join(format!("{}.apks", app_name));
        //extract .apk from completed aab provided by androidbuilder
         let bundle_cmd = format!(
            "java -jar {} build-apks --bundle={} --output={} --connected-device --overwrite --adb {}",
            &bundletool_path,
            &aab_path.display(),
            &apk_path.display(),
            &adb_path

        );
        println!("bundletool command: {}", bundle_cmd);

        let output = Command::new("bash")
            .arg("-c")
            .arg(&bundle_cmd)
            .env("JAVA_HOME", &java_path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| PistonError::ExtractAPKError(format!("Bundletool failed to extract the APK: {}", e)))?;
        if !output.status.success() {
            return Err(PistonError::ExtractAPKError(format!("Bundletool failed to extract APK: {}", String::from_utf8_lossy(&output.stderr))))
        }
        //stream install the extracted .apk to the target device
        let bundle_cmd = format!(
            "java -jar {} install-apks --apks={} --device-id={} --adb {}",
            &bundletool_path,
            &apk_path.display(),
            device_id,
            &adb_path,
        );

        let output = Command::new("bash")
            .arg("-c")
            .arg(&bundle_cmd)
            .env("JAVA_HOME", &java_path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| PistonError::InstallAPKError(format!("Bundletool failed to install the APK: {}", e)))?;
        if !output.status.success() {
            return Err(PistonError::InstallAPKError(format!("Bundletool failed to install APK: {}", String::from_utf8_lossy(&output.stderr))))
        }
        //run the app
        let launch = format!("{}/android.app.NativeActivity", package);
        let adb_cmd = format!(
            "{} shell am start -n {}",
            &adb_path,
            &launch,
        );

        let output = Command::new("bash")
            .arg("-c")
            .arg(&adb_cmd)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| PistonError::RunAPKError(format!("ADB failed to run the APK: {}", e)))?;
        if !output.status.success() {
            return Err(PistonError::RunAPKError(format!("ADB failed to run APK: {}", String::from_utf8_lossy(&output.stderr))))
        }

        Ok(())
    }
}