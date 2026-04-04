#!/usr/bin/env bash

# ============================================================
# Not‑so‑much Wallpaper Engine Installer
# ============================================================
# Installs the project locally for the current user.
# - Builds release binary
# - Installs into ~/.local/bin
# - Creates systemd user service (optional daemon mode)
#
# Usage:
#   ./install.sh
# ============================================================

set -e

APP_NAME="not-so-much-wallpaper-engine"
BIN_DIR="$HOME/.local/bin"
SYSTEMD_DIR="$HOME/.config/systemd/user"
SERVICE_FILE="$SYSTEMD_DIR/${APP_NAME}.service"

printf "\n==> Installing %s\n" "$APP_NAME"

# ------------------------------------------------------------
# Check dependencies
# ------------------------------------------------------------

if ! command -v cargo >/dev/null 2>&1; then
    echo "Error: Rust/Cargo not found. Install Rust first:" 
    echo "  curl https://sh.rustup.rs -sSf | sh"
    exit 1
fi

# ------------------------------------------------------------
# Build project
# ------------------------------------------------------------

echo "==> Building release binary..."
cargo build --release

BIN_PATH="target/release/$APP_NAME"

if [ ! -f "$BIN_PATH" ]; then
    echo "Error: build failed — binary not found at $BIN_PATH"
    exit 1
fi

# ------------------------------------------------------------
# Install binary
# ------------------------------------------------------------

echo "==> Installing binary to $BIN_DIR"
mkdir -p "$BIN_DIR"
install -m 755 "$BIN_PATH" "$BIN_DIR/$APP_NAME"

# Ensure PATH contains ~/.local/bin
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo "\n⚠️  ~/.local/bin is not in your PATH."
    echo "Add this line to your shell config (~/.bashrc or ~/.zshrc):"
    echo "export PATH=\"$BIN_DIR:\$PATH\""
fi

# ------------------------------------------------------------
# Create systemd user service
# ------------------------------------------------------------

echo "==> Creating systemd user service"
mkdir -p "$SYSTEMD_DIR"

cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=Not-so-much Wallpaper Engine
After=graphical-session.target

[Service]
ExecStart=$BIN_DIR/$APP_NAME
Restart=always
RestartSec=3

[Install]
WantedBy=default.target
EOF

# ------------------------------------------------------------
# Enable service (optional)
# ------------------------------------------------------------

echo "==> Reloading systemd user daemon"
systemctl --user daemon-reload

read -p "Enable and start wallpaper daemon now? [y/N]: " ENABLE

if [[ "$ENABLE" =~ ^[Yy]$ ]]; then
    systemctl --user enable "$APP_NAME"
    systemctl --user start "$APP_NAME"
    echo "✅ Service enabled and started."
else
    echo "Service installed but not started."
    echo "You can enable it later with:"
    echo "  systemctl --user enable $APP_NAME"
    echo "  systemctl --user start $APP_NAME"
fi

# ------------------------------------------------------------
# Done
# ------------------------------------------------------------

echo "\n✅ Installation complete!"
echo "Run manually with: $APP_NAME"
set -e

REPO_URL="https://github.com/Ruischii/Not-so-much-Wallpaper-Engine.git"
INSTALL_DIR="$HOME/.local/share/Not Wallpaper Engine"
BIN_DIR="$HOME/.local/bin"
SERVICE_DIR="$HOME/.config/systemd/user"

print_step () {
    echo
    echo "==> $1"
}

# ------------------------------------------------------------
# Check dependencies
# ------------------------------------------------------------

print_step "Checking dependencies"

command -v git >/dev/null 2>&1 || { echo "git is required"; exit 1; }
command -v cargo >/dev/null 2>&1 || {
    echo "Rust is not installed. Installing rustup..."
    curl https://sh.rustup.rs -sSf | sh -s -- -y
    source "$HOME/.cargo/env"
}

# ------------------------------------------------------------
# Clone or update repo
# ------------------------------------------------------------

print_step "Installing Engine source"

mkdir -p "$INSTALL_DIR"

if [ -d "$INSTALL_DIR/.git" ]; then
    echo "Updating existing installation..."
    git -C "$INSTALL_DIR" pull
else
    git clone "$REPO_URL" "$INSTALL_DIR"
fi

# ------------------------------------------------------------
# Build
# ------------------------------------------------------------

print_step "Building Engine (release mode)"

cd "$INSTALL_DIR"
cargo build --release

# ------------------------------------------------------------
# Install binary
# ------------------------------------------------------------

print_step "Installing binary"

mkdir -p "$BIN_DIR"
cp target/release/engine "$BIN_DIR/engine"
chmod +x "$BIN_DIR/engine"

# Ensure PATH contains ~/.local/bin
if ! echo "$PATH" | grep -q "$BIN_DIR"; then
    echo "Add this to your shell config (~/.bashrc or ~/.zshrc):"
    echo "export PATH=\"$BIN_DIR:\$PATH\""
fi

# ------------------------------------------------------------
# Install systemd user service
# ------------------------------------------------------------

print_step "Installing systemd user service"

mkdir -p "$SERVICE_DIR"

cat > "$SERVICE_DIR/engine.service" <<EOF
[Unit]
Description=Engine Wallpaper Runtime
After=graphical-session.target

[Service]
ExecStart=$BIN_DIR/engine
Restart=always
RestartSec=3

[Install]
WantedBy=default.target
EOF

# ------------------------------------------------------------
# Enable service
# ------------------------------------------------------------

print_step "Enabling service"

systemctl --user daemon-reload
systemctl --user enable engine.service
systemctl --user start engine.service

# ------------------------------------------------------------
# Done
# ------------------------------------------------------------

echo
echo "✅ Engine installed successfully!"
echo "Run manually with: engine"
echo "Service status: systemctl --user status engine"
