use std::env;
use std::path::{Path, PathBuf};
use crate::common::constants::INSTALL_DIR_NAME;

#[cfg(windows)]
pub fn get_userprofile() -> PathBuf {
    PathBuf::from(env::var("USERPROFILE").unwrap_or_else(|_| String::from("C:\\Users\\Default")))
}

#[cfg(not(windows))]
pub fn get_userprofile() -> PathBuf {
    PathBuf::from(env::var("HOME").unwrap_or_else(|_| String::from("/tmp")))
}

pub fn get_appdata_dir() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(env::var("APPDATA").unwrap_or_else(|_| String::from("C:\\Users\\Default\\AppData\\Roaming")))
            .join(INSTALL_DIR_NAME)
    } else {
        get_userprofile().join(format!(".config/{}", INSTALL_DIR_NAME))
    }
}

pub fn get_localappdata_dir() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(env::var("LOCALAPPDATA").unwrap_or_else(|_| String::from("C:\\Users\\Default\\AppData\\Local")))
            .join(INSTALL_DIR_NAME)
    } else {
        get_userprofile().join(format!(".local/share/{}", INSTALL_DIR_NAME))
    }
}

pub fn get_temp_install_dir() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(env::var("TEMP").unwrap_or_else(|_| String::from("C:\\Windows\\Temp")))
            .join(INSTALL_DIR_NAME)
    } else {
        PathBuf::from("/tmp").join(INSTALL_DIR_NAME)
    }
}

pub fn get_all_install_dirs() -> Vec<PathBuf> {
    vec![
        get_appdata_dir(),
        get_localappdata_dir(),
        get_temp_install_dir(),
    ]
}

#[cfg(windows)]
pub fn set_hidden_recursive(path: &Path) -> std::io::Result<()> {
    use std::fs;
    use std::os::windows::prelude::*;
    use winapi::um::fileapi::{SetFileAttributesW};
    use winapi::um::winnt::{FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_SYSTEM};

    if !path.exists() {
        return Ok(());
    }

    let path_str: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    
    unsafe {
        SetFileAttributesW(path_str.as_ptr(), FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM);
    }

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            set_hidden_recursive(&entry.path())?;
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn set_hidden_recursive(_path: &Path) -> std::io::Result<()> {
    Ok(())
}
