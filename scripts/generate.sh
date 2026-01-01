#!/bin/bash
set -e

# Configuration
KEY_DIR="keys"
KEY_PATH="$KEY_DIR/master.key"

echo "[*] AutoMine Key Generator & Builder (Phase 13: Tor Mesh)"
mkdir -p "$KEY_DIR"

echo "[1] Building Ghost Tool..."
cargo build --release --bin ghost -p ghost

echo "[2] Generating Keys in $KEY_DIR..."
GHOST_BIN="./target/release/ghost"
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

echo "[3] Building Node (with Injected Key)..."
export MASTER_PUB_KEY="$PUB_KEY"
cargo build --release --bin node -p node

echo "[OK] Build Complete."
echo " -> Ghost Key: $KEY_PATH (KEEP PRIVATE)"
echo " -> Ghost Pub: ${KEY_PATH}.pub (Injected into Node)"
echo ""
echo "To control the Mesh:"
echo "  $GHOST_BIN broadcast --bootstrap 'onion_address_here' --key '$KEY_PATH' --cmd 'ping'"
