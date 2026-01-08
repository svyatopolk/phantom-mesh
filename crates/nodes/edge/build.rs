use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Look for master.pub in the workspace root or debug target area
    // Just looking relatively to the crate root helps if run from workspace.
    // Try reliable locations.
    
    // We assume the user ran 'master keygen' which puts 'master.key' and 'master.pub' 
    // in the current directory where they ran it. Usually the workspace root.
    // The build script runs in target/.../build/... 
    // CARGO_MANIFEST_DIR is crates/bot.
    
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = Path::new(&crate_dir).parent().unwrap().parent().unwrap().parent().unwrap();
    // Use the central keys directory
    let pub_key_path = workspace_root.join("keys").join("master.pub");

    println!("cargo:rerun-if-changed={}", pub_key_path.display());

    if pub_key_path.exists() {
        let content = fs::read_to_string(&pub_key_path).expect("Failed to read master.pub");
        println!("cargo:rustc-env=MASTER_PUB_KEY={}", content.trim());
    } else {
        // Fallback or Panic
        if let Ok(val) = env::var("MASTER_PUB_KEY") {
             println!("cargo:rustc-env=MASTER_PUB_KEY={}", val);
        } else {
             // User requested automation via script. If we build manually without keys, logic dictates we fail or warn.
             println!("cargo:warning=master.pub NOT FOUND at {}. Run 'scripts/generate.sh' first.", pub_key_path.display());
             println!("cargo:rustc-env=MASTER_PUB_KEY=DEADBEEF00000000000000000000000000000000000000000000000000000000");
        }
    }

    // Inject Swarm Key
    let swarm_key_path = workspace_root.join("keys").join("swarm.key");
    if swarm_key_path.exists() {
        let content = fs::read_to_string(&swarm_key_path).expect("Failed to read swarm.key");
        println!("cargo:rustc-env=SWARM_KEY={}", content.trim());
    } else {
        println!("cargo:rustc-env=SWARM_KEY=0000000000000000000000000000000000000000000000000000000000000000"); // Fail open/secure?
    }
}
