use std::{fmt::Debug, net::IpAddr, sync::mpsc, time::Duration};

use ipnetwork::Ipv4Network;
use log::debug;
use network_interface::{NetworkInterface, NetworkInterfaceConfig};

use crate::{
    hcn::{Endpoint, list_endpoints},
    hcs::get_virtual_machine_id,
    routes::add_routes,
};

#[derive(Debug)]
pub struct WslMonitor {
    pub wsl_interface_name: Option<String>,
    pub routes: Vec<Ipv4Network>,
}

impl WslMonitor {
    pub fn new(wsl_interface_name: Option<String>, routes: Vec<Ipv4Network>) -> Self {
        WslMonitor {
            wsl_interface_name,
            routes,
        }
    }

    pub fn start(&self, stop_receiver: mpsc::Receiver<()>) {
        let mut resolved_interface: Option<String> = self.wsl_interface_name.clone();
        let mut resolved_ipaddress: Option<IpAddr> = None;

        loop {
            let current_interface = resolved_interface.clone();
            match current_interface {
                None => match find_wsl_interface() {
                    Ok(val) => {
                        resolved_interface = Some(val.clone());
                        debug!("Auto detected wsl interface: {}", val.clone());
                        continue;
                    }
                    Err(e) => {
                        debug!("Error finding wsl interface: {}", e)
                    }
                },
                Some(interface_name) => {
                    match get_interface_address(interface_name.clone()) {
                        Ok(val) => {
                            let ip_addr = val.addr[0].ip();

                            if resolved_ipaddress.is_none()
                                || ip_addr != resolved_ipaddress.unwrap()
                            {
                                resolved_ipaddress = Some(ip_addr);
                                add_routes(val, self.routes.clone());
                            }
                        }
                        Err(e) => {
                            debug!(
                                "Could not get address if interface {}: {}",
                                interface_name, e
                            )
                        }
                    };
                }
            }

            if let Err(e) = stop_receiver.recv_timeout(Duration::from_secs(10)) {
                let exist = match e {
                    mpsc::RecvTimeoutError::Timeout => false,
                    mpsc::RecvTimeoutError::Disconnected => true,
                };

                if exist {
                    break;
                }
            } else {
                break;
            }
        }
    }
}

fn get_interface_address(interface_name: String) -> Result<NetworkInterface, String> {
    let interfaces =
        NetworkInterface::show().map_err(|e| format!("Failed to get network adapters: {}", e))?;

    let interfaces: Vec<&NetworkInterface> = interfaces
        .iter()
        .filter(|x| x.name == interface_name)
        .collect();

    if interfaces.is_empty() {
        Err(format!("Could not find interface: {}", interface_name))
    } else {
        Ok(interfaces[0].clone())
    }
}

fn find_wsl_interface() -> Result<String, String> {
    let vm_id = get_virtual_machine_id("WSL")?;
    let endpoints = list_endpoints()?;
    let wsl_endpoints: Vec<&Endpoint> = endpoints
        .iter()
        .filter(|x| x.VirtualMachine == vm_id)
        .collect();

    if wsl_endpoints.is_empty() {
        Err(format!("Would not find an endpoint for WSL VM {}", vm_id))
    } else {
        let gateway_ip = wsl_endpoints[0]
            .GatewayAddress
            .parse::<IpAddr>()
            .map_err(|e| format!("Failed to convert VM gateway IP: {}", e))?;

        let interfaces = NetworkInterface::show()
            .map_err(|e| format!("Failed to get network adapters: {}", e))?;

        let gatway_interface: Vec<&NetworkInterface> = interfaces
            .iter()
            .filter(|x| !x.addr.is_empty() && x.addr.iter().any(|y| y.ip() == gateway_ip))
            .collect();

        if gatway_interface.is_empty() {
            Err(String::from(format!(
                "Gatway interface for IP Address {} could not be found",
                wsl_endpoints[0].GatewayAddress
            )))
        } else {
            Ok(gatway_interface[0].name.clone())
        }
    }
}
