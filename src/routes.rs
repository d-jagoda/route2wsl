use std::net::{IpAddr, Ipv4Addr};

use ipnetwork::Ipv4Network;
use log::{debug, error};
use network_interface::NetworkInterface;
use windows::Win32::{
    Foundation::NO_ERROR,
    NetworkManagement::IpHelper::{
        CreateIpForwardEntry2, InitializeIpForwardEntry, MIB_IPFORWARD_ROW2,
    },
    Networking::WinSock::{AF_INET, MIB_IPPROTO_NETMGMT},
};

pub fn add_routes(gateway: NetworkInterface, routes: Vec<Ipv4Network>) {
    unsafe {
        let mut gateway_address: Option<Ipv4Addr> = None;

        for addr in gateway.addr.clone() {
            match addr.ip() {
                IpAddr::V4(a) => {
                    gateway_address = Some(a);
                    break;
                }
                _ => {}
            }
        }

        if gateway_address == None {
            debug!("Gateway IP is incompatible {}", gateway.addr[0].ip());
            return;
        }

        for route in routes {
            debug!(
                "Setting route {} via gateway {}",
                route.ip(),
                gateway_address.unwrap()
            );

            let mut row: MIB_IPFORWARD_ROW2 = MIB_IPFORWARD_ROW2::default();
            InitializeIpForwardEntry(&mut row);

            row.InterfaceIndex = gateway.index;
            row.DestinationPrefix.PrefixLength = route.prefix();
            row.DestinationPrefix.Prefix.si_family = AF_INET;
            row.DestinationPrefix.Prefix.Ipv4.sin_addr.S_un.S_addr =
                u32::from_ne_bytes(route.ip().octets());

            row.NextHop.si_family = AF_INET;
            row.NextHop.Ipv4.sin_addr.S_un.S_addr =
                u32::from_ne_bytes(gateway_address.unwrap().octets());
            row.Metric = 1;
            row.Protocol = MIB_IPPROTO_NETMGMT;

            let result = CreateIpForwardEntry2(&row);

            if result != NO_ERROR {
                let error = windows::core::Error::from(result);
                error!("Failed to set route {}: {} {}", route.ip(), result.0, error);
            }
        }
    }
}
