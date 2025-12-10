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
}

impl IOSBuilder {
    pub fn new() -> Self {
    println!("building for IOS");
    //>>prebuild
    //-check for signing certificate
    //setup the app bundle

    //>>build

    //>>Postbuild
    //move binary to the app bundle and sign
    IOSBuilder{release: false, target: "target".to_string()}
    }
}

struct IOSRunner{
device: String, 
}

impl IOSRunner {
    fn new() -> Self{
        println!("running for IOS");
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