use serde::{Deserialize, Serialize};
use windows::{
    core::{HSTRING, PWSTR}, 
    Win32::System::HostComputeSystem::{HcsCloseOperation, HcsCreateOperation, HcsEnumerateComputeSystems, HcsWaitForOperationResult}
};

pub fn get_virtual_machine_id(owner: &str) -> Result<String, String> {

    #[derive(Debug, Serialize, Deserialize)]
    struct ComputeSystem {
        #[serde(rename = "Id")]
        id: String,
    }

    unsafe {
        let operation = HcsCreateOperation(None, None);
        let query_hstring = HSTRING::from(format!(r#"{{"Owners": ["{}"]}}"#, owner));
        
        HcsEnumerateComputeSystems(&query_hstring, operation.clone())
            .map_err(|e| e.to_string())?;

        let mut result_doc: PWSTR = PWSTR(std::ptr::null_mut());

        HcsWaitForOperationResult(operation, u32::MAX, Some(&mut result_doc))
            .map_err(|e| e.to_string())?;

        let result_string = if !result_doc.is_null() {
            HSTRING::from_wide(result_doc.as_wide())
        } else {
            HSTRING::new()
        };

        HcsCloseOperation(operation);

        let s = result_string.to_string_lossy();

        let compute_systems: Vec<ComputeSystem> = serde_json::from_str(s.as_str())
            .map_err(|e| e.to_string())?;

        if compute_systems.is_empty() {
            Err(String::from(format!("Could not find virtual machine for {}", owner)))
        } else {
            Ok(compute_systems[0].id.clone())
        }
    }
}
