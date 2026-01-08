use sysinfo::{System, Networks};
use num_cpus;

#[cfg(windows)]
use winapi::um::debugapi::IsDebuggerPresent;



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
        if mac.starts_with("00:05:69") || mac.starts_with("00:0C:29") || mac.starts_with("00:1C:14") || mac.starts_with("00:50:56") // VMware
        || mac.starts_with("00:1C:42") // Parallels
        || mac.starts_with("00:15:5D") // Hyper-V
        || mac.starts_with("08:00:27") // VirtualBox
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

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn check_timing() -> bool {
    // Measure time to execute generic instructions
    // If > threshold, step-over suspected
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::_rdtsc;
    #[cfg(target_arch = "x86")]
    use std::arch::x86::_rdtsc;

    unsafe {
        let t1 = _rdtsc();
        // Some operations
        let mut x = 0;
        for _ in 0..100 { x += 1; }
        let t2 = _rdtsc();
        
        // Threshold: typical execution is very fast (< 1000 cycles for this loop)
        // If stepping, it takes millions.
        if (t2 - t1) > 100_000 {
            return true;
        }
    }
    false
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn check_timing() -> bool { false }

fn trap_debugger() {
    // Int 3 Breakpoint
    // If Debugger is present, it pauses.
    // If NOT present, it causes Exception (Crash).
    // This is extreme. We might want to use it carefully.
    // For now, let's just use it as part of "Self-Destruct" if analysis detected.
    // OR: Use IsDebuggerPresent logic.
    // If user asked for "Trap Debugger", we act conditionally.
    
    // NOTE: Intentionally omitted from general check to prevent random crashes on innocent systems
    // unless we wrap in SEH (which is hard in Rust std).
    // We will rely on Timing Check for now as it's cleaner.
}

pub fn is_analysis_environment() -> bool {
    if check_hardware() { return true; }
    if check_uptime() { return true; }
    if check_debugger() { return true; }
    if check_mac_oui() { return true; }
    if check_timing() { return true; }
    false
}
