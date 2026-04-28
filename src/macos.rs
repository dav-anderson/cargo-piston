use std::path::{ Path, PathBuf };
use std::collections::HashMap;
use cargo_metadata::{ Metadata, MetadataCommand };
use std::fs::{ File, create_dir_all, copy, remove_file};
use std::io::Write;
use std::process::{ Command, Stdio };
use crate::Helper;
use crate::PistonError;
use crate::asc::{ AscApiKey, AscClient };

pub struct MacOSBuilder {
    release: bool,
    external: bool,
    target: String,
    cwd: PathBuf,
    output_path: Option<PathBuf>,
    icon_path: Option<String>,
    assets: String,
    cargo_path: String,
    app_name: String,
    bundle_id: String,
    app_version: String,
    asc_api_key: Option<AscApiKey>,
    keystore_path: Option<String>,
    external_cert: Option<String>,
}

impl MacOSBuilder {
    pub fn start(release: bool, external: bool, target: String, cwd: PathBuf, env_vars: HashMap<String, String>) -> Result<(), PistonError> {
        println!("building for MacOS");
        //check operating system (requires MacOS)
        if std::env::consts::OS != "macos"{
            return Err(PistonError::UnsupportedOSError{os: std::env::consts::OS.to_string(), target: target})
        }
        //set release to true if external is true
        let mut op = MacOSBuilder::new(release, external, target, cwd, env_vars)?;
        //TODO check for signing certificate & sign?
        //>>prebuild
        op.pre_build()?;

        //>>build
        op.build()?;

        //>>Postbuild
        op.post_build()?;

        Ok(())
    }

    fn new(release: bool, external: bool, target: String, cwd: PathBuf, env_vars: HashMap<String, String>) -> Result<Self, PistonError> {
        println!("creating MacOSBuilder: release: {:?}, target: {:?}, cwd: {:?}", release, target.to_string(), cwd);
        //parse env vars
        let cargo_path = env_vars.get("cargo_path").cloned().unwrap_or("cargo".to_string());
        let keystore_path = env_vars.get("keystore_path").cloned();
        let external_cert = env_vars.get("external_cert").cloned();
        //parse cargo.toml
        let metadata: Metadata = MetadataCommand::new()
            .current_dir(cwd.clone())
            .exec()
            .map_err(|e| PistonError::CargoParseError(e.to_string()))?;

        let icon_path = Helper::get_icon_path(&metadata);
        let assets = Helper::get_assets_path(&metadata);
        let app_name = Helper::get_app_name(&metadata)?;
        let app_version = Helper::get_app_version(&metadata)?;
        let bundle_id = Helper::get_bundle_id(&metadata, &app_name);

        let asc_api_key: Option<AscApiKey> = match AscApiKey::from_hm(&env_vars) {
            Ok(key) => Some(key),
            Err(e) => {
                println!("Failed to obtain AscApiKey, check .env configuration: {}", e);
                None
            }
        };
        Ok(MacOSBuilder{
            release: release, 
            external: external,
            target: target.to_string(), 
            cwd: cwd, 
            output_path: None, 
            icon_path: icon_path, 
            assets: assets,
            cargo_path: cargo_path, 
            app_name: app_name, 
            bundle_id: bundle_id,
            app_version: app_version, 
            asc_api_key: asc_api_key,
            keystore_path: keystore_path,
            external_cert: external_cert,
        })
    }

     fn pre_build(&mut self) -> Result <(), PistonError>{
        //TODO check xcode for updates
        //TODO allow user to specify a security cert for offline signing?
        println!("Pre build for macos");
        //check for xcode installation
        let xcode_app = "/Applications/Xcode.app";
        if !Path::new(xcode_app).exists() {
            return Err(PistonError::XcodeInstallError(format!("Xcode installation not found at {} Please download xcode from the apple app store at https://apps.apple.com/us/app/xcode/id497799835", xcode_app)))?;
        }
        //Check for xcode-select command line tools installation and pathing
        let xcode_select = Command::new("xcode-select")
            .arg("-p")
            .output()
            .map_err(|e| PistonError::XcodeSelectInstallError(format!("Failed to verify xcode tools installation: {}", e)));

        let expected_path = format!("{}/Contents/Developer", xcode_app);

        let path = String::from_utf8(xcode_select.unwrap().stdout)
            .unwrap()
            .trim()
            .to_string();
        //verify that xcode-select path matches the expected query
        if path == expected_path {
            println!("xcode-select path match")
        }else {
            return Err(PistonError::XcodeSelectPathingError(format!("Xcode-select path query of {} does not match the expected value of {}...set the path with 'sudo xcode-select -s /Applications/Xcode.app/Contents/Developer'",path, expected_path)))
        }   
        let cwd: PathBuf = self.cwd.clone();
        let capitalized = Helper::capitalize_first(&self.app_name.clone());
        let release = if self.release {"release"} else {"debug"};
        let true_bundle_path: PathBuf = if self.release {
            format!("target/{}/macos/{}.app",release, capitalized).into()
        }else {
            format!("target/{}/macos/{}.app",release, capitalized).into()
        };
        let contents_path: PathBuf = true_bundle_path.join("Contents");
        //establish ~/target/release/macos/Appname.app/Contents/Resources
        let res_path: PathBuf = contents_path.join("Resources");
        let assets_tgt = cwd.join(&res_path).join("assets");
        let macos_path = contents_path.join("MacOS");
        self.output_path = Some(cwd.join(&true_bundle_path));
        //Empty the directory if it already exists
        let path = res_path.as_path();
        //empty the dir if it exists
        Helper::empty_directory(path, &["assets"])?;
        //create the target directories
        create_dir_all(path).map_err(|e| PistonError::CreateDirAllError {
        path: self.output_path.as_ref().unwrap().to_path_buf(),
        source: e,
        })?;
        //create binary directories
        create_dir_all(macos_path).map_err(|e| PistonError::CreateDirAllError {
            path: self.output_path.as_ref().unwrap().to_path_buf(),
            source: e,
        })?;
        //sync assets
        let bind = &self.assets.clone();
        let assets_src = Path::new(&bind);
        Helper::sync_assets(assets_src, &assets_tgt)?;
        //establish app icon target path ~/macos/release/Appname.app/Contents/Resources/macos_icon.icns
        let icon_path: PathBuf = res_path.join("macos_icon.icns");
        //establish Info.plist path ~/macos/release/Appname.app/Contents/Info.plist
        let plist_path: PathBuf = contents_path.join("Info.plist");
        //if a plist file exists, first remove it.
        if plist_path.exists() {
            remove_file(&plist_path).map_err(|e| PistonError::RemoveFileError {
                path: plist_path.clone().to_path_buf(),
                source: e,
            })?;
        }
        //create a new Info.plist file
        let mut plist_file = File::create(&plist_path).map_err(|e| PistonError::CreateFileError {
                path: plist_path.clone().to_path_buf(),
                source: e,
            })?;
        //populate the Info.plist file
        let plist_content = format!(
            r#"
            <?xml version="1.0" encoding="UTF-8"?>
            <!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN"
            "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
            <plist version="1.0">
            <dict>
                <key>CFBundleName</key>
                <string>{}</string>
                <key>CFBundleExecutable</key>
                <string>{}</string>
                <key>CFBundleIdentifier</key>
                <string>{}</string>
                <key>CFBundleIconFile</key>
                <string>macos_icon</string>
                <key>CFBundleVersion</key>
                <string>{}</string>
                <key>CFBundlePackageType</key>
                <string>APPL</string>
            </dict>
            </plist>
            "#,
            &capitalized,
            &self.app_name.clone(),
            &self.bundle_id.clone(),
            &self.app_version.clone(),
        );
        plist_file.write_all(plist_content.as_bytes()).map_err(|e| PistonError::WriteFileError(e.to_string()))?;
        //if icon path was provided...convert
        if !self.icon_path.is_none(){
            println!("icon path provided, configuring icon");
            //convert the .png at icon_path to an .icns which resides in the app bundle
            let img_path_clone = self.icon_path.clone().unwrap();
            let img_path = Path::new(&img_path_clone);
            //Configure icon
            Command::new("sips")
                .args(["-s", "format", "icns", &img_path_clone, "--out", &icon_path.display().to_string()])
                .output()
                .map_err(|e| PistonError::MacOSIconError {
                    input_path: img_path.to_path_buf(),
                    output_path: icon_path,
                    source: e,
                })?;
        }
        println!("done configuring MacOS bundle");
        Ok(())
        
    }

    fn build(&mut self) -> Result<(), PistonError>{
        println!("build for macos");
        //build the binary for the specified target
        let cargo_args = format!("build --target {} {}", self.target, if self.release {"--release"} else {""});
        let cargo_cmd = format!("{} {}", self.cargo_path, cargo_args);
        let builder = Command::new("bash")
            .arg("-c")
            .arg(&cargo_cmd)
            .current_dir(self.cwd.clone())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| PistonError::BuildError(format!("Cargo build failed: {}", e)))?;
        if !builder.status.success() {
            return Err(PistonError::BuildError(format!("Cargo build failed: {}", String::from_utf8_lossy(&builder.stderr))))
        }
        //second target triple for universal binary build
        if self.release{
            let secondary = if self.target.contains("aarch64") {"x86_64-apple-darwin"} else {"aarch64-apple-darwin"};
            let cargo_args_second = format!("build --target {} {}", secondary, if self.release {"--release"} else {""});
            let cargo_cmd_second = format!("{} {}", self.cargo_path, cargo_args_second);
            let builder_second = Command::new("bash")
                .arg("-c")
                .arg(&cargo_cmd_second)
                .current_dir(self.cwd.clone())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output()
                .map_err(|e| PistonError::BuildError(format!("Second Cargo build failed: {}", e)))?;
            if !builder_second.status.success() {
                return Err(PistonError::BuildError(format!("Second Cargo build failed: {}", String::from_utf8_lossy(&builder.stderr))))
            }
        }

        Ok(())
    }

    fn post_build(&mut self) -> Result<(), PistonError>{
        println!("post build for macos");
        //binary_path: /Users/<user>/<appname>/target/<target-triple>/<release>/<appname>
        let binary_path = self.cwd.join("target").join(self.target.clone()).join(if self.release {"release"} else {"debug"}).join(self.app_name.clone());
        //binary_tgt_path: /Users/<user>/<appname>/target/<release>/macos/<Appname>.app/Contents/MacOS/<appname>
        let binary_target_path = self.output_path.as_ref().unwrap().join("Contents").join("MacOS").join(self.app_name.clone());
        //if release flag false, copy target triple only
        if !self.release {
            //bundle path should be cwd + target + <target output> + <--release flag or None for debug> + <appname>.exe
            println!("copying binary to app bundle");
            //move the target binary into the app bundle at the proper location
            copy(&binary_path, &binary_target_path).map_err(|e| PistonError::CopyFileError {
                input_path: binary_path.clone().to_path_buf(),
                output_path: binary_target_path.clone().to_path_buf(),
                source: e,
            })?;
            println!("MacOS app bundle available at: {}", &binary_target_path.display());
        //if release flag true, build universal binary
        } else {
            println!("creating universal binary in the app bundle");
            let secondary = if self.target.contains("aarch64") {"x86_64-apple-darwin"} else {"aarch64-apple-darwin"};
            let secondary_path = self.cwd.join("target").join(secondary).join("release").join(self.app_name.clone());
            //secondary_path: /Users/<user>/<appname>/target/<secondary-target-triple>/<release>/<appname>
            let lipo = Command::new("lipo")
                .arg("-create")
                .arg(&binary_path)
                .arg(&secondary_path)
                .arg("-output")
                .arg(&binary_target_path)
                .output()
                .map_err(|e| PistonError::LipoError{
                    first_binary: binary_path.clone(),
                    second_binary: secondary_path.clone(),
                    source: e.to_string()
                })?;
            if !lipo.status.success() {
                return Err(PistonError::LipoError{
                    first_binary: binary_path,
                    second_binary: secondary_path,
                    source: String::from_utf8_lossy(&lipo.stderr).to_string()         
                })
            }
            println!("Universal MacOS app bundle available at: {}", &binary_target_path.display());
        }

        //automated signing
        if self.keystore_path.is_none() || self.asc_api_key.is_none() {
            println!("Either the Keystore path or ASC API key missing from .env, skipping automated signing");
        } else if self.external {
            //if external-release not properly configured, throw error
            if self.external_cert.is_none() {
                return Err(PistonError::Generic("external-release certificate is not properly configured, see documentation".to_string()))
            }
            //perform external release sign if properly configured
            //TODO this is not currently completely implemented
            AscClient::sign_app_bundle(&self.app_name, &self.output_path.as_ref().unwrap(), &self.external_cert.as_ref().unwrap(), false)?;

            //TODO zip + notarytool bundle

        }else {
            println!("keystore path & ASC API key properly configured");
            let asc = AscClient{ api_key: self.asc_api_key.clone(), keystore_path: self.keystore_path.clone().unwrap()};
            //obtain certificate
            let security_cert = asc.create_or_find_security_cert()?;
            let security_profile = format!("{} {}", security_cert.1, security_cert.0);
            println!("your security profile is: {:?}", security_profile);
            let output_path = self.output_path.clone().unwrap();

            let app_name = self.app_name.clone();
            //sign the app bundle for distribution
            AscClient::sign_app_bundle(&app_name, &output_path, &security_profile, false)?;

        }
        Ok(())
    }
}

pub struct MacOSRunner{
release: bool,
cwd: PathBuf,
cargo_path: String,
}

impl MacOSRunner {

    pub fn start(release: bool, cwd: PathBuf, env_vars: HashMap<String, String>) -> Result<(), PistonError> {
        println!("Initializing runner for MacOS");
        let mut op = MacOSRunner::new(release, cwd, env_vars)?;

        op.run()?;

        Ok(())
    }
    fn new(release: bool, cwd: PathBuf, env_vars: HashMap<String, String>) -> Result<Self, PistonError> {
        println!("Creating MacOS Runner: release flag: {:?}, cwd: {:?}", release, cwd);
        //parse env vars
        let cargo_path = env_vars.get("cargo_path").cloned().unwrap_or("cargo".to_string());

        Ok(MacOSRunner{release: release, cwd: cwd, cargo_path: cargo_path})
        
    }

    fn run(&mut self) -> Result<(), PistonError> {
        println!("Running for MacOS");
        //Run the binary for MacOS
        let cargo_args = format!("run {}", if self.release {"--release"} else {""});
        let cargo_cmd = format!("{} {}", self.cargo_path, cargo_args);
        Command::new("bash")
            .arg("-c")
            .arg(&cargo_cmd)
            .current_dir(self.cwd.clone())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| PistonError::BuildError(format!("Cargo Run failed: {}", e)))?;
        Ok(())
    }
}