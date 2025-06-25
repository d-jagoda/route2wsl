use std::{ffi::OsString, path::PathBuf, thread, time::Duration};

use ipnetwork::Ipv4Network;
use log::LevelFilter;
use windows_service::{
    service::{
        ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceState,
        ServiceType,
    },
    service_manager::{ServiceManager, ServiceManagerAccess},
};

use clap::Parser;
use cli::{Cli, Commands};

use crate::{cli};

pub fn install_service(
    service_name: &str,
    wsl_interface: Option<String>,
    routes: Vec<String>,
    log_level: LevelFilter
) -> Result<(), String> {

    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager =
        ServiceManager::local_computer(None::<&str>, manager_access).map_win_err()?;

    let service_binary_path = ::std::env::current_exe().unwrap();

    let service_info = ServiceInfo {
        name: OsString::from(service_name),
        display_name: OsString::from(service_name),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: service_binary_path,
        launch_arguments: build_cmdline_args(wsl_interface, routes, log_level),
        dependencies: vec![],
        account_name: None, // run as System
        account_password: None,
    };

    let service = service_manager
        .create_service(
            &service_info,
            ServiceAccess::CHANGE_CONFIG | ServiceAccess::START,
        )
        .map_win_err()?;

    service.set_description("Configures rules in the IPv4 routing table to forward specific IP traffic through WSL")
        .map_win_err()?;

    println!("Service installed!");

    println!("Starting service");
    service.start(&[OsString::from("Starting from installer")]).map_win_err()?;
    println!("Service started");

    Ok(())
}

pub fn uninstall_service(service_name: &str) -> Result<(), String> {
    println!("Uninstalling service");

    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager =
        ServiceManager::local_computer(None::<&str>, manager_access).map_win_err()?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = service_manager
        .open_service(service_name, service_access)
        .map_win_err()?;

    // Mark service for deletion
    service.delete().map_win_err()?;

    // Our handle to it is not closed yet. So we can still query it.
    if match service.query_status() {
        Ok(it) => it,
        Err(err) => return Err(format!("Failed to get service status: {}", err)),
    }
    .current_state != ServiceState::Stopped
    {
        let _ = service
            .stop()
            .inspect_err(|e| println!("Failed to stop service: {}", e));
    }

    println!("{} is marked for deletion.", service_name);

    Ok(())
}

pub fn print_installation_details(service_name: &str) -> Result<(), String> {
   let existing_installation = get_existing_installation_details(service_name)?;

   println!("Instaled Path: {}", existing_installation.executable);
   println!("With Routes:");
   for route in existing_installation.routes {
     println!("   {route}")
   }

    Ok(())
}

pub fn add_route(service_name: &str, new_routes: Vec<Ipv4Network>) -> Result<(), String> {
    let InstallationDetails { executable, wsl_interface, routes, log_level } = get_existing_installation_details(service_name)?;
    let mut updated_routes = routes;

    for route in new_routes {
        if !updated_routes.contains(&route) {
            updated_routes.push(route);
        }
    }

    println!("Updating service with new routes");
    update_service(service_name, executable, wsl_interface, updated_routes.iter().map(|n| n.to_string()).collect(), log_level)?;

    Ok(())
}

fn build_cmdline_args(wsl_interface: Option<String>, routes: Vec<String>, log_level: LevelFilter) -> Vec<OsString> {
    let mut args = vec![
        OsString::from("run")
    ];

    if let Some(val) = wsl_interface {
        args.extend([OsString::from("--wsl-interface"), OsString::from(val.clone())]);
    }

    args.extend(
        routes
            .iter()
            .flat_map(|route| vec![OsString::from("-r"), OsString::from(route)]),
    );

    args.extend([OsString::from("--log-level"), OsString::from(log_level.to_string())]);
    args
}

fn update_service(
    service_name: &str,
    executable: String,
    wsl_interface: Option<String>,
    routes: Vec<String>,
    log_level: LevelFilter
) -> Result<(), String> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager =
        ServiceManager::local_computer(None::<&str>, manager_access).map_win_err()?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::QUERY_CONFIG | ServiceAccess::STOP | ServiceAccess::START | ServiceAccess::CHANGE_CONFIG;
    let service = service_manager
        .open_service(service_name, service_access)
        .map_win_err()?;

    let current_config = service.query_config().map_win_err()?;
    
    let updated_service_info = ServiceInfo {
        name: OsString::from(service_name),
        display_name: current_config.display_name,
        service_type: current_config.service_type,
        start_type: current_config.start_type,
        error_control: ServiceErrorControl::Normal,
        executable_path: PathBuf::from(executable),
        launch_arguments: build_cmdline_args(wsl_interface, routes, log_level),
        dependencies: current_config.dependencies,
        account_name: None, // run as System
        account_password: None,
    };  

    service.change_config(&updated_service_info).map_win_err()?;

    println!("Restarting service");

    let service_status = service.query_status().map_win_err()?;

    // Stop the service if it's running
    if service_status.current_state != ServiceState::Stopped {
        service.stop().map_win_err()?;
        
        // Wait for service to stop (with timeout)
        let mut attempts = 0;
        while service.query_status().map_win_err()?.current_state != ServiceState::Stopped {
            thread::sleep(Duration::from_secs(1));
            attempts += 1;
            if attempts > 30 {
                return Err("Timeout waiting for service to stop".into());
            }
        }
    }

    service.start(&[OsString::from("Updated from installer")]).map_win_err()?;
    println!("Service restarted");

    Ok(())
}

fn get_existing_installation_details(service_name: &str) -> Result<InstallationDetails, String> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager =
        ServiceManager::local_computer(None::<&str>, manager_access).map_win_err()?;

    let service_access = ServiceAccess::QUERY_CONFIG;
    let service = service_manager
        .open_service(service_name, service_access)
        .map_win_err()?;

    let service_config = service.query_config().map_win_err()?;

    if let Some(path_str) = service_config.executable_path.to_str() {
        let mut path_and_args = windows_args::Args::parse_cmd(path_str);

        if let Some(executable) = path_and_args.next() {
            let mut args: Vec<String> = vec![String::from("route2wsl")];
            args.extend(path_and_args.enumerate().map(|x| x.1));

            let cli: Cli = Cli::try_parse_from(args)
                .map_err(|e| format!("Service was installed with unknown arguments: {}", e))?;

            if let Commands::Run(cli::RunArgs {
                wsl_interface,
                routes,
                log_level,
            }) = cli.command
            {
                return Ok(InstallationDetails {
                    executable,
                    wsl_interface: wsl_interface,
                    routes,
                    log_level: log_level,
                });
            }
        }
    }

    return Err("A valid installation could not be found".into());

}

struct InstallationDetails {
    executable: String,
    pub wsl_interface: Option<String>,
    pub routes: Vec<Ipv4Network>,
    pub log_level: LevelFilter 
}

trait ErrorExt<T> {
    fn map_win_err(self) -> Result<T, String>;
}

impl<T> ErrorExt<T> for Result<T, windows_service::Error> {
    fn map_win_err(self) -> Result<T, String> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err((|e| {
                format!("{}", match e {
                        windows_service::Error::Winapi(error) => error.to_string(),
                        _ => e.to_string(),
                    }
                )
            })(e)),
        }
    }
}