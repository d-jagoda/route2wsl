use std::sync::mpsc;

use clap::Parser;
use cli::{Cli, Commands};
use ipnetwork::Ipv4Network;
use log::{LevelFilter, error, info};
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
};

use crate::{cli, wsl_monitor::WslMonitor, logging::init_service_logger};

pub const SERVICE_NAME: &str = "RouteToWSL";

pub fn bootstrap() {
    windows_service::service_dispatcher::start(SERVICE_NAME, ffi_service_main).unwrap();
}

define_windows_service!(ffi_service_main, service_main);

fn service_main(_arguments: Vec<std::ffi::OsString>) {
    let cli: Cli;

    match Cli::try_parse() {
        Ok(a) => cli = a,
        Err(e) => {
            eprintln!("Commandline parsing failed: {}", e);
            if let Err(e) = init_service_logger(LevelFilter::Info) {
                eprintln!("Failed to initialize logging: {}", e);
            } else {
                error!("Commandline parsing failed: {}", e)
            }

            return;
        }
    }

    let (wsl_interface, routes, log_level) = match cli.command {
        Commands::Run(cli::RunArgs {
            wsl_interface,
            routes,
            log_level,
        }) => (wsl_interface, routes, log_level),
        _ => {
            eprintln!("Unsupported command supplied");
            if let Err(e) = init_service_logger(LevelFilter::Info) {
                eprintln!("Failed to initialize logging: {}", e);
            } else {
                error!("Unsupported command supplied")
            }

            return;
        }
    };

    if let Err(e) = init_service_logger(log_level) {
        eprintln!("Failed to initialize logging: {}", e);
    }

    info!("Running service");

    if let Err(e) = run_service(wsl_interface, routes) {
        error!("Failed to run service: {}", e);
    } else {
        info!("Stopped running service");
    }
}

fn run_service(wsl_interface: Option<String>, routes: Vec<Ipv4Network>) -> Result<(), String> {
    let (stop_sender, stop_receiver) = mpsc::channel();

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => {
                stop_sender.send(()).unwrap();
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)
        .map_err(|e| format!("Failed to register service control handler: {}", e))?;

    let service_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    };

    status_handle
        .set_service_status(service_status)
        .map_err(|e| format!("Failed to set service status: {}", e))?;

    WslMonitor::new(wsl_interface, routes).start(stop_receiver);

    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: std::time::Duration::default(),
            process_id: None,
        })
        .map_err(|e| format!("Failed to set service status: {}", e))?;

    Ok(())
}
