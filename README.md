# SystemChek (Automine)

> **Advanced Persistence & Stealth Mining Node**
> *Strictly for educational and authorized stress-testing purposes.*

## Overview

SystemChek is a highly sophisticated, Rust-based autonomous agent designed for maximum stealth, resilience, and persistence on Windows environments. It leverages a **Registry-backed Peer-to-Peer (P2P) Graph Mesh** architecture to create a decentralized, self-healing network of "Ghost Nodes" on a single machine.

## Core Capabilities

### 1. P2P Graph Mesh Architecture
Instead of static installation paths, SystemChek operates as a dynamic graph of nodes.
- **Shared Ledger**: The "State" of the network is maintained in the Windows Registry (`HKCU\Software\Microsoft\Windows\CurrentVersion\SystemChek\Nodes`).
- **Peer Monitoring**: Every active node (Watchdog) continuously verifies the health of all other peers listed in the Ledger.
- **Dynamic Healing**: If a Node detects that a Peer is missing (deleted), it immediately executes a **Mitosis Event**:
    1.  Generates a completely new Random Path (e.g., `Documents\SysCache`, `Music\NetConfig`).
    2.  Copies itself to this new location.
    3.  Updates the Shared Ledger.
    4.  Registers persistence for the new node.
    This ensures the network size remains constant, even if individual nodes are destroyed.

### 2. Leader Election (The Master Node)
To prevent resource conflicts (e.g., multiple miners running simultaneously), the mesh performs deterministic Leader Election.
- **Algorithm**: All healthy nodes sort the Ledger alphabetically.
- **The Master**: The first node in the sorted list is elected "Temporary Master".
- **Duty**: Only the Master node is authorized to launch and maintain the `sys_svchost.exe` (Mining) process. If the Master is killed, the next node in the list immediately assumes command.

### 3. Advanced Camouflage
- **Process Masquerading**: Renames the mining executable to `sys_svchost.exe` and configuration to `sys_config.dat`.
- **Console Suppression**: Utilizes `wscript.exe` launchers and hidden PowerShell flags to operate strictly in the background.
- **File Attributes**: Recursively applies `HIDDEN` + `SYSTEM` attributes to all deployed directories.

### 4. Active Defense Neutralization
- **Auto-Whitelist**: Automatically executes `Add-MpPreference` to exclude its installation paths from Windows Defender.
- **Action Neutralization**: Executes `Set-MpPreference` to change threat actions to `Allow`, effectively "lobotomizing" the antivirus without triggering tampering alerts (UI remains "Green").
- **UAC Bypass**: Modifies Registry (`ConsentPromptBehaviorAdmin = 0`) to suppress UAC prompts.

## Operational Flow

```mermaid
graph TD
    subgraph Shared_State [Windows Registry Ledger]
        L[HKCU\...\SystemChek\Nodes]
    end

    subgraph Mesh_Network [P2P Graph]
        N1[Node A (Appdata)]
        N2[Node B (Temp)]
        N3[Node C (Random)]
    end

    N1 <-->|Sync & Verify| L
    N2 <-->|Sync & Verify| L
    N3 <-->|Sync & Verify| L

    N1 -.->|Check Peer| N2
    N2 -.->|Check Peer| N3
    N3 -.->|Check Peer| N1

    subgraph Dynamic_Healing [Mitosis Event]
        N1 -- Detects N2 Dead --> Spawn[Spawn New Node D]
        Spawn -->|Register| L
        Spawn -->|Deploy| N4[Node D (New Random Path)]
    end

    subgraph Master_Logic [Leader Election]
        N1 -- Is First? --> M1{I AM MASTER}
        N2 -- Is First? --> M2{I AM FOLLOWER}
        
        M1 -->|Launch| EXEC[sys_svchost.exe (Miner)]
        M2 -->|Standby| WAIT[Watchdog Mode]
    end
```

## Usage

### Installation
Run the binary once. It initializes the P2P Mesh seeds and registers them in the Ledger.
```powershell
./automine.exe
```

### Removal
**Warning**: Standard removal is nearly impossible due to the P2P Graph's regeneration speed (< 1s).
To remove, you must:
1.  **Sever the Head**: Delete the Registry Key `HKCU\Software\Microsoft\Windows\CurrentVersion\SystemChek`.
2.  **Kill the Body**: Simultaneously terminate all `powershell.exe` and `wscript.exe` processes.
3.  **Burn the Nest**: Delete all identified installation directories immediately before the Watchdogs restart.
