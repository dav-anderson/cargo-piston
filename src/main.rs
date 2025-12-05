use anyhow::{bail, Result};
use clap::{Parser, CommandFactory, FromArgMatches};
use cargo_subcommand::Subcommand;
use std::env;

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
    //add custom global options here e.g. #[clap(short, long)] device: Option<String>
}

#[derive(Clone, Debug, Eq, PartialEq, Parser)]
#[group(skip)]
struct BuildArgs {
    #[clap(flatten)]
    common: CommonArgs,
    //add custom global options here e.g. #[clap(short, long)] device: Option<String>
    #[clap(long)]
    dry_run: bool
}

#[derive(Clone, Debug, Eq, PartialEq, Parser)]
#[group(skip)]
struct RunArgs {
    #[clap(flatten)]
    common: CommonArgs,
    //add custom global options here e.g. #[clap(short, long)] device: Option<String>
    #[clap(long)]
    device: Option<String> //should this be optional?
}

#[derive(clap::Subcommand)]
enum PistonSubCmd {
    //talk function
    #[clap(visible_alias = "b")]
    Build(BuildArgs),
    //listen function
    #[clap(visible_alias = "r")]
    Run(RunArgs),
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
            "x86_64-apple-darwin" |
            "arm64e-apple-darwin" |
            "x86_64h-apple-darwin" => Platform::Macos,

            // iOS targets
            "aarch64-apple-ios" |
            "aarch64-apple-ios-macabi" |
            "aarch64-apple-ios-sim" |
            "x86_64-apple-ios" |
            "arm64e-apple-ios" |
            "armv7s-apple-ios" |
            "i386-apple-ios" => Platform::Ios,

            // Linux targets
            "aarch64-unknown-linux-gnu" |
            "i686-unknown-linux-gnu" |
            "x86_64-unknown-linux-gnu" |
            "aarch64-unknown-linux-musl" |
            "arm-unknown-linux-gnueabi" |
            "arm-unknown-linux-gnueabihf" |
            "armv7-unknown-linux-gnueabihf" |
            "i586-unknown-linux-gnu" |
            "i686-unknown-linux-musl" |
            "loongarch64-unknown-linux-gnu" |
            "loongarch64-unknown-linux-musl" |
            "powerpc-unknown-linux-gnu" |
            "powerpc64-unknown-linux-gnu" |
            "powerpc64le-unknown-linux-gnu" |
            "powerpc64le-unknown-linux-musl" |
            "riscv64gc-unknown-linux-gnu" |
            "s390x-unknown-linux-gnu" |
            "x86_64-unknown-linux-musl" |
            "arm-unknown-linux-musleabi" |
            "arm-unknown-linux-musleabihf" |
            "armv5te-unknown-linux-gnueabi" |
            "armv7-unknown-linux-gnueabi" |
            "armv7-unknown-linux-musleabi" |
            "armv7-unknown-linux-musleabihf" |
            "i586-unknown-linux-musl" |
            "riscv64gc-unknown-linux-musl" |
            "sparc64-unknown-linux-gnu" |
            "thumbv7neon-unknown-linux-gnueabihf" |
            "x86_64-unknown-linux-gnux32" |
            "aarch64-unknown-linux-gnu_ilp32" |
            "aarch64_be-unknown-linux-gnu" |
            "aarch64_be-unknown-linux-gnu_ilp32" |
            "aarch64_be-unknown-linux-musl" |
            "armeb-unknown-linux-gnueabi" |
            "csky-unknown-linux-gnuabiv2" |
            "csky-unknown-linux-gnuabiv2hf" |
            "hexagon-unknown-linux-musl" |
            "i686-unknown-haiku" |
            "loongarch64-unknown-linux-ohos" |
            "mips-unknown-linux-gnu" |
            "mips-unknown-linux-musl" |
            "mips-unknown-linux-uclibc" |
            "mips64-unknown-linux-gnuabi64" |
            "mips64-unknown-linux-muslabi64" |
            "mips64el-unknown-linux-gnuabi64" |
            "mips64el-unknown-linux-muslabi64" |
            "mipsel-unknown-linux-gnu" |
            "mipsel-unknown-linux-musl" |
            "mipsel-unknown-linux-uclibc" |
            "powerpc-unknown-linux-gnuspe" |
            "powerpc-unknown-linux-musl" |
            "powerpc-unknown-linux-muslspe" |
            "riscv32gc-unknown-linux-gnu" |
            "riscv32gc-unknown-linux-musl" |
            "riscv64a23-unknown-linux-gnu" |
            "s390x-unknown-linux-musl" |
            "sparc-unknown-linux-gnu" |
            "thumbv7neon-unknown-linux-musleabihf" |
            "x86_64-unknown-dragonfly" |
            "x86_64-unknown-haiku" |
            "x86_64-unknown-linux-none" |
            "x86_64-unikraft-linux-musl" => Platform::Linux,

            // Windows targets
            "aarch64-pc-windows-msvc" |
            "i686-pc-windows-msvc" |
            "x86_64-pc-windows-gnu" |
            "x86_64-pc-windows-msvc" |
            "aarch64-pc-windows-gnullvm" |
            "i686-pc-windows-gnu" |
            "x86_64-pc-windows-gnullvm" |
            "arm64ec-pc-windows-msvc" |
            "i686-pc-nto-qnx700" |
            "i686-uwp-windows-gnu" |
            "i686-win7-windows-gnu" |
            "i686-win7-windows-msvc" |
            "thumbv7a-pc-windows-msvc" |
            "thumbv7a-uwp-windows-msvc" |
            "x86_64-pc-nto-qnx710" |
            "x86_64-pc-nto-qnx710_iosock" |
            "x86_64-pc-nto-qnx800" |
            "x86_64-uwp-windows-gnu" |
            "x86_64-uwp-windows-msvc" |
            "x86_64-win7-windows-gnu" |
            "x86_64-win7-windows-msvc" => Platform::Windows,

            // Android targets
            "aarch64-linux-android" |
            "arm-linux-androideabi" |
            "armv7-linux-androideabi" |
            "i686-linux-android" |
            "x86_64-linux-android" |
            "riscv64-linux-android" => Platform::Android,

            // All others are Unknown
            _ => Platform::Unknown,
        }
    }
}

fn main() -> Result<()> {
//init logs
 env_logger::init();

 //parse command
 let Cmd {
    piston: PistonCmd::Piston { cmd },
 } = Cmd::parse();

 match cmd {
    PistonSubCmd::Build(args) => {
        let cmd = Subcommand::new(args.common.subcommand_args)?;
        let target_opt = cmd.target();
        let platform = match target_opt {
            Some(target) => Platform::from_target(target),
            //if no target flag, determine host platform as default (only macos and linux supported currently)
            None => match std::env::consts::OS {
                "macos" => Platform::Macos,
                "linux" => Platform::Linux,
                _ => Platform::Unknown,  // unsupported
            }
        };
        //determine the platform category of the build target
        let category = match platform {
            Platform::Android => "Android",
            Platform::Ios => "Ios",
            Platform::Linux => "Linux",
            Platform::Windows => "Windows",
            Platform::Macos => "Macos",
            Platform::Unknown => bail!("Unknown or unsupported target: {:?}", target_opt),
        };
        if args.dry_run {
            println!("(Dry run mode enabled)");
        }
        if category == "Android"{
            println!("build orders received for Android targeting {:?}", cmd.target());
            //pseudocode
            //TODO
            //run pre build for Android
            //build the output
            //run post build for Android
            //autoinstall on a target device?
        }else if category == "Ios"{
            println!("build orders received for IOS targeting {:?}", cmd.target());
            //autoinstall on a target device?
        }else if category == "Linux"{
            println!("build orders received for Linux targeting {:?}", cmd.target());
        }else if category == "Windows"{
            println!("build orders received for Windows targeting {:?}", cmd.target());
        }else if category == "Macos"{
            println!("build orders received for Macos targeting {:?}", cmd.target());
        }
        else {
            bail!("build target not supported {:?}", cmd.target());
        }
    }
    PistonSubCmd::Run(args) =>{
        let cmd = Subcommand::new(args.common.subcommand_args)?;
        //TODO can we auto run on a target device without forcing the user to specify a device?
        if args.device.is_none() {
            println!("run orders received with no device, run locally");
        }else{
            println!("run orders received for a target device: {}", args.device.unwrap())
        }
    }
    PistonSubCmd::Version => {
        println!("{}, {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    }
 }
 //load the .env if it exists
 dotenv::dotenv().ok();
 //print the test value from the .env
 let test_value = env::var("test").unwrap_or_else(|_| "not set".to_string());
 println!("Printing .env test key: {}", test_value);
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
