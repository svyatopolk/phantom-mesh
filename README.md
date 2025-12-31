# SystemChek (Automine)

> **Advanced Persistence & Stealth Mining Node**
> *Strictly for educational and authorized stress-testing purposes.*

## Overview

SystemChek is a highly sophisticated, Rust-based autonomous agent designed for maximum stealth, resilience, and persistence on Windows environments. It masquerades as critical system processes (`sys_svchost.exe`) and leverages a multi-layered redundancy architecture to survive active removal attempts.

## Core Capabilities

### 1. Advanced Camouflage ("Ghost Process")
- **Process Masquerading**: Renames the mining executable to `sys_svchost.exe` and configuration to `sys_config.dat`.
- **Console Suppression**: Utilizes `wscript.exe` launchers and hidden PowerShell flags to operate strictly in the background with zero visual footprint.
- **File Attributes**: Recursively applies `HIDDEN` + `SYSTEM` attributes to all deployed directories and files.

### 2. Triple-Layer Redundancy
The system simultaneously deploys to three independent system directories:
1.  `%APPDATA%\SystemChek` (Roaming)
2.  `%LOCALAPPDATA%\SystemChek` (Local)
3.  `%TEMP%\SystemChek`

### 3. Active Defense Neutralization
- **Auto-Whitelist**: Automatically executes `Add-MpPreference` to exclude its installation paths from Windows Defender on first run.
- **Action Neutralization**: Executes `Set-MpPreference` to change default threat actions (High/Severe/Moderate) to `Allow`. This effectively "lobotomizes" the antivirus, keeping the UI active ("Green") while preventing it from deleting the agent.
- **UAC Bypass**: Modifies Registry (`ConsentPromptBehaviorAdmin = 0`) to suppress User Account Control prompts for Administrator actions.

### 4. Self-Healing Watchdogs
Two PowerShell scripts (`sys_monitor.ps1` and `sys_helper.ps1`) run in infinite loops across all deployed locations.
- **Cross-Monitoring**: Each location monitors the existence of the others.
- **Auto-Restoration**: If a file or directory is missing in Location A, Location B immediately replicates itself to restore Location A.
- **Process Revival**: If the main `sys_svchost.exe` process is terminated, it is instantly restarted.

### 5. Polymorphic Persistence ("Immortality Mode")
If the system detects a *hard deletion* (directory removal), it triggers a **Migration Event**:
1.  **Relocation**: Copies itself to a random new location (e.g., `Documents\NetConfig`, `Music\SysDriver`).
2.  **Registry Mutation**: Generates a new random Registry `Run` key (e.g., `HKCU\...\Run\Windows_NetConfig`).
3.  **Path Injection**: Appends the new location to the User's `PATH` environment variable.
This ensures that even if the known locations are scrubbed, the system survives in a new, unpredictable location.

## Operational Flow

```mermaid
graph TD
    A[Installer (automine.exe)] -->|1. Setup| B(Staging Area %TEMP%)
    B -->|2. Download & Rename| C{Distribute}
    C -->|Copy| D[%APPDATA%\SystemChek]
    C -->|Copy| E[%LOCALAPPDATA%\SystemChek]
    C -->|Copy| F[%TEMP%\SystemChek]
    
    D --> G[Apply Hidden/System Attributes]
    E --> G
    F --> G
    
    G --> H[Defense Evasion]
    H -->|PowerShell| I[Add Defender Exclusion]
    H -->|PowerShell| J[Neutralize Defender Actions]
    H -->|Registry| K[Disable UAC Prompts]
    
    H --> L[Persistence]
    L -->|Registry| M[HKCU Run Key "Automine"]
    
    M --> N[Watchdog Activation]
    N --> O[sys_monitor.ps1]
    N --> P[sys_helper.ps1]
    
    O -->|Monitor| Q(sys_svchost.exe)
    P -->|Monitor| Q
    
    subgraph Self_Healing_Loop
        O -.->|Check| E
        O -.->|Check| F
        P -.->|Check| D
        P -.->|Check| F
        
        E -->|Restore if Missing| D
        D -->|Restore if Missing| E
    end
    
    subgraph Polymorphism
        D -- Deleted? --> T[Migrate Random Location]
        T --> U[New Random Path]
        T --> V[New Random Registry Key]
    end
```

## Usage

### Installation
Run the binary once. It requires no arguments.
```powershell
./automine.exe
```
*Note: Run as Administrator for full defense neutralization capabilities.*

### Status Check
```powershell
./automine.exe status
```

### Removal
**Warning**: Standard removal is difficult due to self-healing.
To remove, you must simultaneously terminate all `powershell.exe` and `sys_svchost.exe` processes and delete all 3 directories (`AppData`, `Local`, `Temp`) within the <10s regeneration window.

```powershell
./automine.exe uninstall
```
*(The native uninstall command attempts to automate this race condition).*
