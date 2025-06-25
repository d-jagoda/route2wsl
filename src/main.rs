use clap::Parser;
use cli::{Cli, Commands};

mod cli;
mod wsl_monitor;
mod hcn;
mod hcs;
mod installer;
mod logging;
mod service;
mod routes;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install(cli::RunArgs {
            wsl_interface,
            routes,
            log_level,
        }) => {
            if let Err(_e) = installer::install_service(
                service::SERVICE_NAME,
                wsl_interface,
                routes.iter().map(|n| n.to_string()).collect(),
                log_level
            ) {
                println!("{}", _e);
            }
        }
        Commands::Uninstall => {
            if let Err(_e) = installer::uninstall_service(service::SERVICE_NAME) {
                println!("{}", _e);
            }
        }
        Commands::Inspect => {
            if let Err(_e) = installer::print_installation_details(service::SERVICE_NAME) {
                println!("{}", _e);
            }          
        }
        Commands::AddRoute(cli::ChangeRoutesArgs {
            routes
        }) => {
            if let Err(_e) = installer::add_route(service::SERVICE_NAME, routes) {
                println!("{}", _e);
            }             
        }
        _ => service::bootstrap(),
    }
}
