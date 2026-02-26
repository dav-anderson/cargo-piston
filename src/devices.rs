use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use regex::Regex;
use crate::error::PistonError;

#[derive(Debug)]
pub struct IOSDevice {
model: String,
id: String,
udid: String,
provisioned: bool,
}

#[derive(Debug)]
pub struct Devices {
    ios: Vec<IOSDevice>,
    android: Vec<String>,
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
                    self.android.push(parts[0].to_string())
                }
            }
        }

        Ok(())

    }

    fn populate_ios(&mut self) -> Result<(), PistonError>{
        // Run the command `xcrun devicectl list devices`
        let output = match Command::new("xcrun").args(["devicectl", "list", "devices"]).stdout(Stdio::piped()).output() {
                Ok(o) => o,
                Err(e) => return Err(PistonError::XcrunDevicectlError(e.to_string())),
            };

        // Convert the output to a UTF-8 string.
        let stdout = match str::from_utf8(&output.stdout){
            Ok(o) => o,
            Err(e) => return Err(PistonError::ParseUTF8Error(e.to_string())),
        };

        // Split the output into lines.
        let lines: Vec<String> = stdout.lines().map(str::to_string).collect();

        //if no results
        if lines.len() < 2 {
            return Ok(());
        }

        let dash_line = &lines[1];

        // Find column ranges from dash line
        let mut columns: Vec<(usize, usize)> = Vec::new();
        let dash_chars: Vec<char> = dash_line.chars().collect();
        let mut i = 0;
        while i < dash_chars.len() {
            if dash_chars[i] == '-' {
                let start = i;
                let mut j = i;
                while j < dash_chars.len() && dash_chars[j] == '-' {
                    j += 1;
                }
                columns.push((start, j));
                i = j;
            } else {
                i += 1;
            }
        }

        // Process device lines (skip header and dash line)
        for line in &lines[2..] {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let line_chars: Vec<char> = line.chars().collect();

            let mut fields: Vec<String> = Vec::new();
            for &(start, end) in &columns {
                let actual_end = end.min(line_chars.len());
                let field_slice = if start < line_chars.len() {
                    &line_chars[start..actual_end]
                } else {
                    &[]
                };
                let field: String = field_slice.iter().collect();
                fields.push(field.trim().to_string());
            }

            // fields[0]: name, [1]: hostname, [2]: identifier, [3]: state, [4]: model
            if fields.len() < 4 {
                continue;
            }

            let name = fields[0].to_lowercase();
            if name != "iphone" {
                continue;
            }

            let state = &fields[3];
            if !state.starts_with("available") {
                continue;
            }

            //construct the return fields
            let hostname = &fields[1];
            let id = hostname.trim_end_matches(".coredevice.local").to_string();
            let udid = fields[2].clone();
            let model = if fields.len() > 4 { fields[4].clone() } else { "unknown".to_string() };

            self.ios.push(IOSDevice {
                model,
                id,
                udid,
                provisioned: false,
            });
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
                    println!("{}", device);
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
                    println!("udid: {}", device.udid);
                    println!("provisioned: {}", device.provisioned);
                }
            }
        }
    }
}