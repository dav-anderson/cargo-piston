use std::path::{ PathBuf, Path };
use std::collections::HashMap;
use std::process::{ Command, Stdio };
use std::io::{ Write };
use cargo_metadata::{ Metadata, MetadataCommand };
use std::fs::{ copy,File, create_dir_all, remove_file };
use serde::{ Serialize, Deserialize };
use serde_json::{ Value, json };
use std::thread::sleep;
use std::time::Duration;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use ureq::Response;
use crate::Helper;
use crate::PistonError;
use crate::devices::IOSDevice;

#[derive(Deserialize, Default)]
struct IOSMetadata {
    #[serde(default)]
    bundle_id: Option<String>,
}


pub struct IOSBuilder {
    release: bool,
    target: String,
    cwd: PathBuf,
    output_path: Option<PathBuf>,
    icon_path: Option<String>,
    cargo_path: String,
    app_name: String,
    app_version: String,
    bundle_id: String,
    min_os_version: f32,
    asc_api_key: Option<AscApiKey>,
    device_target: Option<IOSDevice>,
    idp_path: Option<String>,
    keystore_path: Option<String>,
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

        Ok((op.output_path.unwrap(), op.bundle_id))
    }

    fn new(release: bool, target: String, cwd: PathBuf, env_vars: HashMap<String, String>, device_target: Option<IOSDevice>) -> Result<Self, PistonError> {
        println!("creating IOSBuilder: release: {:?}, target: {:?}, cwd: {:?}", release, target.to_string(), cwd);
        //parse env vars
        let cargo_path = env_vars.get("cargo_path").cloned().unwrap_or("cargo".to_string());
        let idp_path = env_vars.get("idp_path").cloned();
        let keystore_path = env_vars.get("keystore_path").cloned();
        println!("Cargo path determined: {}", &cargo_path);
        //parse cargo.toml
        let metadata: Metadata = MetadataCommand::new()
            .current_dir(cwd.clone())
            .exec()
            .map_err(|e| PistonError::CargoParseError(e.to_string()))?;

        let icon_path = Helper::get_icon_path(&metadata);
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

        println!("asc_api_key: {:?}", asc_api_key);

        Ok(IOSBuilder{
            release: release, 
            target: target.to_string(), 
            cwd: cwd, 
            output_path: None, 
            icon_path: icon_path, 
            cargo_path: cargo_path, 
            app_name: app_name, 
            app_version: app_version, 
            bundle_id: bundle_id, 
            min_os_version: min_os_version, 
            asc_api_key: asc_api_key,
            device_target: device_target, 
            idp_path: idp_path,
            keystore_path: keystore_path,
        })
    }

    fn pre_build(&mut self) -> Result <(), PistonError>{
        //TODO check xcode for updates?
        //TODO allow user to specify a security cert for offline signing
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
            .map_err(|e| PistonError::XcodeSelectInstallError("Failed to verify xcode tools installation".to_string()));

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
            .map_err(|e| PistonError::XcodeBuildError("Failed to run xcodebuild -showsdks. Something is likely missing from your installation".to_string()));

        let sdk_binding = sdk_output.unwrap();
        let sdks = String::from_utf8_lossy(&sdk_binding.stdout);
        if !sdks.contains("iOS") {
            return Err(PistonError::IOSSdkMissingError("IOS sdk is missing. Try running 'xcodebuild -downloadPlatform iOS'".to_string()))
        }
        //build the app bundle
        println!("building the dynamic app bundle");
        let cwd: PathBuf = self.cwd.clone();
        println!("working dir: {:?}", cwd);
        let capitalized = Helper::capitalize_first(&self.app_name.clone());
        println!("capitalized app name: {}", capitalized);
        let release = if self.release {"release"} else {"debug"};
        //fix the path to match ios convention
        let partial_path: PathBuf = if self.release {
            format!("target/{}/ios/{}.app",release, capitalized).into()
        }else {
            format!("target/{}/ios/{}.app",release, capitalized).into()
        };
        println!("partial path: {:?}", partial_path);
        //establish ~/target/<release>/ios/Appname.app/Resources
        let res_path: PathBuf = partial_path.join("Resources");
        println!("res path: {:?}", res_path);
        self.output_path = Some(cwd.join(&partial_path));
        println!("full path to ios dir: {:?}", self.output_path);
        //empty the target directory if it exists
        if self.output_path.as_ref().is_none() {
            return Err(PistonError::Generic("output path not provided".to_string()))
        }
        //Empty the directory if it already exists
        let path = res_path.as_path();
        //empty the dir if it exists
        if let Some(path) = &self.output_path {
            if path.exists() {
                let _ = fs::remove_dir_all(path);
            }
        }
        //create the target directories
        create_dir_all(path).map_err(|e| PistonError::CreateDirAllError {
        path: self.output_path.as_ref().unwrap().to_path_buf(),
        source: e,
        })?;
        //establish Info.plist path ~/ios/release/Appname.app/Info.plist
        let plist_path: PathBuf = partial_path.join("Info.plist");
        println!("plist path: {:?}", plist_path);
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
    <key>LSRequiresIphoneOS</key>
    <true/>
    <key>MinimumOSVersion</key>
    <string>{}</string>
    <key>UIDeviceFamily</key>
    <array>
        <integer>1</integer>
        <integer>2</integer>
    </array>
    <key>CFBundleIcons</key>
    <dict>
        <key>CFBundlePrimaryIcon</key>
        <dict>
            <key>CFBundleIconFiles</key>
            <array>
                <string>Resources/ios_icon120</string>
                <string>Resources/ios_icon180</string>
            </array>
            <key>UIPrerenderedIcon</key>
            <false/>
        </dict>
    </dict>
</dict>
</plist>
"#, 
    &capitalized,
    &self.bundle_id.clone(),
    &self.app_name.clone(),
    &self.app_name.clone(),
    &self.app_version.clone(),
    &self.app_version.clone(),
    &self.min_os_version.clone()
);

        plist_file.write_all(plist_content.trim().as_bytes())
            .map_err(|e| PistonError::WriteFileError(e.to_string()))?;

        let _ = Command::new("plutil").args(["convert", "xml1", "-o", &plist_path.display().to_string(), &plist_path.display().to_string()]).output();
        println!("Info.plist created");
        //if icon path was provided...convert
        if !self.icon_path.is_none(){
            println!("icon path provided, configuring icon");
            //resize the icon to both appropriate ios dimensions
            let icon_path120: PathBuf = res_path.join("ios_icon120.png");
            Helper::resize_png(&self.icon_path.as_ref().unwrap(), &icon_path120.display().to_string(), 120, 120)?;
            let icon_path180: PathBuf = res_path.join("ios_icon180.png");
            Helper::resize_png(&self.icon_path.as_ref().unwrap(), &icon_path180.display().to_string(), 180, 180)?;
        }
        println!("done configuring ios bundle");
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
        let binary_path = self.cwd.join("target").join(self.target.clone()).join(if self.release {"release"} else {"debug"}).join(self.app_name.clone());
        let bundle_path = self.output_path.as_ref().unwrap().join(self.app_name.clone());
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
        // Make the binary executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let exe_name = self.app_name.clone();
            let exe_path = self.output_path.as_ref().unwrap().join(&exe_name);
            
            fs::set_permissions(&exe_path, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| PistonError::Generic(format!("Failed to make binary executable: {}", e)))?;
            
            println!("Set executable permission on {}", exe_path.display());
        }
        //output the proper location in the terminal for the user to see 
        println!("iOS app bundle available at: {}", &bundle_path.display());

        //check for apple signing certificate
        if self.keystore_path.is_none() || self.asc_api_key.is_none() {
            println!("Keystore path or ASC API key missing from .env, skipping automated signing");
        } else {
            println!("keystore path & ASC API key properly configured");
            let asc_client = AscClient{ api_key: self.asc_api_key.clone(), keystore_path: self.keystore_path.clone().unwrap()};
            //Note: this presently always create an IOS_DISTRIBUTION cert
            let security_profile = asc_client.create_or_find_security_cert()?;
            println!("your security profile is: {:?}", security_profile);
            let output_path = self.output_path.clone().unwrap();

            //if a device target is provided, check if the target device is provisioned
            if !self.device_target.is_none() {
                println!("device target exists, checking for existing provisioning");
                let target_id = self.device_target.clone().unwrap().id;
                let target_udid = self.device_target.clone().unwrap().udid;
                let idp_path = self.idp_path.clone().unwrap();
                let provisioned = AscClient::is_device_provisioned(&output_path, &target_id, &target_udid, &idp_path)?;
                //if device is not provisioned and api access is available, attempt to provision
                if provisioned == false && !self.asc_api_key.is_none() {
                    println!("attempting to provision target device {:?}", self.device_target);
                    let bundle_id = self.bundle_id.clone();
                    let app_name = self.app_name.clone();
                    //provision device here
                    asc_client.provision_ios_device(&target_id, &bundle_id, &app_name, &security_profile.0, &output_path, &idp_path)?;
                    AscClient::sign_app_bundle(&output_path, security_profile.1.as_ref())?;
                    return Ok(())
                }
            }
            //sign the app bundle for distribution
            AscClient::sign_app_bundle(&output_path, security_profile.1.as_ref())?;
        }
        Ok(())
    }

}

pub struct IOSRunner{
device: IOSDevice, 
}

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
        let runner = IOSRunner::deploy_usb(device.id.as_ref(), &builder.0.display().to_string(), &builder.1)?;

        Ok(())
    }

    //TODO this is currently broken 
    fn deploy_usb(device_id: &str, output_path: &str, bundle_id: &str) -> Result<(), PistonError> {
        // Force-remove any old version of the app (same bundle ID)
        let _ = Command::new("xcrun")
            .args(["devicectl", "device", "uninstall", "app", "--device", device_id, "--bundle-id", bundle_id])
            .output();
        println!("installing app ID: {} to device: {}", bundle_id, device_id);
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

#[derive(Debug, Clone)]
pub struct AscApiKey {
    pub key_id: String,
    pub issuer_id: String,
    pub priv_key: String,
}

impl AscApiKey {
    //parse the ASC API key information from the .env
    pub fn from_hm(env: &HashMap<String, String>) -> Result<Self, String> {
        let key_id = env.get("asc_key_id")
            .ok_or("Missing ASC_KEY_ID in .env")
            .clone();

        let issuer_id = env.get("asc_issuer_id")
            .ok_or("Missing ASC_ISSUER_ID in .env")
            .clone();
        
        let p8_path = env.get("asc_key_path")
            .ok_or("Missing ASC_KEY_PATH in .env")
            .clone();

        let priv_key = fs::read_to_string(&p8_path.unwrap())
            .map_err(|e| format!("failed to read .p8 file at {:?}: {:?}", p8_path, e))?;

        Ok(Self { key_id: key_id.unwrap().to_string(), issuer_id: issuer_id.unwrap().to_string(), priv_key: priv_key})
    }
}

#[derive(Debug)]
pub struct AscClient {
    api_key: Option<AscApiKey>,
    keystore_path: String,
}

impl AscClient {
    //get cached metadata for ios builds
    fn get_cache_dir(&self) -> PathBuf {
        PathBuf::from("target/ios-cache")
    }

    //load the cached security certificate identity
    fn load_cert_cache(&self) -> Option<(String, String)> {
        let path = self.get_cache_dir().join("cert_cache.json");
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                let cert_id = json["cert_id"].as_str()?.to_string();
                let signing_identity = json["signing_identity"].as_str()?.to_string();
                return Some((cert_id, signing_identity));
            }
        }
        None
    }

    //cache security certificate identity
    fn save_cert_cache(&self, cert_id: &str, signing_identity: &str) {
        let dir = self.get_cache_dir();
        let _ = fs::create_dir_all(&dir);
        let data = json!({
            "cert_id": cert_id,
            "signing_identity": signing_identity
        });
        let _ = fs::write(dir.join("cert_cache.json"), data.to_string());
    }

    // Creates or re-uses an iOS Distribution certificate
    // Returns: (certificate_id, signing_identity_name) — name is normalized for codesign, ASC API returns something unique
    pub fn create_or_find_security_cert(
        &self,
    ) -> Result<(String, String), PistonError> {
        // First check cache
        if let Some((cert_id, signing_identity)) = self.load_cert_cache() {
            // Quick local keychain check
            let check = Command::new("security")
                .args(["find-identity", "-v", "-p", "codesigning"])
                .output()
                .map_err(|e| PistonError::SecurityFindIdentityError(e.to_string()))?;
            //use cached security credentials
            let output = String::from_utf8_lossy(&check.stdout);
            if output.contains(&signing_identity) {
                println!("✅ Using cached security certificate");
                return Ok((cert_id, signing_identity));
            }
        }
        // If cache missing locally → create/re-use credentials via API
        // 0. Unlock keychain
        let keychain_path = format!("{}/login.keychain-db", self.keystore_path.clone());
        let _ = Command::new("security")
            .args(["unlock-keychain", &keychain_path])
            .output();

        let status = Command::new("security")
            .args(["show-keychain-info", &keychain_path])
            .output()
            .map_err(|e| PistonError::KeyChainUnlockError(format!("Failed to check keychain: {}", e)))?;

        if String::from_utf8_lossy(&status.stdout).contains("locked") {
            return Err(PistonError::KeyChainUnlockError("User cancelled keychain unlock".to_string()));
        }
        println!("✅ Keychain unlocked");

        let token = self.generate_jwt()?;
        let cert_type = "IOS_DISTRIBUTION";
        let id_type = "Distribution";

        println!("Checking for existing {} certificate in ASC...", id_type);

        let list_resp: Response = ureq::get("https://api.appstoreconnect.apple.com/v1/certificates")
            .set("Authorization", &format!("Bearer {}", token))
            .query("filter[certificateType]", cert_type)
            .call()
            .map_err(|e| PistonError::ASCClientUreqError {
                endpoint: "list certificates".to_string(),
                e: format!("Failed to list certificates: {}", e),
            })?;

        let json: serde_json::Value = list_resp.into_json()
            .map_err(|e| PistonError::IntoJSONError(e.to_string()))?;

        if let Some(existing) = json["data"].as_array().and_then(|arr| arr.first()) {
            let cert_id = existing["id"].as_str().unwrap().to_string();
            let cert_name = existing["attributes"]["name"]
                .as_str()
                .unwrap_or("Unknown")
                .to_string();

            println!("Found existing {} certificate in ASC: {}", id_type, cert_name);

            // Check if it actually exists locally in keychain
            let check = Command::new("security")
                .args(["find-identity", "-v", "-p", "codesigning"])
                .output()
                .map_err(|e| PistonError::KeyChainImportError(format!("Failed to check keychain: {}", e)))?;

            let output = String::from_utf8_lossy(&check.stdout);

            if output.contains(&cert_name) || 
            output.contains(&cert_name.replace("iOS Distribution", "iPhone Distribution")) {
                println!("✅ Certificate also found in local keychain → reusing");

                // Normalize to what codesign expects
                let signing_identity = cert_name.replace("iOS Distribution", "iPhone Distribution");
                return Ok((cert_id, signing_identity));
            } else {
                println!("⚠️  Certificate exists in ASC but missing locally → creating a new one");
                // No automatic revocation, we just create a fresh certificate (Apple allows multiples)
            }
        }

        // CREATE NEW SECURITY CERTIFICATE
        println!("Generating new {} certificate...", id_type);

        let key_path = "temp_key.pem";
        let csr_path = "temp_csr.csr";

        // Generate PEM key + CSR
        let _ = Command::new("openssl")
            .args(["genrsa", "-out", key_path, "2048"])
            .output()
            .map_err(|e| PistonError::OpenSSLKeyGenError(format!("keygen failed: {}", e)))?;

        let _ = Command::new("openssl")
            .args(["req", "-new", "-key", key_path, "-out", csr_path, "-subj", "/CN=Distribution Certificate"])
            .output()
            .map_err(|e| PistonError::OpenSSLCSRError(format!("csr failed: {}", e)))?;

        let csr_content = fs::read_to_string(csr_path)
            .map_err(|e| PistonError::ReadCSRError(format!("Failed to read CSR: {}", e)))?;

        // Upload CSR
        let body = json!({
            "data": {
                "type": "certificates",
                "attributes": {
                    "certificateType": cert_type,
                    "csrContent": csr_content
                }
            }
        });

        let create_resp = ureq::post("https://api.appstoreconnect.apple.com/v1/certificates")
            .set("Authorization", &format!("Bearer {}", token))
            .set("Content-Type", "application/json")
            .send_json(&body)
            .map_err(|e| PistonError::ASCClientUreqError {
                endpoint: "upload CSR".to_string(),
                e: format!("Upload failed: {}", e),
            })?;

        let json: serde_json::Value = create_resp.into_json()
            .map_err(|e| PistonError::IntoJSONError(e.to_string()))?;

        let cert_id = json["data"]["id"].as_str().unwrap().to_string();
        let cert_name = json["data"]["attributes"]["name"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string();

        // Download + decode
        let cert_b64 = json["data"]["attributes"]["certificateContent"]
            .as_str()
            .ok_or_else(|| PistonError::Generic("No certificateContent returned".to_string()))?;

        let cert_der = base64::decode(cert_b64)
            .map_err(|e| PistonError::Base64DecodeError(format!("Decode failed: {}", e)))?;

        let cer_path = "temp_cert.cer";
        fs::write(cer_path, cert_der)
            .map_err(|e| PistonError::WriteFileError(format!("Write failed: {}", e)))?;

        // Import key + cert
        let import_key = Command::new("security")
            .args(["import", key_path, "-k", &keychain_path, self.keystore_path.as_ref(), "-P", ""])
            .output()
            .map_err(|e| PistonError::KeyChainImportError(format!("Key import failed: {}", e)))?;

        if !import_key.status.success() {
            return Err(PistonError::KeyChainImportError(
                String::from_utf8_lossy(&import_key.stderr).trim().to_string()
            ));
        }

        let import_cert = Command::new("security")
            .args(["import", cer_path, "-k", &keychain_path, self.keystore_path.as_ref()])
            .output()
            .map_err(|e| PistonError::KeyChainImportError(format!("Cert import failed: {}", e)))?;

        if !import_cert.status.success() {
            return Err(PistonError::KeyChainImportError(
                String::from_utf8_lossy(&import_cert.stderr).trim().to_string()
            ));
        }

        // Cleanup
        let _ = fs::remove_file(key_path);
        let _ = fs::remove_file(csr_path);
        let _ = fs::remove_file(cer_path);

        // Normalize to what codesign actually expects
        let signing_identity = cert_name.replace("iOS Distribution", "iPhone Distribution");

        println!("✅ New {} certificate created and imported (ID: {}, Name for codesign: {})", id_type, cert_id, signing_identity);
        //cache the security credentials locally
        self.save_cert_cache(&cert_id, &signing_identity);
        Ok((cert_id, signing_identity))
    }

    //generates a JWT for interfacing with Apple AppStoreConnect API
    fn generate_jwt(&self) -> Result<String, PistonError> {
        let exp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize + 1200;

        #[derive(Serialize, Deserialize)]
        struct Claims {
            iss: String,
            exp: usize,
            aud: String,
        }

        let claims = Claims {
            iss: self.api_key.as_ref().unwrap().issuer_id.clone(),
            exp,
            aud: "appstoreconnect-v1".to_string(),
        };

        let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::ES256);
        header.kid = Some(self.api_key.as_ref().unwrap().key_id.clone());

        let key = jsonwebtoken::EncodingKey::from_ec_pem(self.api_key.clone().unwrap().priv_key.as_bytes())
            .map_err(|e| PistonError::ASCClientParseEncodingKeyError(format!("Invalid .pem key: {}", e)))?;

        jsonwebtoken::encode(&header, &claims, &key).map_err(|e| PistonError::ASCClientJWTEncodeError(e.to_string()) )
    }

    //Registers device if needed → creates/re-uses Ad Hoc profile → 
    // downloads .mobileprovision → embeds it → installs to device → extracts entitlements.plist
    pub fn provision_ios_device(
        &self,
        device_id: &str,
        bundle_id: &str,
        app_name: &str,
        certificate_id: &str,
        app_bundle_path: &PathBuf,
        ideviceprovision_path: &str, 
    ) -> Result<(), PistonError> {
        let token = self.generate_jwt()?;
        println!("Provisioning device {} for app '{}' (bundle {})", device_id, app_name, bundle_id);
        let cache_dir = self.get_cache_dir();

        // // 1. Register device if missing
        let device_resource_id = {
            let check = ureq::get("https://api.appstoreconnect.apple.com/v1/devices")
                .set("Authorization", &format!("Bearer {}", token))
                .query("filter[udid]", device_id)
                .call()
                .map_err(|e| PistonError::ASCClientUreqError {
                    endpoint: "check device".to_string(),
                    e: format!("Device check failed: {}", e),
                })?;

            let json: serde_json::Value = check.into_json()
                .map_err(|e| PistonError::IntoJSONError(e.to_string()))?;

            if let Some(existing) = json["data"].as_array().and_then(|a| a.first()) {
                println!("Device already registered");
                existing["id"].as_str().unwrap().to_string()
            } else {
                println!("Registering device...");
                let body = json!({
                    "data": {
                        "type": "devices",
                        "attributes": {
                            "name": app_name,
                            "udid": device_id,
                            "platform": "IOS"
                        }
                    }
                });

                let resp = ureq::post("https://api.appstoreconnect.apple.com/v1/devices")
                    .set("Authorization", &format!("Bearer {}", token))
                    .set("Content-Type", "application/json")
                    .send_json(&body)
                    .map_err(|e| PistonError::ASCClientUreqError {
                        endpoint: "register device".to_string(),
                        e: format!("Registration failed: {}", e),
                    })?;

                let json: serde_json::Value = resp.into_json()
                    .map_err(|e| PistonError::IntoJSONError(e.to_string()))?;

                json["data"]["id"].as_str().unwrap().to_string()
            }
        };
        // 2. Get or create Bundle ID
        let bundle_resource_id = {
            println!("Checking if bundle ID {} exists in ASC...", bundle_id);

            let check = ureq::get("https://api.appstoreconnect.apple.com/v1/bundleIds")
                .set("Authorization", &format!("Bearer {}", token))
                .query("filter[identifier]", bundle_id)
                .call()
                .map_err(|e| PistonError::ASCClientUreqError {
                    endpoint: "check bundle id".to_string(),
                    e: format!("Bundle check failed, if the status code is 409, try setting a new and unique bundle id in your cargo.toml: {}", e),
                })?;

            let json: serde_json::Value = check.into_json()
                .map_err(|e| PistonError::IntoJSONError(e.to_string()))?;

            if let Some(existing) = json["data"].as_array().and_then(|a| a.first()) {
                println!("✅ Bundle ID already exists in ASC");
                existing["id"].as_str().unwrap().to_string()
            } else {
                println!("Bundle ID not found → attempting to create...");
                let body = json!({
                    "data": {
                        "type": "bundleIds",
                        "attributes": {
                            "identifier": bundle_id,
                            "name": app_name,
                            "platform": "IOS"
                        }
                    }
                });

                let resp = ureq::post("https://api.appstoreconnect.apple.com/v1/bundleIds")
                    .set("Authorization", &format!("Bearer {}", token))
                    .set("Content-Type", "application/json")
                    .send_json(&body)
                    .map_err(|e| PistonError::ASCClientUreqError {
                        endpoint: "create bundle id".to_string(),
                        e: format!("HTTP request failed: {}", e),
                    })?;

                let status = resp.status();

                if (200..300).contains(&status) {
                    let json: serde_json::Value = resp.into_json()
                        .map_err(|e| PistonError::IntoJSONError(e.to_string()))?;
                    let id = json["data"]["id"].as_str().unwrap().to_string();
                    println!("✅ Bundle ID created successfully");
                    id
                } else {
                    let error_body = resp.into_string().unwrap_or_default();
                    return Err(PistonError::ASCClientUreqError {
                        endpoint: "create bundle id".to_string(),
                        e: format!("ASC returned {}: {}", status, error_body),
                    });
                }
            }
        };

        // 3. Create Ad Hoc profile
        let profile_name = format!("{}-AdHoc", app_name);
        let profile_id = {
            println!("Checking for existing Ad Hoc profile for this bundle...");

            let check = ureq::get(&format!(
                "https://api.appstoreconnect.apple.com/v1/bundleIds/{}/profiles",
                bundle_resource_id
            ))
            .set("Authorization", &format!("Bearer {}", token))
            .call()
            .map_err(|e| PistonError::ASCClientUreqError {
                endpoint: "check profile".to_string(),
                e: format!("Profile check failed: {}", e),
            })?;

            let json: serde_json::Value = check.into_json()
                .map_err(|e| PistonError::IntoJSONError(e.to_string()))?;
            // Look for an existing Ad Hoc profile
            if let Some(existing) = json["data"].as_array().and_then(|arr| {
                arr.iter().find(|p| {
                    p["attributes"]["profileType"].as_str() == Some("IOS_APP_ADHOC")
                })
            }) {
                println!("✅ Existing matching Ad Hoc profile found");
                existing["id"].as_str().unwrap().to_string()
            } else {
                println!("No matching profile found → creating new one...");

                let body = json!({
                    "data": {
                        "type": "profiles",
                        "attributes": {
                            "name": profile_name,
                            "profileType": "IOS_APP_ADHOC"
                        },
                        "relationships": {
                            "bundleId": { "data": { "type": "bundleIds", "id": bundle_resource_id } },
                            "certificates": { "data": [{ "type": "certificates", "id": certificate_id }] },
                            "devices": { "data": [{ "type": "devices", "id": device_resource_id }] }
                        }
                    }
                });

                let create_resp = ureq::post("https://api.appstoreconnect.apple.com/v1/profiles")
                    .set("Authorization", &format!("Bearer {}", token))
                    .set("Content-Type", "application/json")
                    .send_json(&body)
                    .map_err(|e| PistonError::ASCClientUreqError {
                        endpoint: "create profile".to_string(),
                        e: format!("Profile creation failed: {}", e),
                    })?;

                let json: serde_json::Value = create_resp.into_json()
                    .map_err(|e| PistonError::IntoJSONError(e.to_string()))?;

                json["data"]["id"].as_str().unwrap().to_string()
            }
        };

        // 4. Download .mobileprovision
        let dl_resp = ureq::get(&format!(
            "https://api.appstoreconnect.apple.com/v1/profiles/{}",
            profile_id
        ))
        .set("Authorization", &format!("Bearer {}", token))
        .call()
        .map_err(|e| PistonError::ASCClientUreqError {
            endpoint: "download profile".to_string(),
            e: format!("Profile download failed: {}", e),
        })?;

        let dl_json: serde_json::Value = dl_resp.into_json()
            .map_err(|e| PistonError::IntoJSONError(e.to_string()))?;

        let b64 = dl_json["data"]["attributes"]["profileContent"]
            .as_str()
            .ok_or_else(|| PistonError::Generic("No profileContent returned".to_string()))?;

        let profile_data = base64::decode(b64)
            .map_err(|e| PistonError::Base64DecodeError(format!("Decode failed: {}", e)))?;

        let profile_path = format!("{}.mobileprovision", profile_name);
        fs::write(&profile_path, profile_data)
            .map_err(|e| PistonError::WriteFileError(format!("Failed to write profile: {}", e)))?;

        //cache mobile provision locally
        let cache_dir = self.get_cache_dir().join("profiles");
        let _ = fs::create_dir_all(&cache_dir);
        let cached_profile = cache_dir.join(format!("{}.mobileprovision", profile_name));
        let _ = fs::copy(&profile_path, &cached_profile);

        // 5. Embed into .app bundle
        let embedded_path = format!("{}/embedded.mobileprovision", app_bundle_path.display());
        fs::copy(&profile_path, &embedded_path)
            .map_err(|e| PistonError::WriteFileError(format!("Failed to embed profile: {}", e)))?;

        // 6. Install profile to device
        let install = Command::new(ideviceprovision_path)
            .args(["install", &embedded_path, "--udid", device_id])
            .output()
            .map_err(|e| PistonError::DeviceProvisionError(format!("ideviceprovision failed: {}", e)))?;

        if !install.status.success() {
            return Err(PistonError::DeviceProvisionError(
                String::from_utf8_lossy(&install.stderr).trim().to_string()
            ));
        }

        // 7. Extract entitlements.plist
        AscClient::ensure_entitlements(&app_bundle_path)?;

        // Cleanup
        let _ = fs::remove_file(profile_path);

        println!("✅ Provisioning complete for '{}' → entitlements.plist ready", app_name);
        Ok(())
    }

    //TODO add support for distribution entitlement capabilities
    // Always extracts entitlements.plist from the embedded.mobileprovision in the bundle
    // Works whether we just provisioned or are reusing a cached profile
    pub fn ensure_entitlements(app_bundle_path: &PathBuf) -> Result<(), PistonError> {
        let embedded = app_bundle_path.join("embedded.mobileprovision");
        //no device target specified, build Entitlements for distribution
        if !embedded.exists() {
            let entitlements_path = app_bundle_path.join("entitlements.plist");
            //TODO this does not currently add any capabilities outlined in ASC bundle creation
            let content = r#"<?xml version="1.0" encoding="UTF-8"?>
            <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
            <plist version="1.0">
            <dict>
            </dict>
            </plist>"#;

            fs::write(&entitlements_path, content.trim())
                .map_err(|e| PistonError::WriteFileError(format!("Failed to write entitlements.plist: {}", e)))?;

            println!("Created minimal entitlements.plist for Distribution");
            return Ok(())
        }
        //build entitlements for a target device based on an embedded.mobileprovision
        let entitlements_path = app_bundle_path.join("entitlements.plist");

        let cms = Command::new("security")
            .args(["cms", "-D", "-i", embedded.to_str().unwrap()])
            .output()
            .map_err(|e| PistonError::Generic(format!("security cms failed: {}", e)))?;

        let mut plutil = Command::new("plutil")
            .args(["-extract", "Entitlements", "xml1", "-o", entitlements_path.to_str().unwrap(), "-"])
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| PistonError::Generic(format!("plutil spawn failed: {}", e)))?;

        if let Some(mut stdin) = plutil.stdin.take() {
            stdin.write_all(&cms.stdout).map_err(|e| PistonError::WritePlUtilError(format!("Failed to write plutil file: {}", e)))?;
        }

        let result = plutil.wait_with_output()
            .map_err(|e| PistonError::Generic(format!("plutil failed: {}", e)))?;
        if !result.status.success() {
            return Err(PistonError::Generic("Failed to extract entitlements.plist".to_string()));
        }

        println!("✅ Entitlements.plist extracted from embedded profile");
        Ok(())
    }

    //check if we already posess a provisioning profile for the target device
    pub fn is_device_provisioned(
        app_bundle_path: &PathBuf,
        device_id: &str,
        udid: &str,
        idp_path: &str,
    ) -> Result<bool, PistonError> {
        println!("Checking provisioning status...");

        // Look for ANY .mobileprovision in the bundle
        let entries = fs::read_dir(app_bundle_path)
            .map_err(|e| PistonError::ReadDirError { path: app_bundle_path.clone(), source: e })?;

        let mut profile_files = vec![];
        for entry in entries {
            let entry = entry.map_err(|e| PistonError::MapDirError{
                path: app_bundle_path.to_path_buf(),
                source: e,
        })?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".mobileprovision") {
                profile_files.push(entry.path());
            }
        }

        if profile_files.is_empty() {
            println!("No provisioning profile found in bundle");
            return Ok(false);
        }

        // Check each profile
        for profile_path in profile_files {
            let output = Command::new("security")
                .args(["cms", "-D", "-i", profile_path.to_str().unwrap()])
                .output()
                .map_err(|e| PistonError::QueryProvisionError{
                    path: profile_path.to_path_buf(),
                    source: e,
                })?;

            let xml = String::from_utf8_lossy(&output.stdout);

            if xml.contains(&format!("<string>{}</string>", udid)) {
                println!("✅ Device is in provisioning profile: {}", profile_path.display());

                // Check if it's installed on the device
                let list = Command::new(idp_path)
                    .args(["list", "--udid", udid])
                    .output();
                let list_res = list.unwrap();
                if !list_res.status.success() {
                    return Err(PistonError::Generic(format!("Failed to list provisioning profiles with IDP")))
                }
                let installed = String::from_utf8_lossy(&list_res.stdout);
                if installed.contains(profile_path.file_name().unwrap().to_str().unwrap()) {
                    return Ok(true);
                }
            }
        }

        println!("Device is NOT provisioned in any profile");
        Ok(false)
    }

    //sign an ios app bundle
    pub fn sign_app_bundle(app_bundle_path: &PathBuf, security_profile: &str) -> Result<(), PistonError> {
        let app_bundle = app_bundle_path.display().to_string();
        println!("Signing bundle: {}", app_bundle);
        // Delete any old signature so we can re-sign after provisioning added the profile
        let code_signature_dir = format!("{}/_CodeSignature", app_bundle);
        let code_signature_path = Path::new(&code_signature_dir);
        if code_signature_path.exists() {
            let _ = fs::remove_dir_all(&code_signature_path);
        }
        AscClient::ensure_entitlements(&app_bundle_path)?;
        let output = Command::new("codesign")
            .args([
                "--force",
                "--sign", security_profile,
                "--entitlements", &format!("{}/entitlements.plist", app_bundle),
                "--timestamp",
                "--options", "runtime",
                "--deep",
                &app_bundle,
            ])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| PistonError::CodesignError(e.to_string()))?;

        if !output.status.success() {
            return Err(PistonError::CodesignError(
                String::from_utf8_lossy(&output.stderr).trim().to_string()
            ));
        }

        println!("✅ Bundle signed successfully");
        Ok(())
    }
}

