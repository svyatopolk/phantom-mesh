use sysinfo::System;
use num_cpus;

#[cfg(windows)]
use winapi::um::debugapi::IsDebuggerPresent;

pub fn is_analysis_environment() -> bool {
    if check_hardware() { return true; }
    if check_debugger() { return true; }
    false
}

fn check_hardware() -> bool {
    // 1. CPU Cores
    // Most sandboxes allocate 1 core. Real users usually have >= 2 (even >= 4 nowadays).
    if num_cpus::get() < 2 {
        return true;
    }

    // 2. RAM
    // Sandboxes often have small RAM (2GB-4GB). Real gaming/mining rigs have > 8GB.
    let mut sys = System::new_all();
    sys.refresh_memory();
    let total_ram_gb = sys.total_memory() / 1024 / 1024; // KB -> MB -> GB (approx)
    
    // Threshold: < 3.5GB is suspicious (4GB sticks usually show ~3.8GB usable)
    if total_ram_gb < 3500 {
        return true;
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
    // TODO: Add Linux ptrace check if needed
    false
}
