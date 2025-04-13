use serde::{Deserialize, Serialize};
use windows::{
    core::{GUID, HSTRING, PWSTR}, 
    Win32::System::HostComputeNetwork::{HcnCloseEndpoint, HcnEnumerateEndpoints, HcnOpenEndpoint, HcnQueryEndpointProperties}
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Endpoint {
    pub ID: String,
    pub Name: String,
    pub VirtualNetwork: String,
    pub VirtualNetworkName: String,
    pub IPAddress:  String,
    pub GatewayAddress: String,
    pub VirtualMachine:  String,
}

pub fn list_endpoints() -> Result<Vec<Endpoint>, String> {
   
    let ids = list_endpoint_ids()?;
    let result : Result<Vec<Endpoint>, String> = ids.iter().map(|id|  {
        let s = get_endpoint_properties(id.clone())?;
        let network: Endpoint = serde_json::from_str(s.as_str()).map_err(|e| e.to_string())?;
        Ok(network)

    }).collect();

    return result;
}

fn list_endpoint_ids() -> Result<Vec<String>, String> {
    unsafe {
        let mut result_doc: PWSTR = PWSTR(std::ptr::null_mut());
        let mut error_record: PWSTR = PWSTR(std::ptr::null_mut());
        HcnEnumerateEndpoints(
            &HSTRING::from(r#""#),
            &mut result_doc,
            Some(&mut error_record),
        )
        .map_err(|e| e.message())?;

        let result_string = if !result_doc.is_null() {
            HSTRING::from_wide(result_doc.as_wide())
        } else {
            HSTRING::new()
        };

        let s = result_string.to_string_lossy();
        let network_ids: Vec<String> = serde_json::from_str(s.as_str()).unwrap();

        Ok(network_ids)
    }
}

fn get_endpoint_properties(id: String) -> Result<String, String> {
    unsafe {
        let endpoint_id = GUID::try_from(id.as_str()).unwrap();
        let mut endpoint_handle: *mut core::ffi::c_void = std::ptr::null_mut();
        let mut error_record: PWSTR = PWSTR(std::ptr::null_mut());

        HcnOpenEndpoint(&endpoint_id, &mut endpoint_handle, Some(&mut error_record))
            .map_err(|e| e.message())?;

        let mut result_doc: PWSTR = PWSTR(std::ptr::null_mut());
        let mut error_record: PWSTR = PWSTR(std::ptr::null_mut());

        HcnQueryEndpointProperties(
            endpoint_handle,
            &HSTRING::from(r#""#),
            &mut result_doc,
            Some(&mut error_record),
        )
        .map_err(|e| e.message())?;

        HcnCloseEndpoint(endpoint_handle).map_err(|e| e.message())?;

        let result_string = if !result_doc.is_null() {
            HSTRING::from_wide(result_doc.as_wide())
        } else {
            HSTRING::new()
        };

        let s = result_string.to_string_lossy();
        Ok(s)
    }
}