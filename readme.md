# Not Wallpaper Engine


This project is a lightweight, modular, and production‑ready background rendering engine designed for Wayland environments. It supports media playback, scripting, plugins, adaptive performance control, and extensible rendering pipelines.

---

## ✨ Features

* 🧩 **Entity Component System (ECS)** core
* 🎬 **Media Engine** (video, image, web sources)
* 🎧 **Audio Spectrum Processing** (PipeWire‑ready)
* 🧠 **WASM Script Runtime** (sandboxed logic)
* 🌐 **Web Wallpaper Runtime**
* 🔌 **Plugin System**
* ⚡ **Adaptive Performance Controller**
* 🖥 **Wayland Background Backend**
* 🎨 **Render Graph Architecture**
* 🚀 Designed for daemon/service execution

---

## 🏗 Architecture Overview

```
Engine
 ├── ECS World
 ├── Render Graph
 ├── Renderer
 ├── Wayland Backend
 ├── Media Engine
 ├── Audio Engine
 ├── Physics Engine
 ├── Script Runtime (WASM)
 ├── Web Runtime
 ├── Plugin Host
 ├── Asset Manager
 └── Performance Controller
```

The engine runs as a continuous loop that:

1. Dispatches compositor events
2. Updates systems
3. Executes render graph
4. Presents frames

---

## 📦 Installation

### Requirements

* Rust (stable)
* Wayland compositor
* Linux (recommended)

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

Clone the repository:

```bash
git clone https://github.com/Ruischii/Not-so-much-Wallpaper-Engine.git
cd Not-so-much-wallpaper-engine
```

Build:

```bash
cargo build --release
```

Run:

```bash
cargo run --release
```

---

## 🚀 Running as a Daemon (Recommended)

Example **systemd user service**:

```
~/.config/systemd/user/engine.service
```

```ini
[Unit]
Description=Engine Wallpaper Runtime

[Service]
ExecStart=%h/.cargo/bin/engine
Restart=always

[Install]
WantedBy=default.target
```

Enable:

```bash
systemctl --user enable --now engine
```

---

## 🔌 Plugin System

Plugins implement:

```rust
trait Plugin {
    fn update(&mut self, dt: f32);
}
```

Plugins can:

* modify rendering
* react to audio
* control entities
* create effects

---

## 🧠 Script Runtime

WASM scripts allow safe runtime logic execution.

Planned capabilities:

* hot reload
* sandbox execution
* event bindings

---

## ⚙ Performance Modes

| Mode        | Description           |
| ----------- | --------------------- |
| Performance | Maximum visuals       |
| Balanced    | Default mode          |
| Battery     | Reduced FPS & effects |

---

## 📁 Suggested Project Layout

```
src/
 ├── main.rs
 └── engine.rs
```

---

## 🛣 Roadmap

* [ ] Vulkan renderer
* [ ] GPU video decode
* [ ] Hot reload assets
* [ ] Plugin marketplace
* [ ] Multi‑monitor support
* [ ] Scene editor

---

## 🤝 Contributing

Pull requests are welcome. Please keep modules decoupled and follow Rust idioms.

---

## 📜 License

MIT License

---
