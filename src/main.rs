use anyhow::{bail, Result};
use clap::{Parser};
use cargo_subcommand::Subcommand;
use std::env;
use std::process::Command;
use crate::android::{ AndroidBuilder, AndroidRunner };
use crate::ios::{ IOSBuilder, IOSRunner };
use crate::linux::{ LinuxBuilder, LinuxRunner };
use crate::macos::{ MacOSBuilder, MacOSRunner };
use crate::windows::WindowsBuilder;
use crate::error::PistonError;
use crate::helper::Helper;
use crate::devices::{Devices, AndroidDevice, IOSDevice};
mod devices;
mod android;
mod ios;
mod linux;
mod macos;
mod windows;
mod helper;
mod error;

#[derive(Parser)]
#[command(name = "piston")] //top level command
struct Cmd {
    #[clap(subcommand)]
    piston: PistonCmd,
}

#[derive(clap::Subcommand)]
enum PistonCmd {
    Piston {
        #[clap(subcommand)]
        cmd: PistonSubCmd
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Parser)]
#[group(skip)]
struct CommonArgs {
    #[clap(flatten)]
    subcommand_args: cargo_subcommand::Args,
}

#[derive(Clone, Debug, Eq, PartialEq, Parser)]
#[group(skip)]
struct BuildArgs {
    #[clap(flatten)]
    common: CommonArgs,
    //add custom global options if needed e.g. #[clap(short, long)] device: Option<String>
    #[clap(long)]
    dry_run: bool
}

#[derive(Clone, Debug, Eq, PartialEq, Parser)]
#[group(skip)]
struct RunArgs {
    #[clap(flatten)]
    common: CommonArgs,
    //can add custom global options if needed e.g. #[clap(short, long)] device: Option<String>
    #[clap(long)]
    device: Option<String>
}

#[derive(clap::Subcommand)]
enum PistonSubCmd {
    //Build function
    #[clap(visible_alias = "b")]
    Build(BuildArgs),
    //Run function
    #[clap(visible_alias = "r")]
    Run(RunArgs),
    //List Devices function
    #[clap(visible_alias = "l")]
    ListDevices,
    //Library Version function
    #[clap(visible_alias = "v")]
    Version,
}

// Enum for categorizing targets into platforms
#[derive(Debug)]
enum Platform {
    Android,
    Ios,
    Linux,
    Windows,
    Macos,
    Unknown,
}

impl Platform {
    // Function to categorize a target triple into a Platform using explicit pattern matching
    fn from_target(target: &str) -> Self {
        let lower_target = target.to_lowercase();
        match lower_target.as_str() {
            // MacOS targets
            "aarch64-apple-darwin" |
            "x86_64-apple-darwin" 

            // MacOS untested/unsupported
            // "arm64e-apple-darwin" |
            // "x86_64h-apple-darwin" 

            => Platform::Macos,

            // iOS targets
            "aarch64-apple-ios" |
            "x86_64-apple-ios" 

            // IOS untested/unsupported
            // "aarch64-apple-ios-macabi" |
            // "aarch64-apple-ios-sim" |
            // "arm64e-apple-ios" |
            // "armv7s-apple-ios" |
            // "i386-apple-ios" 
            
            => Platform::Ios,

            // Linux targets
            "aarch64-unknown-linux-gnu" |
            "x86_64-unknown-linux-gnu" 

            // Linux untested/unsupported
            // "i686-unknown-linux-gnu" |
            // "aarch64-unknown-linux-musl" |
            // "arm-unknown-linux-gnueabi" |
            // "arm-unknown-linux-gnueabihf" |
            // "armv7-unknown-linux-gnueabihf" |
            // "i586-unknown-linux-gnu" |
            // "i686-unknown-linux-musl" |
            // "loongarch64-unknown-linux-gnu" |
            // "loongarch64-unknown-linux-musl" |
            // "powerpc-unknown-linux-gnu" |
            // "powerpc64-unknown-linux-gnu" |
            // "powerpc64le-unknown-linux-gnu" |
            // "powerpc64le-unknown-linux-musl" |
            // "riscv64gc-unknown-linux-gnu" |
            // "s390x-unknown-linux-gnu" |
            // "x86_64-unknown-linux-musl" |
            // "arm-unknown-linux-musleabi" |
            // "arm-unknown-linux-musleabihf" |
            // "armv5te-unknown-linux-gnueabi" |
            // "armv7-unknown-linux-gnueabi" |
            // "armv7-unknown-linux-musleabi" |
            // "armv7-unknown-linux-musleabihf" |
            // "i586-unknown-linux-musl" |
            // "riscv64gc-unknown-linux-musl" |
            // "sparc64-unknown-linux-gnu" |
            // "thumbv7neon-unknown-linux-gnueabihf" |
            // "x86_64-unknown-linux-gnux32" |
            // "aarch64-unknown-linux-gnu_ilp32" |
            // "aarch64_be-unknown-linux-gnu" |
            // "aarch64_be-unknown-linux-gnu_ilp32" |
            // "aarch64_be-unknown-linux-musl" |
            // "armeb-unknown-linux-gnueabi" |
            // "csky-unknown-linux-gnuabiv2" |
            // "csky-unknown-linux-gnuabiv2hf" |
            // "hexagon-unknown-linux-musl" |
            // "i686-unknown-haiku" |
            // "loongarch64-unknown-linux-ohos" |
            // "mips-unknown-linux-gnu" |
            // "mips-unknown-linux-musl" |
            // "mips-unknown-linux-uclibc" |
            // "mips64-unknown-linux-gnuabi64" |
            // "mips64-unknown-linux-muslabi64" |
            // "mips64el-unknown-linux-gnuabi64" |
            // "mips64el-unknown-linux-muslabi64" |
            // "mipsel-unknown-linux-gnu" |
            // "mipsel-unknown-linux-musl" |
            // "mipsel-unknown-linux-uclibc" |
            // "powerpc-unknown-linux-gnuspe" |
            // "powerpc-unknown-linux-musl" |
            // "powerpc-unknown-linux-muslspe" |
            // "riscv32gc-unknown-linux-gnu" |
            // "riscv32gc-unknown-linux-musl" |
            // "riscv64a23-unknown-linux-gnu" |
            // "s390x-unknown-linux-musl" |
            // "sparc-unknown-linux-gnu" |
            // "thumbv7neon-unknown-linux-musleabihf" |
            // "x86_64-unknown-dragonfly" |
            // "x86_64-unknown-haiku" |
            // "x86_64-unknown-linux-none" |
            // "x86_64-unikraft-linux-musl" 

            => Platform::Linux,

            // Windows targets
            "x86_64-pc-windows-gnu" 

            // Windows untested/unsupported
            // "aarch64-pc-windows-msvc" |
            // "i686-pc-windows-msvc" |
            // "x86_64-pc-windows-msvc" |
            // "aarch64-pc-windows-gnullvm" |
            // "i686-pc-windows-gnu" |
            // "x86_64-pc-windows-gnullvm" |
            // "arm64ec-pc-windows-msvc" |
            // "i686-pc-nto-qnx700" |
            // "i686-uwp-windows-gnu" |
            // "i686-win7-windows-gnu" |
            // "i686-win7-windows-msvc" |
            // "thumbv7a-pc-windows-msvc" |
            // "thumbv7a-uwp-windows-msvc" |
            // "x86_64-pc-nto-qnx710" |
            // "x86_64-pc-nto-qnx710_iosock" |
            // "x86_64-pc-nto-qnx800" |
            // "x86_64-uwp-windows-gnu" |
            // "x86_64-uwp-windows-msvc" |
            // "x86_64-win7-windows-gnu" |
            // "x86_64-win7-windows-msvc" 

            => Platform::Windows,

            // Android targets
            "aarch64-linux-android" |
            "x86_64-linux-android" 
            // Android untested/unsupported
            // "arm-linux-androideabi" |
            // "armv7-linux-androideabi" |
            // "i686-linux-android" |
            // "riscv64-linux-android" 
            
            => Platform::Android,

            // All others are Unknown
            _ => Platform::Unknown,
        }
    }
}

fn main() -> Result<()> {
//init logs
 env_logger::init();

//read .env file
let env_vars = Helper::load_env_file()?;

// Parse local current working dir
let cwd = match env::current_dir(){
    Ok(cwd) => cwd,
    Err(_) => bail!("error getting working directory")
};

 //parse command
 let Cmd {
    piston: PistonCmd::Piston { cmd },
 } = Cmd::parse();

 match cmd {
    PistonSubCmd::Build(args) => {
        //TODO remove this after implementing linux host builder support
        if std::env::consts::OS == "linux" {
            bail!("Linux host support not yet implemented");
        }
        let cmd = Subcommand::new(args.common.subcommand_args)?;
        //handle the target flag
        let target_opt = cmd.target();
        //determine the target platform
        let platform = match target_opt {
            Some(target) => Platform::from_target(target),
            //if no target flag, determine host platform as default (only macos and linux supported currently)
            None => match std::env::consts::OS {
                "macos" => Platform::Macos,
                "linux" => Platform::Linux,
                _ => Platform::Unknown,  // unsupported
            }
        };
        //handle the release flag
        let release: bool = cmd.args().release;
        //determine the target to pass into the builder if no flag is provided
        let target_string = if cmd.target().is_none() {
            let output = Command::new("rustc")
                .arg("-vV")
                .output();
            let stdout = output.unwrap().stdout;
            let stdout_str = String::from_utf8_lossy(&stdout);
            let value = stdout_str.lines()
                .find(|line| line.starts_with("host:"))
                .map(|line| line.trim_start_matches("host: ").trim().to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            value
            //flag provided, pass in provided value
            } else {
                cmd.target().unwrap().to_string()
            };
        //call the appropriate builder for the designated target
        match platform {
            Platform::Android => {
            println!("build orders received for Android targeting {:?}, release is set to {:?}", cmd.target(), release);
            AndroidBuilder::start(release, target_string, cwd, env_vars)?;
            },
            Platform::Ios => {
            println!("build orders received for IOS targeting {:?}, release is set to {:?}", cmd.target(), release);
            IOSBuilder::start(release, target_string, cwd, env_vars)?;
            },
            Platform::Linux => {
            println!("build orders received for Linux targeting {:?}, release is set to {:?}", cmd.target(), release);
            LinuxBuilder::start(release, target_string, cwd, env_vars)?;
            },
            Platform::Windows => {
            println!("build orders received for Windows targeting {:?}, release is set to {:?}", cmd.target(), release);
            WindowsBuilder::start(release, target_string, cwd, env_vars)?;
            },
            Platform::Macos => {
            println!("build orders received for Macos targeting {:?}, release is set to {:?}", cmd.target(), release);
            MacOSBuilder::start(release, target_string, cwd, env_vars)?;
            },
            Platform::Unknown => bail!("Unknown or unsupported target: {:?}", target_opt),
        };
        if args.dry_run {
            println!("(Dry run mode enabled)");
        }
    }
    PistonSubCmd::Run(args) =>{
        let cmd = Subcommand::new(args.common.subcommand_args)?;
        //handle the release flag
        let release: bool = cmd.args().release;
        //no device flag, run locally
        if args.device.is_none() {
            println!("run orders received with no device, run locally");
            //MacOS host machine
            if std::env::consts::OS == "macos" {
                MacOSRunner::start(release, cwd, env_vars)?;
            //Linux host machine
            }else if std::env::consts::OS == "linux" {
                LinuxRunner::start(release, cwd, env_vars)?;
            }
            else {
                bail!("Unsupported host system, cargo-piston only supports macos or linux host machines. Your host machine: {:?}", std::env::consts::OS)
            }
        //explicit device flag
        }else{
            let tgt_unwrap = args.device.unwrap();
            let target_device = tgt_unwrap.trim().clone();
            //explicit device flag can either be "ios" or "android" or the target device id
            println!("run orders received for a target device: {}", &tgt_unwrap);
            let devices = Devices::list_devices(env_vars.clone(), true)?;
            let android_device: Option<&AndroidDevice> = devices.android.iter().find(|device| device.id == target_device);
            let ios_device: Option<&IOSDevice> = devices.ios.iter().find(|device| device.id == target_device);
            //general IOS target
            if target_device == "ios" && !devices.ios.is_empty(){
                //TODO make this a smarter choice, instead of defaulting to first item in the vec
                let device = &devices.ios[0];
                println!("general IOS runner target: {:?}", &device);
                IOSRunner::start(release, cwd, env_vars, &device)?;
            //general Android target
            }else if target_device == "android" && !devices.android.is_empty(){
                //TODO make this a smarter choice, instead of defaulting to first item in the vec
                let device = &devices.android[0];
                println!("general Android runner target: {:?}", &device);
                AndroidRunner::start(release, cwd, env_vars, &device)?;
            //explicit android target
            } else if !android_device.is_none() {
                println!("explicit Android runner target: {:?}", &android_device);
                AndroidRunner::start(release, cwd, env_vars, &android_device.unwrap())?;
            //explicit iOS target
            } else if !ios_device.is_none() {
                println!("explicit IOS runner target: {:?}", &ios_device);
                IOSRunner::start(release, cwd, env_vars, &ios_device.unwrap())?;
            } else {
                bail!("Device not found");
            }
        }
        
    }
    PistonSubCmd::ListDevices => {
        println!("list all available connected devices and relevant information");
        Devices::list_devices(env_vars, false)?;
    }
    PistonSubCmd::Version => {
        println!("{}, {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    }
 }
 Ok(())
}

#[test]
fn test_platform_from_target() {
    assert!(matches!(Platform::from_target("aarch64-apple-darwin"), Platform::Macos));
    assert!(matches!(Platform::from_target("aarch64-apple-ios"), Platform::Ios));
    assert!(matches!(Platform::from_target("aarch64-unknown-linux-gnu"), Platform::Linux));
    assert!(matches!(Platform::from_target("x86_64-pc-windows-msvc"), Platform::Windows));
    assert!(matches!(Platform::from_target("aarch64-linux-android"), Platform::Android));
    assert!(matches!(Platform::from_target("some-unknown-target"), Platform::Unknown));
}

//TODO refactor main to use custom error types and remove anyhow as a dep

//TODO refactor most of main into a lib.rs

//TODO implement automated signing for iOS, MacOS, Windows, and Android outputs

//TODO if apple app store connect api key provided in .env, perform setup via api if needed, otherwise assume user prefers to do it manually
//Obtain a signing cert if it doesn't already exist
//provision a target device if its not already provisioned

//TODO cargo.toml
//more extensive android permissions
//more extensive macos/ios permissions

//TODO add linux host machine support for all features