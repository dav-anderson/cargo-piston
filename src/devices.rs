use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use crate::error::PistonError;

#[derive(Debug, Clone)]
pub struct IOSDevice {
pub model: String,
pub id: String,
}

#[derive(Debug, Clone)]
pub struct AndroidDevice {
    // model: String,
    pub id: String,
}

#[derive(Debug)]
pub struct Devices {
    pub ios: Vec<IOSDevice>,
    pub android: Vec<AndroidDevice>,
}

impl Devices {
    pub fn list_devices(env_vars: HashMap<String, String>, silent: bool) -> Result<Self, PistonError> {
        //new devices struct
        let mut devices = Devices {
            ios: Vec::new(),
            android: Vec::new(),
        };

        let sdk_path: Option<String> = env_vars.get("sdk_path").cloned();
        let adb_path = format!("{}/platform-tools/adb", sdk_path.unwrap_or_default());
        //query Android devices if adb_path is configured in .env
        if Path::new(&adb_path).exists() {
            devices.populate_android(adb_path)?;
        }else{
            println!("Android installation not found");
        }
        //query iOS devices if on MacOS
        if std::env::consts::OS == "macos" {
            devices.populate_ios()?;
        }
        //print the device results to the terminal
        if !silent {
            devices.print_devices();
        }
        //return the struct
        Ok(devices)
    }

    pub fn populate_android(&mut self, adb_path: String) -> Result<(), PistonError>{
        //Run the command `adb devices`
        let output = match Command::new(adb_path).arg("devices").output() {
            Ok(o) => o,
            Err(e) => return Err(PistonError::ADBDevicesError(e.to_string())),
        };

        //convert the output to utf8
        let stdout = match str::from_utf8(&output.stdout) {
            Ok(o) => o,
            Err(e) => return Err(PistonError::ParseUTF8Error(e.to_string())),
        };

        //split the output into lines
        let lines: Vec<&str> = stdout.lines().collect();

        //Skip the first line ("list of devices attached") and process the results
        for line in lines.iter().skip(1) {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() == 2 && parts[1] == "device" {
                    //add the device serial to the vector
                    self.android.push(AndroidDevice {
                        //TODO make this model field more dynamic for chipset architecture currently useless
                        // model: "Android Device".to_string(),
                        id: parts[0].to_string(),
                    });
                }
            }
        }

        Ok(())

    }

    fn populate_ios(&mut self) -> Result<(), PistonError> {
        // Run the command `xcrun xctrace list devices`
        let output = match Command::new("xcrun").args(["xctrace", "list", "devices"]).stdout(Stdio::piped()).output() {
            Ok(o) => o,
            Err(e) => return Err(PistonError::XcrunDevicectlError(e.to_string())),
        };

        // Convert the output to a UTF-8 string.
        let stdout = match str::from_utf8(&output.stdout){
            Ok(o) => o,
            Err(e) => return Err(PistonError::ParseUTF8Error(e.to_string())),
        };

        // Split the output into lines (matching the style of the original function).
        let lines: Vec<String> = stdout.lines().map(str::to_string).collect();

        let mut in_devices_section = false;

        for line in &lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if trimmed == "== Devices ==" {
                in_devices_section = true;
                continue;
            }

            if trimmed == "== Devices Offline ==" {
                in_devices_section = false;
                break; // No need to process anything after the offline section
            }

            if !in_devices_section {
                continue;
            }

            // Only parse iPhones (following the original function's lowercase convention for filtering).
            if !trimmed.to_lowercase().starts_with("iphone") {
                continue;
            }

            // Extract model and identifier.
            // The identifier is always inside the *last* set of parentheses.
            // Model keeps any parentheses (e.g. "iPhone (26.4.2)").
            if let Some(last_open) = trimmed.rfind('(') {
                if let Some(last_close) = trimmed.rfind(')') {
                    if last_open < last_close {
                        let id = trimmed[(last_open + 1)..last_close].trim().to_string();
                        let model = trimmed[0..last_open].trim().to_string();

                        self.ios.push(IOSDevice {
                            model,
                            id,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    pub fn print_devices(&self) {
        //empty device list
        if self.ios.is_empty() && self.android.is_empty() {
            println!();
            println!("Cargo Piston Device List:");
            println!();
            println!("No devices connected");
        } else {
            //Android device list
            println!();
            println!("Cargo Piston Device List:");
            println!();
            if !self.android.is_empty() {
                println!("Android:");
                for (index, device) in self.android.iter().enumerate() {
                    println!();
                    println!("Device {}:", index + 1);
                    println!("id: {}", device.id);
                }
                if !self.ios.is_empty() {
                    println!();

                }
            }
            //iOS device list
            if !self.ios.is_empty() {
                println!("iOS:");
                for (index, device) in self.ios.iter().enumerate() {
                    println!();
                    println!("Device {}:", index + 1);
                    println!("Model: {}", device.model);
                    println!("id: {}", device.id);
                }
            }
        }
    }
}