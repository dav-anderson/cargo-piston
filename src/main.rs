use anyhow::{bail, Result};
use clap::{Parser, CommandFactory, FromArgMatches};
use cargo_subcommand::Subcommand;

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
        if cmd.target() == Some("x86_64-apple-ios"){
            println!("build orders received for x86_64-apple-ios");
            //pseudocode
            //run pre build for x86_64-apple-ios
            //build the output
            //run post build for x86_64-apple-ios
        }else if cmd.target() == Some("aarch64-apple-ios"){
            println!("build orders received for aarch64-apple-ios")
            //run pre build for aarch64-apple-ios
            //build the output
            //run post build for x86_64-apple-ios
        }
        else {
            bail!("build target not supported {:?}", cmd.target());
        }
    }
    PistonSubCmd::Run(args) =>{
        let cmd = Subcommand::new(args.common.subcommand_args)?;
        println!("run orders received for device: {}", args.device.unwrap())
    }
    PistonSubCmd::Version => {
        println!("{}, {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    }
 }
 Ok(())
}