use clap::{Args, Parser, Subcommand};
use ipnetwork::Ipv4Network;
use log::LevelFilter;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    /// The name of the WSL network interface. If specified this interface will be used instead of auto detecting it.
    #[clap(long)]
    pub wsl_interface: Option<String>,

    /// Route in the format IP/MASK. This argument can be repeated. For example: -r 10.1.0.0/16 -r 10.96.0.0/12
    #[clap(
        action(clap::ArgAction::Append),
        long("route"),
        short,
        required(true),
        value_parser  = validate_route,
        value_name = "ROUTE"
    )]
    pub routes: Vec<Ipv4Network>,

    #[clap(long, default_value("Info"))]
    pub log_level: LevelFilter 
}

#[derive(Subcommand)]
#[clap(name = "Route to WSL")]
#[clap(
    about = "Configures rules in the IPv4 routing table to forward specific IP traffic through WSL"
)]
pub enum Commands {
    /// Installs the service
    Install(RunArgs),

    /// Uninstalls the service
    Uninstall,

    /// Runs the service
    #[clap(hide = true)]
    Run(RunArgs),
}

pub fn validate_route(val: &str) -> Result<Ipv4Network, String> {

    if !val.contains('/') {
        return Err(String::from("Use CIDR format like 10.0.0.0/24"));
    }   

    match val.parse::<Ipv4Network>() {
        Ok(network) => Ok(network),
        Err(e) => Err(format!("{}", e)),
    }
}