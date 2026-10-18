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
  <img src="https://github.com/user-attachments/assets/aed3d00b-0900-4943-8c79-a048bfdadedc" width="49%"> <img src="https://github.com/user-attachments/assets/3531872c-a57b-4bfc-ba6b-943f0f4678b1" width="49%">
  <img src="https://github.com/user-attachments/assets/50e4c8b5-88f8-425d-85c3-6d301ee51e7a" width="49%"> <img src="https://github.com/user-attachments/assets/0d0021eb-757f-42d4-9080-f2e5e604e978" width="49%">
</div>

---

## ✨ Key Features

* **Instant process lifecycle detection** via native Windows WMI event notifications
* **Flexible trigger & action pipeline** supporting multi-step automations with per-process audio control
* **Live execution tracking** with real-time pipeline step highlighting and concurrency counters
* **Portable automation sharing** via compact, compressed clipboard codes (single & batch import/export)
* **Custom rule icon system** with automatic target process detection and Win32 icon extraction
* **Companion process mode** with automated startup and exit cleanup
* **Interactive process manager** with real Win32 icon extraction and metrics
* **Modern visual studio & themes** with Dark, Light, and Chocolate palettes in English & Russian
* **Built-in auto-updater** with GitHub release checks and changelog modal
* **Zero-spin background daemon** sitting quietly in the system tray

*Specifically:*

<details>
<summary><b>Process Monitoring & Detection</b></summary>

- **WMI Event Subscriptions**: Subscribes directly to Windows Management Instrumentation (`__InstanceCreationEvent` and `__InstanceDeletionEvent` for `Win32_Process`) for instant, event-driven launch and termination detection.
- **Fallback Polling Engine**: High-frequency snapshotting via `CreateToolhelp32Snapshot` for constrained environments or custom polling intervals.
- **Win32 Icon Rasterization**: Extracts high-fidelity application icons on-the-fly (`ExtractIconExW` + GDI DIBs with 1-bit transparency mask support and proportional aspect ratio scaling) rendered directly into the UI texture cache.

</details>

<details>
<summary><b>Automation & Rules Engine</b></summary>

- **Comprehensive Triggers**:
  - *Process Lifecycle*: Target process launch, process termination, or sustained running time.
  - *Time & Schedules*: Fixed intervals (seconds/minutes/hours), daily schedules at exact times, one-shot date/time alarms, app startup, and app shutdown.
  - *System Events*: User idle state (custom inactivity threshold), user resume active, AC power plug-in, and battery power switch.
- **Multi-Action Pipelines**:
  - *Process Management*: Launch executable (custom arguments, working directory), terminate process (graceful or forced kill), restart process.
  - *Per-Process Audio Control (WASAPI)*: Direct per-application audio session management via Windows WASAPI (`IAudioSessionManager2` & `ISimpleAudioVolume`). Mute, unmute, toggle mute, or set volume (0–100%) for specific processes or wildcards, with automatic retry polling for newly launched apps.
  - *Master Audio & Multimedia*: Master system sound control via Windows Core Audio `IAudioEndpointVolume` (Mute, Unmute, Toggle Mute, Set Volume 0–100%) and frequency-tuned audible beeps.
  - *System Alerts*: Native Windows balloon / toast notifications with custom title and body text.
  - *Shell Commands & Scripts*: Execute custom PowerShell commands, CLI scripts, or batch files with dynamic parameter expansion, supported by an expandable modal editor with syntax highlighting.
  - *Timing Delays*: Configurable pause intervals between pipeline execution steps.
- **Rule Safeguards & Execution**:
  - *Non-Blocking Asynchronous Engine*: Rule actions and delays execute in decoupled background Tokio tasks, ensuring heavy commands or delays never freeze the daemon or the UI.
  - *Live Execution Tracking*: Daemon tracks active rule execution IDs, running pipeline steps, and concurrent executions (`active_count`) in real time, dynamically accelerating UI refresh rates (80ms sync intervals) during execution.
  - *Portable Rule Sharing*: One-click sharing of individual automations or full automation libraries using compressed, self-contained base64 clipboard payloads (`PH1:` format) with automatic conflict resolution upon import.
  - *Execution Controls*: Configurable cooldown intervals to prevent rapid re-trigger storms, execution counters, one-click rule cloning, and non-destructive pause toggles.

</details>

<details>
<summary><b>Companion Process Mode</b></summary>

- **Coupled Lifecycle**: Attach one or more companion utilities to a parent application (e.g., launching fan control, overlay recorders, or audio tools alongside games).
- **Graceful Cleanup**: Automatically executes specialized on-exit actions when the primary application closes, shutting down companion processes and reverting system states.

</details>

<details>
<summary><b>Architecture & IPC</b></summary>

- **Windows Named Pipes**: Reliable, bidirectional client-daemon communication over `\\.\pipe\ProcessHandler_daemon` using strongly-typed Serde JSON payloads.
- **Single-Instance Enforcement**: Global Windows mutex prevents duplicate daemon instances; running an extra instance brings the existing GUI window to the foreground.
- **System Tray Integration**: Unobtrusive tray icon with quick-action context menu (Open GUI, Exit), double-click activation, and dynamic language localization.
- **Registry Autorun**: Seamless 1-click startup registration in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
- **Integrated Auto-Updater**: Built-in GitHub Releases checker, streaming binary downloads, and zero-downtime in-place self-replacement with automated daemon and GUI restarts.

</details>

<details>
<summary><b>User Interface & Design</b></summary>

- **Design System & Themes**: Sleek industrial aesthetic with custom typography (Archivo, Inter, GeistMono), subtle hairline borders, and three built-in color themes (**Dark**, **Light**, and **Chocolate**) with instant live switching.
- **Full Internationalization (i18n)**: Comprehensive English and Russian localization covering all interface views, inspector controls, logs, settings, modals, and tray menus.
- **Interactive Rule Builder**:
  - *Animated Drag-and-Drop*: Reorder pipeline action steps smoothly with 6-dot drag handles, drop slot indicators, live previews, and viewport edge auto-scrolling.
  - *Live Step Highlighting*: Highlights currently running actions in real time directly within both the rule card summaries and the Inspector pipeline editor.
  - *Custom Rule Icons & Auto-Detection*: Assign custom icons (`.exe`, `.ico`, `.dll`, or image files) to rules, or enable automatic icon discovery from target processes upon initial trigger, with contextual Phosphor icon fallbacks.
  - *Step Cloning*: One-click action step duplication directly beneath the source step.
  - *Modal Script Editor*: Centered expandable modal for PowerShell and Batch scripts with custom syntax highlighting, live two-way sync, and backdrop dimming.
- **Release Changelog Modal**: In-app viewer for GitHub release notes with rich Markdown/HTML formatting and 1-click update installation.
- **Live Process Table**: Filter by application category (All, Apps, Background, Windows) and sort by Name, PID, Memory, or CPU.
- **Execution Log Inspector**: Real-time diagnostic activity feed capturing trigger evaluations, action results, and system events.

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
│   │   ├── actions.rs                # Execution handlers (Process, Master Audio, Process Audio, Toast, Commands)
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
│   │   │   ├── changelog_modal.rs    # GitHub release notes & changelog viewer
│   │   │   ├── header_bar.rs         # Custom window titlebar & controls
│   │   │   ├── inspector.rs          # Rule inspector, trigger & action editor (drag-and-drop)
│   │   │   ├── logs_view.rs          # Real-time event & action logs
│   │   │   ├── processes_view.rs     # Live process table with search & sorting
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
