<div align="center">
  <img src="assets/app.svg" alt="ProcessHandler Logo" width="120" height="120">
  
  # ProcessHandler
  
  **Lightweight, reactive Windows process automation & system monitoring suite**
  
  [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?logo=opensourceinitiative&logoColor=white)](LICENSE)
  [![Platform](https://img.shields.io/badge/Platform-Windows%2010%2F11-0078D6?logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCIgZmlsbD0id2hpdGUiPjxwYXRoIGQ9Ik0wIDMuNDQ5TDkuNzUgMi4xdjkuNDVIMFYzLjQ0OXpNMCAxMi40NWg5Ljc1djkuNDVMMCAyMC41NTF2LTguMTAxek0xMS4yNSAxLjg5bDEyLjc1LTEuODl2MTEuNDVoLTEyLjc1VjEuODl6bTEyLjc1IDEwLjU2djExLjQ1bC0xMi43NS0xLjg5di05LjU2aDEyLjc1eiIvPjwvc3ZnPg==)](https://www.microsoft.com/windows)
  [![Rust](https://img.shields.io/badge/Rust-%3E%3D1.75-black?logo=rust&logoColor=white)](https://www.rust-lang.org)
  [![egui](https://img.shields.io/badge/egui-0.36-blue?logo=rust&logoColor=white)](https://github.com/emilk/egui)
</div>

---

## 📖 Overview

**ProcessHandler** is a fast, native Windows process automation service and task monitor built in 100% pure Rust. Designed as a dual-tier system, it decouples background execution from user interaction:

1. **System Tray Daemon**: An ultra-low overhead background service that listens reactively to Windows WMI process lifecycle events, manages schedules, and executes automated rule pipelines with zero window baggage.
2. **Interactive Desktop GUI**: A hardware-accelerated, high-DPI desktop interface powered by `egui` and `eframe`, communicating seamlessly with the daemon via high-throughput Windows Named Pipes (`\\.\pipe\ProcessHandler_daemon`).

Whether you need to boost game performance by binding companion utilities, safeguard your PC from rogue memory leaks, or automate daily workflow scripts, ProcessHandler delivers full control with near-zero resource utilization (<0.1% CPU, ~2MB RAM).

<div align="center">
  <img src="https://github.com/user-attachments/assets/6eb36b2e-5852-451b-abbe-d1bd2be790c7" width="100%">
  <img src="https://github.com/user-attachments/assets/3859f79f-23eb-4bed-8d07-a258b7ffdeb4" width="49%"> <img src="https://github.com/user-attachments/assets/f8e86dfc-f37e-48fc-8f26-95849b7f2282" width="49%">
</div>

---

## ✨ Key Features

* **Instant process lifecycle detection** via native Windows WMI event notifications and low-latency snapshotting
* **Flexible multi-action pipeline** combining process control, PowerShell/CMD scripts, webhooks, and delays
* **Deep Windows audio control** covering per-process volume/mute, master sound, and microphone devices (WASAPI)
* **Comprehensive trigger engine** spanning process events, scheduled timers, user idle states, and system power transitions
* **Companion process mode** with automated lifecycle coupling and graceful exit cleanup
* **Ultra-lightweight daemon architecture** with background tray operation (<0.1% CPU, ~2MB RAM) and named-pipe IPC
* **Portable rule sharing** via compressed, self-contained clipboard codes (single and batch import/export)

*Specifically:*

<details>
<summary><b>Process Monitoring & Detection</b></summary>

- **Event-Driven Subscriptions**: Direct Windows WMI hooks for instant process start/exit detection without CPU-heavy polling loops.
- **Fallback Polling Engine**: High-frequency snapshotting fallback for restricted or custom environments.
- **Native Icon Extraction**: Real-time extraction of high-resolution Win32 application icons rendered directly into the interface.

</details>

<details>
<summary><b>Automation & Rules Engine</b></summary>

- **Triggers**:
  - *Process Lifecycle*: Target application launch, termination, or running duration (exact match, wildcards, regex).
  - *Schedules & Alarms*: Fixed intervals, daily recurring schedules, and one-shot date/time alarms.
  - *System & Power Events*: User idle/active state, session lock/unlock, system sleep/wake, and AC/battery power transitions.
- **Action Pipelines**:
  - *Process Management*: Launch applications (custom arguments, working directory, admin rights, hidden mode), terminate process trees, or restart.
  - *Audio & Microphone Control*: Control master volume, per-application audio sessions (WASAPI mute/volume), and microphone recording endpoints.
  - *PowerShell & CMD Scripts*: Execute custom scripts and CLI commands with an integrated syntax-highlighting editor.
  - *Alerts & Webhooks*: Native Windows toast notifications and outbound HTTP REST webhooks (POST/GET/PUT).
  - *Flow & System*: Screen locking and configurable pause intervals between steps.
- **Execution & Safeguards**:
  - Non-blocking asynchronous task execution ensuring the daemon and UI never freeze.
  - Cooldown timers, execution counters, rule cloning, and pause toggles.
  - One-click clipboard import/export of rules via compressed portable payloads.

</details>

<details>
<summary><b>Companion Process Mode</b></summary>

- **Lifecycle Binding**: Attach companion utilities, overlays, or performance monitors to a primary game or application.
- **Automated Cleanup**: Automatically terminates companion processes and reverts system state when the primary application closes.

</details>

<details>
<summary><b>Architecture & System Integration</b></summary>

- **Dual-Tier Decoupled Architecture**: Lightweight background daemon (<0.1% CPU, ~2MB RAM) running continuously in the system tray, separated from the desktop GUI.
- **Named Pipes IPC**: Fast, thread-safe local communication between interface and daemon.
- **Tray & Startup Control**: Unobtrusive system tray icon with quick actions and one-click Windows startup registration.
- **In-App Auto-Updater**: Built-in GitHub Releases checker with zero-downtime binary updates.

</details>

<details>
<summary><b>User Interface & Workspace</b></summary>

- **Visual Studio**: Hardware-accelerated interface with Dark, Light, and Chocolate palettes, plus full English and Russian localization.
- **Interactive Rule Builder**: Drag-and-drop step reordering, real-time step execution highlighting, automatic target process icon detection, and builder draft preservation.
- **Action Catalog & Process Picker**: Fast in-context modal dialogs to browse available actions and select active processes without losing editor state.
- **Activity Feed**: Real-time diagnostic logs capturing trigger evaluations, action outputs, and system events.

</details>

---

## 🚀 Installation

### For End Users

#### Prerequisites
- **Windows 10 / 11** (64-bit)

To install, download the latest release from [**GitHub Releases**](https://github.com/psyattack/ProcessHandler/releases):
- **Portable Version (`ProcessHandler.exe`):** Place the executable in any convenient directory and run it.

> [!NOTE]
> When you run ProcessHandler for the first time, it automatically registers itself in the Windows startup list. You can always toggle the startup option in the settings window. (If you move the executable file, the startup may stop working; simply turn it off and then back on again.)

---

### For Developers

<details>
<summary><b>Development Setup</b></summary>

#### Prerequisites
- **Rust** (1.75+ recommended, MSVC toolchain)

#### Build & Run Commands

```powershell
# Clone the repository
git clone https://github.com/psyattack/ProcessHandler.git
cd ProcessHandler

# Run in development mode (spawns daemon and opens GUI)
cargo run

# Run only the GUI (connects to an existing background daemon)
cargo run -- --gui

# Run only the background daemon (tray mode)
cargo run -- --daemon

# Execute test suite
cargo test

# Run Clippy linter
cargo clippy --tests

# Build optimized release binary
cargo build --release
```

The compiled binary will be located at `target\release\ProcessHandler.exe`.

</details>

---

## 🛠️ Tech Stack

<table>
<tr>
<td width="50%">

### Frontend / UI
- **egui** (Immediate-mode GUI engine)
- **eframe** (Desktop windowing & rendering integration)
- **Custom Design Tokens & Themes** (Dark, Light, Chocolate palettes)
- **i18n Localization Engine** (Full English & Russian support)
- **Script Syntax Highlighter** (PowerShell & Batch lexer)
- **Win32 Shell GDI & image crate** (Native icon rasterization, PNG/JPEG decoding & textures)
- **arboard** (Cross-platform system clipboard integration)
- **Named Pipe Client** (Thread-safe IPC client)

</td>
<td width="50%">

### Backend / Engine
- **Rust** (High-performance, memory-safe systems language)
- **Tokio** (Asynchronous runtime for decoupled rule execution & IPC)
- **windows-rs** (Official Win32, COM & Core Audio bindings)
- **WMI** (Windows Management Instrumentation event listener)
- **Core Audio & WASAPI** (System volume & per-process)
- **reqwest** (Async HTTP client for GitHub releases & binary updates)
- **Named Pipes Server** (Multi-client background IPC listener)
- **Serde + Serde JSON** (Strongly-typed state persistence)
- **flate2 & Base64** (Zlib compression for portable rule sharing payloads)
- **Windows Registry API** (System autorun persistence)

</td>
</tr>
</table>

---

## 📂 Project Structure

<details>
<summary><b>View structure</b></summary>

```
ProcessHandler/
├── assets/                           # Application assets & branding
│   ├── fonts/                        # Bundled variable fonts (Archivo, GeistMono, Inter)
│   ├── app.ico                       # Win32 application icon
│   ├── app.png                       # High-res RGBA icon
│   └── app.svg                       # Vector logo
├── src/                              # Source code
│   ├── daemon/                       # Background daemon service
│   │   ├── actions.rs                # Execution handlers (Process, Master/Mic/Process Audio, Toast, Commands)
│   │   ├── engine.rs                 # Central automation state engine & rule evaluator
│   │   ├── ipc_server.rs             # Named pipe listener & connection worker
│   │   ├── system_events.rs          # Win32 idle time & AC/battery power monitor
│   │   ├── tray.rs                   # Win32 system tray manager, menu & message loop
│   │   └── watcher.rs                # WMI events & Toolhelp32 process snapshotting
│   ├── gui/                          # egui desktop user interface
│   │   ├── components/               # UI components
│   │   │   ├── icons.rs              # Win32 shell icon extractor, image loader & texture cache
│   │   │   ├── script_highlighter.rs # PowerShell & Batch syntax highlighter
│   │   │   └── widgets.rs            # Custom canvas controls (buttons, inputs, cards, drag handles, glyphs)
│   │   ├── views/                    # Application screens & panels
│   │   │   ├── action_catalog_modal.rs # Studio modal for categorized action browsing & search
│   │   │   ├── changelog_modal.rs    # GitHub release notes & changelog viewer
│   │   │   ├── header_bar.rs         # Custom window titlebar & controls
│   │   │   ├── inspector.rs          # Rule inspector, trigger & action editor (drag-and-drop)
│   │   │   ├── logs_view.rs          # Real-time event & action logs
│   │   │   ├── process_picker_modal.rs # In-context process picker modal with filter & sorting
│   │   │   ├── rules_view.rs         # Rule management list & cards
│   │   │   ├── script_modal.rs       # Expandable script editor modal
│   │   │   └── settings_view.rs      # Balanced 2x2 settings (System, Engine, Appearance, Updates)
│   │   ├── state.rs                  # GUI state management & IPC synchronization
│   │   └── theme.rs                  # Design system tokens (Dark, Light, Chocolate palettes)
│   ├── i18n/                         # Internationalization & localization
│   │   ├── en.rs                     # English string catalog
│   │   └── ru.rs                     # Russian string catalog
│   ├── ipc/                          # Inter-process communication
│   │   ├── client.rs                 # Named pipe IPC client
│   │   └── protocol.rs               # Request & response message types (Serde)
│   ├── autorun.rs                    # Windows Registry autorun integration
│   ├── config.rs                     # JSON configuration persistence & validation (theme, language, updates)
│   ├── daemon.rs                     # Daemon entrypoint & service lifecycle
│   ├── gui.rs                        # GUI entrypoint & eframe application loop
│   ├── i18n.rs                       # Internationalization loader & language switcher
│   ├── models.rs                     # Domain data models (Rule, Trigger, Action, Process)
│   ├── sharing.rs                    # Portable automation compression, encoding & clipboard sharing
│   ├── updater.rs                    # GitHub releases auto-update & self-replacement engine
│   └── main.rs                       # CLI dispatcher (--daemon, --gui, default)
├── build.rs                          # Windows resource compilation (icon, manifest)
├── Cargo.toml                        # Rust manifest & dependencies
└── LICENSE                           # MIT License
```

</details>

---

## 📝 Configuration

Configuration and state files are located in:  
**`%LOCALAPPDATA%\ProcessHandler\`**

| File / Directory | Description |
|------|-------------|
| `config.json` | Application settings, user rules, poll interval, window geometry, color theme (`dark`, `light`, `chocolate`), language (`english`, `russian`), startup update checks, and autorun state |

> [!NOTE]
> Event and action execution logs are maintained in an in-memory ring buffer (up to 100 entries) in RAM and streamed via IPC directly to the GUI without incurring disk I/O wear.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).

---

## 🙏 Credits

- **UI/UX Idea:** [ArtCraft Launcher](https://github.com/storytold/craft-launcher)

---

## ⚠️ Disclaimer

ProcessHandler interacts directly with Windows system processes and administrative APIs. When configuring process termination or priority rules, ensure critical operating system processes are excluded from automated termination.

---

## 💬 Support & Contributing

For issues, questions, or feature requests, please open an issue on [**GitHub Issues**](https://github.com/psyattack/ProcessHandler/issues).

---
