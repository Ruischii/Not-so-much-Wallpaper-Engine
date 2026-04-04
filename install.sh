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
