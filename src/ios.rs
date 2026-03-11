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
use crate::Helper;
use crate::PistonError;
use crate::devices::IOSDevice;

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use ureq::Response;


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
    dev_name: Option<String>,
    device_target: Option<IOSDevice>,
    idp_path: Option<String>,
    apple_cer: Option<String>,
    keystore_path: Option<String>,
}

impl IOSBuilder {

    pub fn start(release: bool, target: String, cwd: PathBuf, env_vars: HashMap<String, String>, device_target: Option<IOSDevice>) -> Result<(), PistonError> {
        println!("building for iOS");
        //check operating system (requires MacOS)
        if std::env::consts::OS != "macos"{
            return Err(PistonError::UnsupportedOSError{os: std::env::consts::OS.to_string(), target: target})
        }
        //TODO check for signing certificate & sign?

        let mut op = IOSBuilder::new(release, target, cwd, env_vars, device_target)?;
        //>>prebuild
        op.pre_build()?;

        //>>build
        op.build()?;

        //>>Postbuild
        op.post_build()?;

        Ok(())
    }

    fn new(release: bool, target: String, cwd: PathBuf, env_vars: HashMap<String, String>, device_target: Option<IOSDevice>) -> Result<Self, PistonError> {
        println!("creating IOSBuilder: release: {:?}, target: {:?}, cwd: {:?}", release, target.to_string(), cwd);
        //parse env vars
        let cargo_path = env_vars.get("cargo_path").cloned().unwrap_or("cargo".to_string());
        let idp_path = env_vars.get("idp_path").cloned();
        let apple_cer = env_vars.get("apple_cer").cloned();
        let dev_name = env_vars.get("dev_name").cloned();
        println!("dev name: {:?}", dev_name);
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
            dev_name: dev_name,
            device_target: device_target, 
            idp_path: idp_path,
            apple_cer: apple_cer,
            keystore_path: keystore_path,
        })
    }

    fn pre_build(&mut self) -> Result <(), PistonError>{
        //TODO check xcode for updates?
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
        Helper::empty_directory(path)?;
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
        let plist_content = format!(
            r#"
            <?xml version="1.0" encoding="UTF-8"?>
            <!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN"
            "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
            <plist version="1.0">
            <dict>
                <key>CFBundleName</key>
                <string>{}</string>
                <key>CFBundleIdentifier</key>
                <string>{}</string>
                <key>CFBundleInfoDictionaryVersion</key>
                <string>6.0</string>
                <key>CFBundleExecutable</key>
                <string>{}</string>
                <key>CFBundlePackageType</key>
                <string>AAPL</string>
                <key>CFBundleShortVersionString</key>
                <string>{}</string>
                <key>CFBundleVersion</key>
                <string>{}</string>
                <key>LSRequiresIphoneOS</key>
                <true/>
                <key>MinimumOSVersion</key>
                <string>{}</string>
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
            &self.app_version.clone(),
            &self.app_version.clone(),
            &self.min_os_version.clone(),
        );
        plist_file.write_all(plist_content.as_bytes()).map_err(|e| PistonError::WriteFileError(e.to_string()))?;
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

        //TODO check for apple signing certificate
        if self.keystore_path.is_none() || self.dev_name.is_none(){
            println!("Keystore path or developer name missing from .env, skipping automated signing");
        } else {
            println!("keystore path provided");
            let asc_client = AscClient{ api_key: self.asc_api_key.clone(), keystore_path: self.keystore_path.clone().unwrap()};
            //TODO error check this result
            asc_client.create_or_find_security_certificate(self.dev_name.clone().unwrap(), self.apple_cer.clone(), true)?;


            //check if the user provided signing certificate exists and is valid for the requested operation
            //otherwise create one if api access is avaialble and create a signing certificate

            //if the user includes the release flag in the build, we must assume an IOS distirbution cert is needed
            //AKA IOS_APP_ADHOC profile type cert 
            // If not flagging for release we should assume IOS Development certificate is needed
            //AKA IOS_APP_DEVELOPMENT profile type cert

            //see AscClient Struct and methods

            //if a device target is provided, check if the target device is provisioned
            if !self.device_target.is_none() {
                println!("device target exists, checking for existing provisioning");
                let output_path = self.output_path.clone().unwrap();
                let target_id = self.device_target.clone().unwrap().id;
                let target_udid = self.device_target.clone().unwrap().udid;
                let idp_path = self.idp_path.clone().unwrap();
                let provisioned = Provision::is_device_provisioned(&output_path, &target_id, &target_udid, &idp_path)?;
                //if device is not provisioned and api access is available, attempt to provision
                if provisioned == false && !self.asc_api_key.is_none() {
                    println!("attempting to provision target device {:?}", self.device_target);
                    //TODO provision device here
                }
            }

        }
        println!("done configuring ios bundle");
        Ok(())
        
    }

    fn build(&mut self) -> Result <(), PistonError>{
        println!("build for ios");
        //build the binary for the specified target
        let cargo_args = format!("build --target {} {}", self.target, if self.release {"--release"} else {""});
        let cargo_cmd = format!("{} {}", self.cargo_path, cargo_args);
        Command::new("bash")
            .arg("-c")
            .arg(&cargo_cmd)
            .current_dir(self.cwd.clone())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| PistonError::BuildError(format!("Cargo build failed: {}", e)))?;

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
        //output the proper location in the terminal for the user to see 
        println!("iOS app bundle available at: {}", &bundle_path.display());

        //TODO sign the bundle

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

        let builder = IOSBuilder::start(release, target_string, cwd, env_vars, Some(device.clone()))?;

        //need to pass in output dir, bundle id, 
        let runner = IOSRunner::deploy_usb(device)?;

        //>>postbuild
        //sign app bundle
        //deploy installation and run on target device

        Ok(())
    }

    fn deploy_usb(device: &IOSDevice) -> Result<(), PistonError> {
        //TODO
        // let output = Command::new("xcrun")
        //     .args(["devicectl", "device", "install", "app", "--device", &device.id.clone(), &format!("{}/{}/ios/{}.app", session.projects_path.as_ref().unwrap(), session.current_project.as_ref().unwrap(), capitalize_first(session.current_project.as_ref().unwrap()))])
        //     .output()
        //     .unwrap();
        // println!("Deploying bundle id: {} to device: {}", &bundle_id, &device.id);
        // let output = Command::new("xcrun")
        //     .args(["devicectl", "device", "process", "launch", "--device", &device.id.clone(), &bundle_id])
        //     .output()
        //     .unwrap();
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

    // Creates a new iOS security certificate (or re-uses if one already exists for this machine)
    // Returns: (certificate_id, signing_identity_name)
    pub fn create_or_find_security_certificate(
        &self,
        dev_name: String,
        apple_cer: Option<String>,
        distribution: bool,
    ) -> Result<(String, String), PistonError> {
        println!("checking for existing security certificates");
        // Trigger macOS unlock prompt (blocks until user unlocks or cancels)
        let keychain_path = format!("{}/login.keychain-db", self.keystore_path.clone());

        let unlock_result = Command::new("security")
            .args(["unlock-keychain", &keychain_path])
            .output();

        if let Err(e) = unlock_result {
            return Err(PistonError::KeyChainUnlockError(format!("Failed to unlock keychain: {}", e)));
        }
        //(fails fast if user cancelled)
        let status = Command::new("security")
            .args(["show-keychain-info", &keychain_path])
            .output()
            .map_err(|e| PistonError::KeyChainUnlockError(format!("Failed to check keychain status: {}", e)))?;

        if String::from_utf8_lossy(&status.stdout).contains("locked") {
            return Err(PistonError::KeyChainUnlockError("Keychain is still locked. User cancelled the unlock prompt.".to_string()));
        }
        println!("✅ Keychain unlocked");

        //TODO if the user has specified a signing certificate in the .env, verify this is valid and use it
        if apple_cer.is_none() {
            println!("apple cer provided: {:?}", apple_cer);
        }
        else{
            println!("no apple cer provided in the .env");
        }

        //TODO otherwise create a signing certificate
        let token = self.generate_jwt()?;
        let certificate_type = if distribution { "IOS_DISTRIBUTION" } else { "IOS_DEVELOPMENT" };

        // 1. Check if we already have a valid development certificate in ASC
        println!("checking for valid development certificate in ASC");
        let list_resp: Response = ureq::get("https://api.appstoreconnect.apple.com/v1/certificates")
            .set("Authorization", &format!("Bearer {}", token))
            .query("filter[certificateType]", certificate_type)
            .call()
            .map_err(|e| PistonError::ASCClientUreqError{
                endpoint: "https://api.appstoreconnect.apple.com/v1/certificates".to_string(),
                e:  format!("Existing security certificate get failed: {}", e),
            })?;
        println!("security certificates list: {:?}", list_resp);
        let json: serde_json::Value = list_resp.into_json().map_err(|e| PistonError::IntoJSONError(e.to_string()))?;
        println!("security certificates list in JSON: {:?}", &json);
        if let Some(existing) = json["data"].as_array().and_then(|arr| arr.first()) {
            let cert_id = existing["id"].as_str().unwrap().to_string();
            //TODO improve this
            let identity = format!("Apple Development: {:?} ({})", dev_name, "YOUR_TEAM_ID");
            println!("✅ Re-using existing development certificate (ID: {})", cert_id);
            return Ok((cert_id, identity));
        }

        // 2. Generate private key + CSR using openssl
        println!("generating private key using Openssl");
        let key_path = "temp_dev_key.p8";
        let csr_path = "temp_dev_csr.csr";

        std::process::Command::new("openssl")
            .args(["genpkey", "-algorithm", "RSA", "-out", key_path, "-pkeyopt", "rsa_keygen_bits:2048"])
            .output()
            .map_err(|e| PistonError::OpenSSLKeyGenError(format!("openssl keygen failed: {}", e)))?;

        println!("generating CSR using openssl");
        println!("dev name: {:?}", dev_name);
        std::process::Command::new("openssl")
            .args([
                "req", "-new", "-key", key_path,
                "-out", csr_path,
                "-subj", &format!("/CN={}", dev_name),
            ])
            .output()
            .map_err(|e| PistonError::OpenSSLCSRError(format!("openssl csr failed: {}", e)))?;

        let csr_content = fs::read_to_string(csr_path)
            .map_err(|e| PistonError::ReadCSRError(format!("Failed to read CSR: {}", e)))?;

        // 3. Upload CSR to ASC
        println!("uploading CSR to appstoreconnect API");
        let body = json!({
            "data": {
                "type": "certificates",
                "attributes": {
                    "certificateType": "IOS_DEVELOPMENT",
                    "csrContent": csr_content
                }
            }
        });

        let create_resp: Response = ureq::post("https://api.appstoreconnect.apple.com/v1/certificates")
            .set("Authorization", &format!("Bearer {}", token))
            .set("Content-Type", "application/json")
            .send_json(&body)
            .map_err(|e| PistonError::ASCClientUreqError{
                endpoint: "https://api.appstoreconnect.apple.com/v1/certificates".to_string(),
                e:  format!("CSR Upload failed: {}", e),
            })?;

        let json: serde_json::Value = create_resp.into_json().map_err(|e| PistonError::IntoJSONError(e.to_string()))?;
        let cert_id = json["data"]["id"].as_str().unwrap().to_string();

        // 4. Download the signed certificate
        println!("downloading signed certificate");
        let cert_b64 = json["data"]["attributes"]["certificateContent"]
            .as_str()
            .ok_or_else(|| PistonError::Generic("No certificateContent returned".to_string()))?;

        let cert_der = base64::decode(cert_b64).map_err(|e| PistonError::Base64DecodeError(format!("Base64 decode failed: {}", e)))?;
        let cer_path = "temp_dev_cert.cer";
        fs::write(&cer_path, cert_der).map_err(|e| PistonError::WriteFileError(format!("Base64 decode failed: {}", e)))?;

        // 5. Import into macOS keychain (private key + cert)
        println!("importing security certificate into MacOS keychain");
        std::process::Command::new("security")
            .args(["import", cer_path, "-k", "login.keychain", "-T", "/usr/bin/codesign"])
            .output()
            .map_err(|e| PistonError::KeyChainImportError(format!("security import failed: {}", e)))?;

        // Clean up temp files
        let _ = fs::remove_file(key_path);
        let _ = fs::remove_file(csr_path);
        let _ = fs::remove_file(cer_path);

        let signing_identity = format!("Apple Development: {} (Team ID will be auto-detected)", dev_name);
        println!("✅ New development certificate created (ID: {})", cert_id);

        Ok((cert_id, signing_identity))
    }

    //TODO this
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
            .map_err(|e| PistonError::ASCClientParseEncodingKeyError(format!("Invalid .p8 key: {}", e)))?;

        jsonwebtoken::encode(&header, &claims, &key).map_err(|e| PistonError::ASCClientJWTEncodeError(e.to_string()) )
    }

    //TODO this
    pub fn register_ios_device(&self, ios_device: &IOSDevice) -> Result<String, PistonError> {
        let token = self.generate_jwt()?;

        let body = json!({
            "data": {
                "type": "devices",
                "attributes": {
                    "name": &ios_device.model,
                    "udid": &ios_device.udid,
                    "platform": "IOS"
                }
            }
        });

        let resp: Response = ureq::post("https://api.appstoreconnect.apple.com/v1/devices")
            .set("Authorization", &format!("Bearer {}", token))
            .set("Content-Type", "application/json")
            .send_json(&body)
            .map_err(|e| PistonError::ASCClientUreqError{
                endpoint: "https://api.appstoreconnect.apple.com/v1/devices".to_string(),
                e: format!("Device Registration failed: {}", e),
            })?;

        let json: serde_json::Value = resp.into_json().map_err(|e| PistonError::IntoJSONError(e.to_string()))?;
        let resource_id = json["data"]["id"].as_str().unwrap_or("").to_string();

        println!("✅ Device registered in ASC (resource ID: {})", resource_id);
        Ok(resource_id)
    }

    //TODO this
    pub fn find_or_create_bundle_id(&self, bundle_id: &str, name: &str) -> Result<String, PistonError> {
        let token = self.generate_jwt()?;

        let search: Response = ureq::get("https://api.appstoreconnect.apple.com/v1/bundleIds")
            .set("Authorization", &format!("Bearer {}", token))
            .query("filter[identifier]", bundle_id)
            .call()
            .map_err(|e| PistonError::ASCClientUreqError{
                endpoint: "https://api.appstoreconnect.apple.com/v1/bundleIds".to_string(),
                e: format!("Find bundle ID failed: {}", e),
            })?;

        let json: serde_json::Value = search.into_json().map_err(|e| PistonError::IntoJSONError(e.to_string()))?;
        if let Some(first) = json["data"].as_array().and_then(|a| a.first()) {
            return Ok(first["id"].as_str().unwrap().to_string());
        }

        let body = json!({
            "data": {
                "type": "bundleIds",
                "attributes": {
                    "identifier": bundle_id,
                    "name": name,
                    "platform": "IOS"
                }
            }
        });

        let resp: Response = ureq::post("https://api.appstoreconnect.apple.com/v1/bundleIds")
            .set("Authorization", &format!("Bearer {}", token))
            .set("Content-Type", "application/json")
            .send_json(&body)
            .map_err(|e| PistonError::ASCClientUreqError{
                endpoint: "https://api.appstoreconnect.apple.com/v1/bundleIds".to_string(),
                e:format!("BundleID creation failed: {}", e),
            })?;

        let json: serde_json::Value = resp.into_json().map_err(|e| PistonError::IntoJSONError(e.to_string()))?;
        Ok(json["data"]["id"].as_str().unwrap().to_string())
    }

    //TODO This
    pub fn create_development_profile(
        &self,
        bundle_resource_id: &str,
        certificate_id: &str,      // still needed once (we can automate later)
        device_resource_id: &str,
        profile_name: &str,
    ) -> Result<String, PistonError> {
        let token = self.generate_jwt()?;

        let body = json!({
            "data": {
                "type": "profiles",
                "attributes": { "name": profile_name, "profileType": "IOS_APP_DEVELOPMENT" },
                "relationships": {
                    "bundleId": { "data": { "type": "bundleIds", "id": bundle_resource_id } },
                    "certificates": { "data": [{ "type": "certificates", "id": certificate_id }] },
                    "devices": { "data": [{ "type": "devices", "id": device_resource_id }] }
                }
            }
        });

        let create_resp: Response = ureq::post("https://api.appstoreconnect.apple.com/v1/profiles")
            .set("Authorization", &format!("Bearer {}", token))
            .set("Content-Type", "application/json")
            .send_json(&body)
            .map_err(|e| PistonError::ASCClientUreqError{
                endpoint: "https://api.appstoreconnect.apple.com/v1/profiles".to_string(),
                e: format!("Profile creation failed: {}", e),
            })?;

        let json: serde_json::Value = create_resp.into_json().map_err(|e| PistonError::IntoJSONError(e.to_string()))?;
        let profile_id = json["data"]["id"].as_str().unwrap().to_string();

        let get_resp: Response = ureq::get(&format!(
            "https://api.appstoreconnect.apple.com/v1/profiles/{}",
            profile_id
        ))
        .set("Authorization", &format!("Bearer {}", token))
        .call()
        .map_err(|e| PistonError::ASCClientUreqError {
            endpoint: format!("https://api.appstoreconnect.apple.com/v1/profiles/{}",profile_id),
            e: e.to_string(),
        })?;

        let profile_json: serde_json::Value = get_resp.into_json().map_err(|e| PistonError::IntoJSONError(e.to_string()))?;
        let b64 = profile_json["data"]["attributes"]["profileContent"]
            .as_str()
            .ok_or_else(|| PistonError::Generic("No profileContent returned".to_string()))?;

        let decoded = base64::decode(b64).map_err(|e| PistonError::Base64DecodeError(format!("Base64 decode failed: {}", e)))?;

        let profile_path = format!("{}.mobileprovision", profile_name.replace(' ', "-"));
        fs::write(&profile_path, decoded).map_err(|e| PistonError::WriteFileError(e.to_string()))?;

        println!("✅ Profile saved → {}", profile_path);
        Ok(profile_path)
    }
}

pub struct Provision {}

impl Provision {
    //TODO make sure this can handle multiple provision profiles
    pub fn is_device_provisioned(app_bundle_path: &PathBuf, device_id: &str, udid: &str, idp_path: &str) -> Result<bool, PistonError> {
        println!("checking if target device is properly provisioned");
        //obtain the mobile provision file name
        let mobileprovision_file: String;
        let entries = fs::read_dir(app_bundle_path)
                .map_err(|e| PistonError::ReadDirError {
            path: app_bundle_path.to_path_buf(),
            source: e,
        })?;

            let mut matching_files = Vec::new();
            for entry in entries {
                let entry = entry.map_err(|e| PistonError::MapDirError {
                    path: app_bundle_path.to_path_buf(),
                    source: e,
                })?;
                let file_name = entry.file_name().to_string_lossy().into_owned();
                if file_name.ends_with(".mobileprovision") {
                    matching_files.push(file_name);
                }
            }

            match matching_files.len() {
                0 => {
                    println!("No provisioning profile found");
                    return Ok(false);
                }
                1 => {
                    println!("Exactly one provisioning profile found");
                    mobileprovision_file = matching_files[0].clone();
                }
                _ => {
                    println!("something weird happened finding the provisioning profile");
                    return Ok(false);
                }
            }
        // 
        
        let profile_path_str = format!("{}/{}", &app_bundle_path.display(), &mobileprovision_file);
        let profile_path = Path::new(&profile_path_str);
        //query the mobile provision profile
        //excute command `security cms -D -i /output/path/to/app/bundle/file.mobileprovision`
        let output = Command::new("security")
            .arg("cms")
            .arg("-D")
            .arg("-i")
            .arg(profile_path)
            .output()
            .map_err(|e| PistonError::QueryProvisionError {
                path: profile_path.to_path_buf(),
                source: e,
            })?;
        if !output.status.success() {
            return Err(PistonError::Generic(format!("security command failed")));
        }
        //check for an existing device provision
        let xml = String::from_utf8(output.stdout)
            .map_err(|e| PistonError::ParseUTF8Error(format!("error parsing xml from mobile provision")))?;
        let key_str = "<key>ProvisionedDevices</key>";
        let Some(key_pos) = xml.find(key_str) else {
            println!("provision profile does not contain valid syntax: <key>ProvisionedDevice</key>");
            return Ok(false);
        };
        let start_after_key = key_pos + key_str.len();
        let rest_after_key = &xml[start_after_key..];
        let string_open = "<array>";
        let Some(array_pos) = rest_after_key.find(string_open) else {
            println!("provision profile does not contain valid syntax: <array>");
            return Ok(false);
        };
        let start_of_array = array_pos + string_open.len();
        let rest_after_open = &rest_after_key[start_of_array..];
        let string_close = "</array>";
        let Some(close_pos) = rest_after_open.find(string_close) else {
            println!("provision profile does not contain valid syntax: </array>");
            return Ok(false);
        };
        let array_content = &rest_after_open[..close_pos];
        let device_entry = format!("<string>{}</string>", udid);
        //check if the profile contains the device id
        println!("checking if array content: {:?} contains device entry: {:?}", &array_content, &device_entry);
        if array_content.contains(&device_entry) {
            println!("provisioning profile contains the device id...checking device for installation");
            //check that the profile is installed on the device
            if let Some(key_pos) = xml.find("<key>Name</key>") {
                if let Some(string_start) = xml[key_pos..].find("<string>") {
                    let start = key_pos + string_start + "<string>".len();
                    if let Some(string_end) = xml[start..].find("</string>") {
                        let profile_name = xml[start..start + string_end].trim().to_string();
                        if !profile_name.is_empty() {
                            //list the installed provisions
                            let output = Command::new(idp_path)
                                .args(["list", "--udid", udid])
                                .output();
                            let output_res = output.unwrap();
                            if !output_res.status.success() {
                                return Err(PistonError::Generic(format!("Failed to list provisioning profiles with IDP")));
                            }
                            let profiles = String::from_utf8_lossy(&output_res.stdout);
                            if profiles.contains(&profile_name) {
                                println!("target device is already provisioned");
                                return Ok(true)
                            }else{
                                println!("provisioning profile is not currently installed on the target device");
                                return Ok(false)
                            }
                        }
                    }
                }
            }
            return Err(PistonError::Generic("Name not found in provisioning profile".to_string()));
        } else {
            println!("target device is not provisioned");
            return Ok(false)
        }
    }
}

