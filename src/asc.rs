use serde_json::json;
use serde::{ Serialize, Deserialize };
use std::collections::{ HashMap, HashSet };
use std::fs;
use std::path::PathBuf;
use std::process::{ Command, Stdio };
use ureq::Response;
use base64::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};
use std::io::Write;
use crate::PistonError;


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
    pub api_key: Option<AscApiKey>,
    pub keystore_path: String,
}

impl AscClient {
    //load the cached security certificate identity
    fn load_cert_cache(&self, cache_dir: &PathBuf) -> Option<(String, String)> {
        let path = cache_dir.join("cert_cache.json");
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
    fn save_cert_cache(&self, cert_id: &str, signing_identity: &str, cache_dir: &PathBuf) {
        let _ = fs::create_dir_all(&cache_dir);
        let data = json!({
            "cert_id": cert_id,
            "signing_identity": signing_identity
        });
        println!("SAVING TO CACHE: {:?}", data);
        let _ = fs::write(cache_dir.join("cert_cache.json"), data.to_string());
    }

    //parse team id from a security find-identity result
    fn parse_team_id(&self, stdout: &str, cert_name: &str) -> Result<String, PistonError> {
        let mut team_ids_for_cert: HashSet<String> = HashSet::new();

        for line in stdout.lines() {
            // Extract the quoted part: everything inside the "..."
            if let (Some(start), Some(end)) = (line.find('"'), line.rfind('"')) {
                if start < end {
                    let quoted_content = &line[start + 1..end];

                    // Inside the quotes: "certificate_name (TEAM_ID)"
                    if let Some(paren_pos) = quoted_content.rfind(" (") {
                        let name_part = quoted_content[..paren_pos].trim();
                        let rest = &quoted_content[paren_pos + 2..]; // after " ("

                        if let Some(close_pos) = rest.find(')') {
                            let potential_team_id = &rest[..close_pos];

                            if name_part == cert_name {
                                team_ids_for_cert.insert(potential_team_id.to_string());
                            }
                        }
                    }
                }
            }
        }

        // New logic you asked for:
        let team_id = match team_ids_for_cert.len() {
            1 => team_ids_for_cert.into_iter().next().unwrap(),
            0 => {
                return Err(PistonError::Generic(format!(
                    "No certificate matching the name '{}' was found in the keychain.",
                    cert_name
                )));
            }
            _ => {
                // Only error when the same name has DIFFERENT team IDs
                return Err(PistonError::Generic(format!(
                    "Multiple certificates with the exact same name '{}' were found but with different Team IDs: {:?}. \
                    Team ID cannot be uniquely determined. Please designate a team ID in your .env with 'team_id=<team_id>' ",
                    cert_name, team_ids_for_cert
                )));
            }
        };
        return Ok(team_id)
    }

    // Creates or re-uses a universal security certificate
    // Returns: (certificate_id, signing_identity_name) — name is normalized for codesign, ASC API returns something unique
    pub fn create_or_find_security_cert(
        &self,
        team_id_override: Option<String>,
    ) -> Result<(String, String), PistonError> {
        let cert_type = "DISTRIBUTION";
        let id_type = "Apple Distribution";

        let cache_dir = PathBuf::from("target/asc-cache");
        //1. Check cache
        if let Some((cert_id, signing_identity)) = self.load_cert_cache(&cache_dir) {
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
        //2. Unlock keychain
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
        
        //reuse existing certificate if possible
        if let Some(existing) = json["data"].as_array().and_then(|arr| arr.first()) {
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

            if output.contains(&cert_name){
                println!("✅ Certificate also found in local keychain → reusing");
                //parse the team id from the local security profiles in the keychain if override is not set
                let team_id = if team_id_override.is_none() {self.parse_team_id(&output, &cert_name)?} else {team_id_override.unwrap()};
                //cache the security credentials locally
                self.save_cert_cache(&team_id, &cert_name, &cache_dir);
                return Ok((team_id, cert_name));
            } else {
                println!("⚠️  Certificate exists in ASC but missing locally → creating a new one");
                // No automatic revocation, we just create a fresh certificate (Apple allows multiples)
            }
        }

        //3. CREATE NEW SECURITY CERTIFICATE
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
                endpoint: "upload CSR: https://api.appstoreconnect.apple.com/v1/certificates".to_string(),
                e: format!("Upload failed: {}", e),
            })?;

        let json: serde_json::Value = create_resp.into_json()
            .map_err(|e| PistonError::IntoJSONError(e.to_string()))?;

        println!("JSON RESP: {:?}", json);

        let cert_name = json["data"]["attributes"]["name"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string();

        // Download + decode
        let cert_b64 = json["data"]["attributes"]["certificateContent"]
            .as_str()
            .ok_or_else(|| PistonError::Generic("No certificateContent returned".to_string()))?;


        let cert_der = BASE64_STANDARD
            .decode(cert_b64)
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

        // Parse the Team ID from the newly uploaded certificate
        let check = Command::new("security")
            .args(["find-identity", "-v", "-p", "codesigning"])
            .output()
            .map_err(|e| PistonError::KeyChainImportError(format!("Failed to check keychain: {}", e)))?;
        let output = String::from_utf8_lossy(&check.stdout);
        let team_id = if team_id_override.is_none() {self.parse_team_id(&output, &cert_name)?} else {team_id_override.unwrap()};

        // Cleanup
        let _ = fs::remove_file(key_path);
        let _ = fs::remove_file(csr_path);
        let _ = fs::remove_file(cer_path);

        println!("✅ New {} certificate created and imported (TEAM_ID: {}, CERT_NAME: {})", id_type, team_id, cert_name);
        //cache the security credentials locally
        self.save_cert_cache(&team_id, &cert_name, &cache_dir);
        Ok((team_id, cert_name))
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

    //Registers device if needed → creates/re-uses profile → 
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

        
        let profile_data = BASE64_STANDARD
            .decode(b64)
            .map_err(|e| PistonError::Base64DecodeError(format!("Decode failed: {}", e)))?;

        let profile_path = format!("{}.mobileprovision", profile_name);
        fs::write(&profile_path, profile_data)
            .map_err(|e| PistonError::WriteFileError(format!("Failed to write profile: {}", e)))?;

        //cache mobile provision locally
        let cache_dir = PathBuf::from("target/ios-cache");
        let cache_dir_pro = cache_dir.join("profiles");
        let _ = fs::create_dir_all(&cache_dir_pro);
        let cached_profile = cache_dir_pro.join(format!("{}.mobileprovision", profile_name));
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

        let _ = Command::new("xattr")
            .args(["-cr", app_bundle_path.to_str().unwrap()])
            .status()
            .map_err(|e| PistonError::Generic(format!("xattr error: {}", e)))?;

        // Cleanup
        let _ = fs::remove_file(profile_path);

        println!("✅ Provisioning complete for '{}' → entitlements.plist ready", app_name);
        Ok(())
    }

    //TODO add support for distribution entitlement capabilities
    // Always extracts entitlements.plist from the embedded.mobileprovision in the bundle
    // Works whether we just provisioned or are reusing a cached profile
    pub fn ensure_entitlements(app_bundle_path: &PathBuf) -> Result<(), PistonError> {
        println!("ENSURING ENTITLEMENTS for: {:?}", app_bundle_path.display());
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

    //sign an ios or macos app bundle for App Store distribution
    pub fn sign_app_bundle(
        app_name: &str,
        app_bundle_path: &PathBuf,
        security_profile: &str,
        ios: bool,
        external: bool,
    ) -> Result<(), PistonError> {
        let bundle_path = app_bundle_path.display().to_string();
        println!("🔏 Signing {} bundle: {} with security profile: {}", if ios { "iOS" } else { "macOS" }, bundle_path, security_profile);

        // Remove any old signature
        let code_signature_dir = app_bundle_path.join("_CodeSignature");
        if code_signature_dir.exists() {
            let _ = fs::remove_dir_all(&code_signature_dir);
        }

        let path = if ios {app_bundle_path.clone()} else {app_bundle_path.clone().join("Contents")};
        AscClient::ensure_entitlements(&path)?;

        if ios {
            // ==================== iOS SIGNING (two-step) ====================
            println!("   → Signing iOS executable...");

            let status = Command::new("codesign")
                .args([
                    "--force",
                    "--sign", security_profile,
                    "--entitlements", &format!("{}/entitlements.plist", path.display()),
                    "--options", "runtime",
                    "--timestamp",
                    "--deep",
                    "--generate-entitlement-der",
                    &app_bundle_path.join(app_name).display().to_string(),
                ])
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .map_err(|e| PistonError::CodesignError(e.to_string()))?;

            if !status.success() {
                return Err(PistonError::CodesignError("Failed to sign iOS executable".to_string()));
            }
        }

        // ==================== COMMON BUNDLE SIGNING ====================
        println!("   → Signing outer bundle...");

        let entitlements_path = format!("{}/entitlements.plist", path.display());

        let mut args = vec![
            "--force",
            "--sign", security_profile,
            "--entitlements", &entitlements_path,
            "--timestamp",
            "--deep",
            "--generate-entitlement-der",
            "--verbose",
            &bundle_path,
        ];

        // macOS App Store builds should NOT use --options runtime
        if ios || external {
            args.insert(5, "--options");
            args.insert(6, "runtime");
        }

        let status = Command::new("codesign")
            .args(&args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| PistonError::CodesignError(e.to_string()))?;

        if !status.status.success() {
            return Err(PistonError::CodesignError(
                String::from_utf8_lossy(&status.stderr)
                    .trim()
                    .to_string(),
            ));
        }

        println!("✅ {} bundle signed successfully!", if ios { "iOS" } else { "macOS" });
        Ok(())
    }
}