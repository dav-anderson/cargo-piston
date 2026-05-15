use std::path::{ PathBuf, Path };
use std::collections::HashMap;
use std::process::{ Command, Stdio };
use std::io::{ Write };
use cargo_metadata::{ Metadata, MetadataCommand };
use std::fs::{ copy, File, create_dir_all, remove_file };
use std::fs;
use crate::Helper;
use crate::PistonError;
use crate::devices::IOSDevice;
use crate::asc::{ AscApiKey, AscClient };


pub struct IOSBuilder {
    release: bool,
    target: String,
    cwd: PathBuf,
    output_path: Option<PathBuf>,
    ipa_path: Option<PathBuf>,
    icon_path: Option<String>,
    _assets: String,
    cargo_path: String,
    app_name: String,
    app_version: String,
    bundle_id: String,
    min_os_version: f32,
    asc_api_key: Option<AscApiKey>,
    device_target: Option<IOSDevice>,
    idp_path: Option<String>,
    keystore_path: Option<String>,
    team_id: Option<String>,
}

impl IOSBuilder {

    pub fn start(release: bool, target: String, cwd: PathBuf, env_vars: HashMap<String, String>, device_target: Option<IOSDevice>) 
    -> Result<(PathBuf, String), PistonError> {
        println!("building for iOS");
        //check operating system (requires MacOS)
        if std::env::consts::OS != "macos"{
            return Err(PistonError::UnsupportedOSError{os: std::env::consts::OS.to_string(), target: target})
        }

        let mut op = IOSBuilder::new(release, target, cwd, env_vars, device_target)?;
        //>>prebuild
        op.pre_build()?;

        //>>build
        op.build()?;

        //>>Postbuild
        op.post_build()?;

        //return bundle output path and bundle id from cargo.toml & plist
        Ok((op.ipa_path.unwrap(), op.bundle_id))
    }

    fn new(release: bool, target: String, cwd: PathBuf, env_vars: HashMap<String, String>, device_target: Option<IOSDevice>) -> Result<Self, PistonError> {
        println!("creating IOSBuilder: release: {:?}, target: {:?}, cwd: {:?}", release, target.to_string(), cwd);
        //parse env vars
        let cargo_path = env_vars.get("cargo_path").cloned().unwrap_or("cargo".to_string());
        let idp_path = env_vars.get("idp_path").cloned();
        let keystore_path = env_vars.get("keystore_path").cloned();
        let team_id = env_vars.get("team_id").cloned();
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
        let min_os_version = Helper::get_min_os(&metadata);

        let asc_api_key: Option<AscApiKey> = match AscApiKey::from_hm(&env_vars) {
            Ok(key) => Some(key),
            Err(e) => {
                println!("Failed to obtain AscApiKey, check .env configuration: {}", e);
                None
            }
        };
        Ok(IOSBuilder{
            release: release, 
            target: target.to_string(), 
            cwd: cwd, 
            output_path: None, 
            ipa_path: None,
            icon_path: icon_path,
            _assets: assets,
            cargo_path: cargo_path,
            app_name: app_name,
            app_version: app_version, 
            bundle_id: bundle_id, 
            min_os_version: min_os_version, 
            asc_api_key: asc_api_key,
            device_target: device_target, 
            idp_path: idp_path,
            keystore_path: keystore_path,
            team_id: team_id,
        })
    }

    fn pre_build(&mut self) -> Result <(), PistonError>{
        //TODO check xcode for updates?
        //TODO allow user to specify a security cert for offline signing?
        println!("Pre build for ios");
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
        //check for xcode ios sdk
        let sdk_output = Command::new("xcodebuild")
            .arg("-showsdks")
            .output()
            .map_err(|e| PistonError::XcodeBuildError(format!("Failed to run xcodebuild -showsdks. Something is likely missing from your installation: {}", e)));

        let sdk_binding = sdk_output.unwrap();
        let sdks = String::from_utf8_lossy(&sdk_binding.stdout);
        if !sdks.contains("iOS") {
            return Err(PistonError::IOSSdkMissingError("IOS sdk is missing. Try running 'xcodebuild -downloadPlatform iOS'".to_string()))
        }
        //build the app bundle
        let cwd: PathBuf = self.cwd.clone();
        let capitalized = Helper::capitalize_first(&self.app_name.clone());
        let release = if self.release {"release"} else {"debug"};
        //fix the path to match ios convention
        let partial_path: PathBuf = if self.release {
            format!("target/{}/ios/{}.app",release, capitalized).into()
        }else {
            format!("target/{}/ios/{}.app",release, capitalized).into()
        };
        self.output_path = Some(cwd.join(&partial_path));
        if self.output_path.as_ref().is_none() {
            return Err(PistonError::Generic("output path not provided".to_string()))
        }
        let bundle_path = self.output_path.as_ref().unwrap();
        println!("Bundle path: {:?}", bundle_path);

        //empty the app bundle directory if it exists
        if bundle_path.as_path().exists() {
            let _ = fs::remove_dir_all(bundle_path);
        }
        
        //Create the app bundle directory
        create_dir_all(bundle_path).map_err(|e| PistonError::CreateDirAllError {
            path: bundle_path.to_path_buf(),
            source: e,
        })?;

        println!("syncing resources...");
        let parent = bundle_path.parent();
        println!("parent path: {:?}", parent);
        let assets_path = parent.unwrap().join("Assets.xcassets");
        println!("Assets path: {:?}", assets_path.display());
        let appicon_path = assets_path.join("AppIcon.appiconset");
        println!("App icon path: {:?}", appicon_path);

        let path = appicon_path.as_path();

        //empty the AppIcon dir if it exists
        if appicon_path.exists() {
            let _ = fs::remove_dir_all(path);
        }
        
        //create the AppIcon and Assets directory
        create_dir_all(path).map_err(|e| PistonError::CreateDirAllError {
        path: self.output_path.as_ref().unwrap().to_path_buf(),
        source: e,
        })?;

        //if icon path was provided...convert
        if !self.icon_path.is_none(){
            println!("icon path provided, configuring icon");
            //resize the icon to both appropriate ios dimensions
            let icon_path120: PathBuf = appicon_path.join("ios_icon120.png");
            Helper::resize_png(&self.icon_path.as_ref().unwrap(), &icon_path120.display().to_string(), 120, 120)?;
            let icon_path180: PathBuf = appicon_path.join("ios_icon180.png");
            Helper::resize_png(&self.icon_path.as_ref().unwrap(), &icon_path180.display().to_string(), 180, 180)?;
        }


        //create Contents.json within Appicon.appiconset
        let contents_path: PathBuf = appicon_path.join("Contents.json");
        //if a contents file exists, first remove it.
        if contents_path.exists() {
            remove_file(&contents_path).map_err(|e| PistonError::RemoveFileError {
                path: contents_path.clone().to_path_buf(),
                source: e,
            })?;
        }
        //create a new Contents.json file
        let mut contents_file = File::create(&contents_path).map_err(|e| PistonError::CreateFileError {
            path: contents_path.clone().to_path_buf(),
            source: e,
        })?;

        //populate the Contents.json file
        let json_contents = r#"{
    "images": [
        {
            "size": "120x120",
            "idiom": "iphone",
            "filename": "ios_icon120.png",
            "scale": "1x"
        },
        {
            "size": "180x180",
            "idiom": "iphone",
            "filename": "ios_icon180.png",
            "scale": "1x" 
        }
    ],
    "info": {
        "version": 1,
        "author": "xcode"
    }
}"#;
        
        //write the contents.json file
        contents_file.write_all(json_contents.trim().as_bytes())
            .map_err(|e| PistonError::WriteFileError(e.to_string()))?;


        //TODO sync assets is currently disabled 
        //eventually this will move assets into subdirs contained within Assets.xcassets/

        // sync assets
        // let bind = &self.assets.clone();
        // let assets_src = Path::new(&bind);
        // let parent = self.output_path.clone().unwrap().parent();
        // let assets_tgt = parent.join("Assets.xcassets");
        // Helper::sync_assets(assets_src, &assets_tgt)?;

        let assets_output = bundle_path.join("Assets.car");
        //compile assets to Testbuild.app/Assets.car
        let status = Command::new("xcrun")
            .args([
                "actool", 
                "--output-format", "human-readable-text",
                "--platform", "iphoneos",
                "--minimum-deployment-target", "15.0",
                "--app-icon", "AppIcon",
                "--compile", 
                &assets_path.display().to_string(),
                &assets_output.display().to_string()
            ])
            .output()
            .map_err(|e| PistonError::Generic(format!("Failed to compile assets: {}", e)))?;

        if !status.status.success(){
            return Err(PistonError::Generic(format!("Failed to compile assets: {}", String::from_utf8_lossy(&status.stderr))))
        }

        //establish Info.plist path ~/ios/release/Appname.app/Info.plist
        let plist_path: PathBuf = bundle_path.join("Info.plist");
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
        let plist_content = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>{}</string>
    <key>CFBundleIdentifier</key>
    <string>{}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleDisplayName</key>
    <string>{}</string>
    <key>CFBundleExecutable</key>
    <string>{}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>{}</string>
    <key>CFBundleVersion</key>
    <string>{}</string>
    <key>LSRequiresIPhoneOS</key>
    <true/>
    <key>MinimumOSVersion</key>
    <string>{}</string>
    <key>UIDeviceFamily</key>
    <array>
        <integer>1</integer>
        <integer>2</integer>
    </array>
    <key>CFBundleSupportedPlatforms</key>
    <array>
        <string>iPhoneOS</string>
    </array>
    <key>DTPlatformName</key>
    <string>iphoneos</string>
    <key>DTPlatformVersion</key>
    <string>18.5</string>
    <key>DTSDKName</key>
    <string>iphoneos18.5</string>
    <key>DTCompiler</key>
    <string>com.apple.compilers.llvm.clang.1_0</string>
    <key>DTXcode</key>
    <string>1620</string>
    <key>CFBundleIconName</key>
    <string>AppIcon</string>
    <key>CFBundleIcons</key>
    <dict>
        <key>CFBundlePrimaryIcon</key>
        <dict>
            <key>CFBundleIconName</key>
            <string>AppIcon</string>
        </dict>
    </dict>
</dict>
</plist>
"#, 
    &capitalized,
    &self.bundle_id.clone(),
    &capitalized,
    &capitalized,
    &self.app_version.clone(),
    &self.app_version.clone(),
    &self.min_os_version.clone()
);

        plist_file.write_all(plist_content.trim().as_bytes())
            .map_err(|e| PistonError::WriteFileError(e.to_string()))?;

        let output = Command::new("plutil")
            .args(["-convert", "binary1", &plist_path.display().to_string()])
            .output()
            .map_err(|e| PistonError::PlutilConvertError(e.to_string()))?;
        if !output.status.success() {
            return Err(PistonError::PlutilConvertError(String::from_utf8_lossy(&output.stderr).to_string()))
        }
        Ok(())
        
    }

    fn build(&mut self) -> Result <(), PistonError>{
        println!("build for ios");
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

        Ok(())
    }

    fn post_build(&mut self) -> Result <(), PistonError>{
        println!("post build for ios");
        let provision_cache = self.cwd.join("target").join("ios-cache").join("profiles");
        let binary_path = self.cwd.join("target").join(self.target.clone()).join(if self.release {"release"} else {"debug"}).join(self.app_name.clone());
        let capitalized = Helper::capitalize_first(&self.app_name.clone());
        let bundle_path = self.output_path.as_ref().unwrap().join(&capitalized);
        //bundle path should be cwd + target + <target output> + <--release flag or None for debug> + <appname>.exe
        //move the target binary into the app bundle at the proper location
        copy(&binary_path, &bundle_path).map_err(|e| PistonError::CopyFileError {
            input_path: binary_path.clone().to_path_buf(),
            output_path: bundle_path.clone().to_path_buf(),
            source: e,
        })?;
        // Make the binary executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let exe_name = self.app_name.clone();
            let exe_path = self.output_path.as_ref().unwrap().join(&exe_name);
            
            fs::set_permissions(&exe_path, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| PistonError::Generic(format!("Failed to make binary executable: {}", e)))?;
        }
        //strip extended attributes
        let output = Command::new("xattr")
            .args(["-cr", &bundle_path.display().to_string()])
            .output()
            .map_err(|e| PistonError::Generic(e.to_string()))?;
        if !output.status.success() {
            return Err(PistonError::Generic(String::from_utf8_lossy(&output.stderr).to_string()))
        }
        //output the proper location in the terminal for the user to see 
        println!("iOS app bundle available at: {}", &bundle_path.display());

        //check for apple signing certificate
        if self.keystore_path.is_none() || self.asc_api_key.is_none() {
            println!("Keystore path or ASC API key missing from .env, skipping automated signing");
        } else {
            println!("keystore path & ASC API key properly configured");
            let asc_client = AscClient{ api_key: self.asc_api_key.clone(), keystore_path: self.keystore_path.clone().unwrap()};
            //obtain security certificate
            let security_cert = asc_client.create_or_find_security_cert(self.team_id.clone())?;
            let security_profile = format!("{} ({})", security_cert.1, security_cert.0);
            println!("your security profile is: {:?}", security_profile);
            let output_path = self.output_path.clone().unwrap();
            let app_name = self.app_name.clone();
            let bundle_id = self.bundle_id.clone();
            //if a device target is provided, check if the target device is provisioned
            if !self.device_target.is_none() {
                println!("device target exists, checking for existing provisioning");
                let target_id = self.device_target.clone().unwrap().id;
                let idp_path = self.idp_path.clone().unwrap();
                let provisioned = AscClient::is_device_provisioned(&output_path, &target_id, &idp_path, &provision_cache)?;
                //if device is not provisioned and api access is available, attempt to provision
                if provisioned == false && !self.asc_api_key.is_none() {
                    println!("attempting to provision target device {:?}", self.device_target);
                    let app_name = self.app_name.clone();
                    //provision device here
                    asc_client.provision_ios_device(&target_id, &bundle_id, &app_name, &security_profile, &output_path, &idp_path, &provision_cache)?;
                }
            }
            //sign the app bundle
            AscClient::sign_app_bundle(&app_name, &output_path, &security_profile, &bundle_id, true, false)?;
            //remove any existing .ipa
            let parent = output_path.parent().unwrap();
            println!("Parent path is: {}", parent.display());
            let ipa_path = parent.join(format!("{}.ipa", &capitalized));
            println!("IPA path is: {}", ipa_path.display());
            self.ipa_path = Some(ipa_path.clone());
            if ipa_path.as_path().exists() {
                remove_file(&ipa_path).map_err(|e| PistonError::RemoveFileError {
                    path: ipa_path.to_path_buf(),
                    source: e,
                })?;
            }
            //remove any existing payload dir and its contents
            let payload_path = parent.join("Payload");
            if payload_path.exists() {
                let _ = fs::remove_dir_all(&payload_path);
            }
            //create ~/Payload dir
            create_dir_all(&payload_path).map_err(|e| PistonError::CreateDirAllError {
                path: payload_path.to_path_buf(),
                source: e,
            })?;
            //copy app bundle contents to payload dir
            let dest = payload_path.join(&capitalized);
            copy(&bundle_path, &dest).map_err(|e| PistonError::CopyFileError {
                input_path: output_path.clone().to_path_buf(),
                output_path: dest.clone().to_path_buf(),
                source: e,
            })?;
            //zip contents of the payload to create an .ipa
            let status = Command::new("zip")
                .arg("-r")
                .arg(&ipa_path)
                .arg(&payload_path)
                .output()
                .map_err(|e| PistonError::Generic(format!("Error zipping payload: {}", e)))?;

            if !status.status.success() {
                return Err(PistonError::Generic(format!("Error zipping payload: {}", String::from_utf8_lossy(&status.stderr))))
            }
            //cleanup temp payload dir
            let _ = std::fs::remove_dir_all(payload_path);

            println!("Your app is available at: {:?}", &ipa_path.display());
        }
        Ok(())
    }

}

pub struct IOSRunner{}

impl IOSRunner{

    pub fn start(release: bool, cwd: PathBuf, env_vars: HashMap<String, String>, device: &IOSDevice) -> Result<(), PistonError> {
        println!("running for IOS");
        let target_string = "aarch64-apple-ios".to_string();
        if std::env::consts::OS != "macos"{
            println!("error cannot run mac on linux");
            return Err(PistonError::UnsupportedOSError{os: std::env::consts::OS.to_string(), target: target_string})
        }
        //build the app bundle and sign
        let builder = IOSBuilder::start(release, target_string, cwd, env_vars, Some(device.clone()))?;
        //deploy the app bundle to the target device
        IOSRunner::deploy_usb(device.id.as_ref(), &builder.0.display().to_string(), &builder.1)?;

        Ok(())
    }

    //TODO this is currently broken 
    fn deploy_usb(device_id: &str, output_path: &str, bundle_id: &str) -> Result<(), PistonError> {

        // Force-remove any old version of the app (same bundle ID)
        let _ = Command::new("xcrun")
            .args(["devicectl", "device", "uninstall", "app", "--device", device_id, "--bundle-id", bundle_id])
            .output();
        println!("installing app ID: {} located at: {} to device: {}", bundle_id, output_path, device_id);
        let output = Command::new("xcrun")
            .args(["devicectl", "device", "install", "app", "--device", &device_id, &output_path])
            .output()
            .map_err(|e| PistonError::XcrunInstallError(e.to_string()))?;
        if !output.status.success() {
            println!("Failed to install with Xcrun: {:?}", &output);
            return Err(PistonError::XcrunInstallError(String::from_utf8_lossy(&output.stderr).trim().to_string()));
        }
        println!("Running bundle id: {} on device: {}", &bundle_id, &device_id);
        let output = Command::new("xcrun")
            .args(["devicectl", "device", "process", "launch", "--device", &device_id, &bundle_id])
            .output()
            .map_err(|e| PistonError::XcrunLaunchError(e.to_string()))?;
        if !output.status.success() {
            return Err(PistonError::XcrunLaunchError(String::from_utf8_lossy(&output.stderr).trim().to_string()));
        }
        Ok(())
    }
}

