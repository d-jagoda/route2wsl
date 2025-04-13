use std::ffi::OsString;

use log::LevelFilter;
use windows_service::{
    service::{
        ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceState,
        ServiceType,
    },
    service_manager::{ServiceManager, ServiceManagerAccess},
};

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

    fn build_args(wsl_interface: Option<String>, routes: Vec<String>, log_level: LevelFilter) -> Vec<OsString> {
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

    let service_info = ServiceInfo {
        name: OsString::from(service_name),
        display_name: OsString::from(service_name),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: service_binary_path,
        launch_arguments: build_args(wsl_interface, routes, log_level),
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