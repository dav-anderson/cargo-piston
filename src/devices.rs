use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use regex::Regex;
use crate::error::PistonError;

#[derive(Debug)]
pub struct IOSDevice {
id: String,
udid: String,
unlocked: bool,
provisioned: bool,
}

#[derive(Debug)]
pub struct Devices {
    ios: Vec<IOSDevice>,
    android: Vec<String>,
}

//TODO replace all .expects with PistonErrors
//TODO error handle all unwraps

impl Devices {
    pub fn start(env_vars: HashMap<String, String>) -> Self {
        let mut devices = Devices {
            ios: Vec::new(),
            android: Vec::new(),
        };

        let sdk_path: Option<String> = env_vars.get("sdk_path").cloned();
        let adb_path = format!("{}/platform-tools/adb", sdk_path.unwrap());
        if Path::new(&adb_path).exists() {
            devices.populate_android(adb_path);
        }else{
            println!("Android installation not found");
        }
        if std::env::consts::OS == "macos" {
            devices.populate_ios();
        }
        devices.print_devices();
        devices
    }

    pub fn populate_android(&mut self, adb_path: String) {
        let output = Command::new(adb_path).arg("devices").output().expect("faield to execute adb devices command. Ensure ADB is installed");

        //convert the output to utf8
        let stdout = str::from_utf8(&output.stdout).expect("failed to parse ADB output as utf8");

        //split the output into lines
        let lines: Vec<&str> = stdout.lines().collect();

        //Skip the first line ("list of devices attached") and process the results
        for line in lines.iter().skip(1) {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() == 2 && parts[1] == "device" {
                    //add the device serial to the vector
                    self.android.push(parts[0].to_string())
                }
            }
        }

    }

    pub fn populate_ios(&mut self) {
        //obtain device ids from devicectls
        //execute `xcrun devicectl list devices`
        let output_devicectl = Command::new("xcrun")
            .args(["devicectl", "list", "devices"])
            .stdout(Stdio::piped())
            .output()
            .expect("failed to execute `xcrun devicectl list devices` command. Ensure libimobile devices is installed");


        //convert the output to a utf8
        let stdout_devicectl = str::from_utf8(&output_devicectl.stdout);
        let re = Regex::new(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}").unwrap();
        let ids: Vec<&str> = re.find_iter(&stdout_devicectl.unwrap()).map(|m| m.as_str()).collect();

        //get UDIDs from xctrace
        let output_xctrace = Command::new("xcrun")
            .args(["xctrace", "list", "devices"])
            .stdout(Stdio::piped())
            .output()
            .expect("Failed to run xctrace command");

        let stdout_xctrace = String::from_utf8(output_xctrace.stdout);
        //TODO error handle this
        let binding = stdout_xctrace.unwrap();
        let lines: Vec<&str> = binding.lines().collect();

        let device_target = "iphone";
    
        let pattern = r"(?i)^iPhone\s+\([^)]+\)\s+\(([0-9a-f]{8}-[0-9a-f]{16})\)";
        let re_xctrace = Regex::new(pattern).unwrap();

        let mut udids: Vec<String> = Vec::new();
        for line in lines {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                if let Some(captures) = re_xctrace.captures(trimmed) {
                    if let Some(udid_match) = captures.get(1) {
                        let udid_str = udid_match.as_str();
                        if device_target.to_lowercase() == "iphone" && udid_str.len() == 25 {
                            udids.push(udid_str.to_string());
                        }
                    }
                }
            }
        }

        // Pair them assuming same order
        let min_len = ids.len().min(udids.len());
        if ids.len() != udids.len() {
            eprintln!("Warning: Number of devices from devicectl ({}) and xctrace ({}) do not match. Using {} devices.", ids.len(), udids.len(), min_len);
        }
        for i in 0..min_len {
            self.ios.push(IOSDevice {
                id: ids[i].to_string().clone(),
                udid: udids[i].clone(),
                unlocked: false,
                provisioned: false,
            });
        }
    }

        //TODO deprecated can remove
        // Add each matching UUID to the ios vector
        // for uuid in uuids {
        //     self.ios.push(uuid.to_string());
        // }

    pub fn print_devices(&self) {
        if self.ios.is_empty() && self.android.is_empty() {
            println!();
            println!("Cargo Piston Device List:");
            println!();
            println!("No devices connected");
        } else {
            println!();
            println!("Cargo Piston Device List:");
            println!();
            if !self.android.is_empty() {
                println!("Android:");
                for (index, device) in self.android.iter().enumerate() {
                    println!();
                    println!("Device {}:", index + 1);
                    println!("{}", device);
                }
                if !self.ios.is_empty() {
                    println!();

                }
            }
            if !self.ios.is_empty() {
                println!("iOS:");
                for (index, device) in self.ios.iter().enumerate() {
                    println!();
                    println!("Device {}:", index + 1);
                    println!("id: {}", device.id);
                    println!("udid: {}", device.udid);
                    println!("unlocked: {}", device.unlocked);
                    println!("provisioned: {}", device.provisioned);
                }
            }
        }
    }
}