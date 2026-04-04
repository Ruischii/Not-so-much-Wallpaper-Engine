# Not‑so‑much Wallpaper Engine

A **Wayland‑native wallpaper runtime** written in Rust.

This project aims to become a lightweight, extensible alternative to Wallpaper Engine for Linux — designed around modern compositor standards, performance, and modular engine architecture.

> ⚠️ **Project Status:** Early prototype. Not production‑ready yet.

---

## ✨ Goals

* Native **Wayland background rendering** (no window hacks)
* Low CPU/GPU usage while idle
* Plugin‑driven wallpaper system
* Video, shader, and web wallpapers
* Daemon‑based runtime
* Safe scripting via WASM sandboxing

---

## 🧠 Architecture Overview

The project is structured like a small real‑time engine rather than a simple wallpaper player.

```
Engine
 ├── ECS World
 ├── Runtime Loop
 ├── Renderer
 ├── Wayland Backend
 ├── Media System
 ├── Plugin Host
 └── Performance Controller
```

### Core Concepts

**Engine**
Manages lifecycle, scheduling, and system execution.

**Renderer**
Responsible for drawing wallpapers using GPU acceleration.

**Wayland Backend**
Creates compositor‑aware background surfaces using layer‑shell.

**Plugins**
Extend wallpaper behavior without modifying the core engine.

---

## 🚧 Current Status

What works:

* Project structure
* Engine foundation
* Buildable Rust project

What is **not implemented yet**:

* Wayland layer‑shell wallpaper surface
* Video playback
* Web wallpapers
* Plugin runtime
* Daemon service management

This repository currently represents the **engine foundation**.

---

## 📦 Requirements

* Linux (Wayland session required)
* Rust (stable)
* Cargo

Install Rust if needed:

```bash
curl https://sh.rustup.rs -sSf | sh
```

---

## 🔧 Build

Clone the repository:

```bash
git clone https://github.com/Ruischii/Not-so-much-Wallpaper-Engine.git
cd Not-so-much-Wallpaper-Engine
```

Build:

```bash
cargo build --release
```

Binary output:

```
target/release/not-so-much-wallpaper-engine
```

---

## ▶️ Run (Prototype Mode)

Currently the program runs as a normal application for development testing:

```bash
cargo run
```

It does **not yet replace your desktop wallpaper**.

---

## ⚙️ Planned Daemon Mode

Future versions will run as a background service:

* persistent wallpaper runtime
* automatic startup via systemd user service
* IPC control interface

Example (planned):

```bash
systemctl --user enable wallpaper-engine
systemctl --user start wallpaper-engine
```

---

## 🗺️ Roadmap

### Phase 1 — Wallpaper Proof

* [ ] Wayland layer‑shell integration
* [ ] Fullscreen background surface
* [ ] Render solid color frame

### Phase 2 — Rendering

* [ ] GPU renderer (wgpu)
* [ ] Shader wallpapers
* [ ] Multi‑monitor support

### Phase 3 — Media

* [ ] Video wallpapers
* [ ] Audio visualization

### Phase 4 — Extensibility

* [ ] Plugin API
* [ ] WASM sandbox runtime

---

## 🤝 Contributing

Contributions are welcome.

Recommended areas:

* Wayland integration
* Rendering systems
* Performance optimization
* Plugin architecture

Please open an issue before large changes.

---

## 📄 License

MIT License

---

## 💡 Vision

The goal is not to clone Wallpaper Engine directly, but to build a **modern Linux‑native wallpaper runtime** designed around Wayland and modular engine design.

If you are interested in graphics engines, Linux desktop systems, or Rust real‑time software — this project is for you.
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
