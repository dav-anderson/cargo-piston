use std::path::{ PathBuf, Path };
use std::collections::HashMap;
use std::process::{ Command, Stdio };
use std::io::{ Write };
use cargo_metadata::{ Metadata, MetadataCommand };
use std::fs::{ copy,File, create_dir_all, remove_file };
use crate::Helper;
use crate::PistonError;

// use anyhow::{Context, Result};
// use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
// use openssl::asn1::Asn1Time;
// use openssl::bn::BigNum;
// use openssl::hash::MessageDigest;
// use openssl::nid::Nid;
// use openssl::pkey::{PKey, Private};
// use openssl::rsa::Rsa;
// use openssl::x509::extension::{BasicConstraints, KeyUsage, SubjectKeyIdentifier};
// use openssl::x509::{X509NameBuilder, X509ReqBuilder, X509Req};
// use reqwest::blocking::Client;
// use serde::{Deserialize, Serialize};
// use std::fs;
// use std::time::{Duration, SystemTime, UNIX_EPOCH};


pub struct IOSBuilder {
    release: bool,
    target: String,
    cwd: PathBuf,
    output_path: Option<PathBuf>,
    icon_path: Option<String>,
    cargo_path: String,
    app_name: String,
    app_version: String,
}

impl IOSBuilder {

    pub fn start(release: bool, target: String, cwd: PathBuf, env_vars: HashMap<String, String>) -> Result<(), PistonError> {
        println!("building for iOS");
        //check operating system (requires MacOS)
        if std::env::consts::OS != "macos"{
            return Err(PistonError::UnsupportedOSError{os: std::env::consts::OS.to_string(), target: target})
        }
        let mut op = IOSBuilder::new(release, target, cwd, env_vars)?;
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
        println!("creating IOSBuilder: release: {:?}, target: {:?}, cwd: {:?}", release, target.to_string(), cwd);
        //parse env vars
        let cargo_path = env_vars.get("cargo_path").cloned().unwrap_or("cargo".to_string());
        println!("Cargo path determined: {}", &cargo_path);
        //parse cargo.toml
        let metadata: Metadata = MetadataCommand::new()
            .current_dir(cwd.clone())
            .exec()
            .map_err(|e| PistonError::CargoParseError(e.to_string()))?;

        let icon_path = Helper::get_icon_path(&metadata);
        let app_name = Helper::get_app_name(&metadata)?;
        let app_version = Helper::get_app_version(&metadata)?;
        Ok(IOSBuilder{release: release, target: target.to_string(), cwd: cwd, output_path: None, icon_path: icon_path, cargo_path: cargo_path, app_name: app_name, app_version: app_version})
    }

    fn pre_build(&mut self) -> Result <(), PistonError>{
        //TODO check xcode for updates?
        //TODO check for libimobiledevice?

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
        //TODO dynamic bundle id and store in state
        let bundle_id = "placeholder.com";
        //populate the Info.plist file
        //TODO make min os version dynamic
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
                <string>17.5</string>
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
            &bundle_id,
            &self.app_name.clone(),
            &self.app_version.clone(),
            &self.app_version.clone(),
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
        Ok(())
    }

}

struct IOSRunner{
device: String, 
}

impl IOSRunner {

    pub fn start() -> Result<(), PistonError> {
        println!("running for IOS");
        if std::env::consts::OS != "macos"{
            println!("error cannot run mac on linux");
            // return Err(PistonError::UnsupportedOSError{os: std::env::consts::OS.to_string(), target: target})
        }
        Ok(())
    }

    fn new() -> Self{
        //>>prebuild
        //check for apple signing certificate
        //check target device for provisioning
        //provision device if required
        //setup the app bundle

        //>>build

        //>>postbuild
        //move binary to the app bundle and sign
        //deploy installation and run on target device
        IOSRunner{device: "device".to_string()}

    }
}

// // JWT Claims struct
// #[derive(Debug, Serialize, Deserialize)]
// struct Claims {
//     iss: String,  // Your issuer ID (team ID)
//     iat: u64,     // Issued at (current time)
//     exp: u64,     // Expiration (e.g., now + 20 min)
//     aud: String,  // "appstoreconnect-v1"
//     scope: Vec<String>,  // Optional scopes, e.g., ["GET /v1/apps"]
// }

// fn generate_jwt(private_key_pem: &str, key_id: &str, issuer_id: &str) -> Result<String> {
//     let private_key = EncodingKey::from_ec_pem(private_key_pem.as_bytes())?;  // Or from_rsa_pem if RSA
//     let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
//     let claims = Claims {
//         iss: issuer_id.to_string(),
//         iat: now,
//         exp: now + 1200,  // 20 minutes
//         aud: "appstoreconnect-v1".to_string(),
//         scope: vec!["POST /v1/certificates".to_string(), "GET /v1/certificates".to_string()],
//     };
//     let mut header = Header::new(Algorithm::ES256);  // ES256 for EC keys
//     header.kid = Some(key_id.to_string());
//     encode(&header, &claims, &private_key).context("Failed to encode JWT")
// }

// // Generate CSR
// fn generate_csr() -> Result<String> {
//     let rsa = Rsa::generate(2048)?;
//     let pkey = PKey::from_rsa(rsa)?;

//     let mut name_builder = X509NameBuilder::new()?;
//     name_builder.append_entry_by_nid(Nid::COMMONNAME, "Your Common Name")?;
//     let name = name_builder.build();

//     let mut req_builder = X509ReqBuilder::new()?;
//     req_builder.set_pubkey(&pkey)?;
//     req_builder.set_subject_name(&name)?;
//     req_builder.set_version(0)?;

//     let mut key_usage = KeyUsage::new();
//     key_usage.digital_signature().key_encipherment();
//     req_builder.add_extension(key_usage.build()?)?;

//     let mut basic_constraints = BasicConstraints::new();
//     basic_constraints.ca();
//     req_builder.add_extension(basic_constraints.build()?)?;

//     let subject_key_identifier = SubjectKeyIdentifier::new();
//     req_builder.add_extension(subject_key_identifier.from_pkey(&pkey)?)?;

//     req_builder.sign(&pkey, MessageDigest::sha256())?;

//     let csr = req_builder.build();
//     let csr_pem = csr.to_pem()?;
//     Ok(base64::encode(csr_pem))
// }

// #[derive(Serialize)]
// struct CertificateRequest {
//     data: CertificateData,
// }

// #[derive(Serialize)]
// struct CertificateData {
//     #[serde(rename = "type")]
//     type_: String,
//     attributes: CertificateAttributes,
// }

// #[derive(Serialize)]
// struct CertificateAttributes {
//     certificate_type: String,  // e.g., "IOS_DEVELOPMENT"
//     csr_content: String,
// }

// #[derive(Deserialize)]
// struct CertificateResponse {
//     data: CertificateResponseData,
// }

// #[derive(Deserialize)]
// struct CertificateResponseData {
//     id: String,
//     attributes: CertificateResponseAttributes,
// }

// #[derive(Deserialize)]
// struct CertificateResponseAttributes {
//     certificate_content: String,  // Base64-encoded DER
// }

// fn main() -> Result<()> {
//     let private_key_pem = fs::read_to_string("path/to/AuthKey_XXXXXXXXXX.p8")?;  // Your private key file
//     let key_id = "XXXXXXXXXX";  // Your API key ID
//     let issuer_id = "your-team-id";  // Your team/issuer ID

//     let jwt = generate_jwt(&private_key_pem, key_id, issuer_id)?;

//     let csr_base64 = generate_csr()?;

//     let client = Client::new();
//     let request_body = CertificateRequest {
//         data: CertificateData {
//             type_: "certificates".to_string(),
//             attributes: CertificateAttributes {
//                 certificate_type: "IOS_DEVELOPMENT".to_string(),  // Change as needed
//                 csr_content: csr_base64,
//             },
//         },
//     };

//     // Create certificate
//     let create_response: CertificateResponse = client
//         .post("https://api.appstoreconnect.apple.com/v1/certificates")
//         .header("Authorization", format!("Bearer {}", jwt))
//         .json(&request_body)
//         .send()?
//         .json()?;

//     let cert_id = create_response.data.id;

//     // Download certificate content (requires new JWT if expired)
//     let download_jwt = generate_jwt(&private_key_pem, key_id, issuer_id)?;  // Regenerate if needed
//     let download_response: CertificateResponse = client
//         .get(format!("https://api.appstoreconnect.apple.com/v1/certificates/{}/downloadCertificateContent", cert_id))
//         .header("Authorization", format!("Bearer {}", download_jwt))
//         .send()?
//         .json()?;

//     let cert_content_base64 = download_response.data.attributes.certificate_content;
//     let cert_der = base64::decode(cert_content_base64)?;
//     fs::write("certificate.cer", cert_der)?;

//     println!("Certificate downloaded to certificate.cer");
//     Ok(())
// }