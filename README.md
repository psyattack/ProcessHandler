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

1. **System Tray Daemon**: An ultra-low overhead background service that listens reactively to system events, process lifecycles, and triggers, managing schedules and executing automated rule pipelines with zero window baggage.
2. **Interactive Desktop GUI**: A hardware-accelerated, high-DPI desktop interface powered by `egui` and `eframe`, communicating seamlessly with the daemon via high-throughput Windows Named Pipes (`\\.\pipe\ProcessHandler_daemon`).

Whether you need to boost game performance by binding companion utilities, safeguard your PC from rogue memory leaks, or automate daily workflow scripts, ProcessHandler delivers full control with near-zero resource utilization (~0.1% CPU, ~2MB RAM).

<div align="center">
  <img src="https://github.com/user-attachments/assets/777a1893-7a49-45bf-ad52-84375a5ef808" width="100%">
</div>

---

## ✨ Key Features

* **Event-driven process detection** via native Windows WMI subscriptions and low-latency snapshotting
* **Versatile trigger engine** spanning manual on-demand execution, process lifecycles, window focus, CPU/RAM thresholds, global hotkeys, file watcher, webhooks, system events, and schedules
* **Pre-conditions & logic branching** with multi-condition checks (running apps, window focus, time/days, system events, resource thresholds, file watcher), AND/OR chaining, and dynamic If/Else action flows
* **Flexible action pipeline** covering process control, window management, screen overlays (HUD), CPU/memory tuning, Discord Rich Presence, archive compression, filesystem operations, scripts, macros, audio devices, and webhooks
* **Overlay Studio** for creating interactive on-screen HUDs, custom crosshairs, telemetry badges, progress bars, and vector shapes with anti-capture protection (`WDA_EXCLUDEFROMCAPTURE`)
* **Embedded Lua 5.4 runtime** (`mlua`) with dedicated `ph` API alongside PowerShell and Batch scripting
* **Macro Studio** with hardware keyboard/mouse recording, in-game 3D camera rotation support, timeline editing, loop control, and anti-bot humanization
* **Deep Windows audio & device control** covering master & per-process volume/mute, channel balance, audio format switching (sample rate & bit depth), and default devices (WASAPI)
* **Companion process mode** with automated lifecycle coupling and graceful exit cleanup
* **Ultra-lightweight daemon architecture** with optional stealth tray mode, process masking, background operation (<0.1% CPU, ~2MB RAM), and named-pipe IPC
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
  - *Manual On-Demand*: Instant execution via GUI button or linked hotkey without background polling.
  - *Process Lifecycle*: Target application launch, termination, or duration (exact match, wildcard, regex).
  - *Window Focus*: Gain or loss of active foreground window focus.
  - *Resource Thresholds*: Sustained high CPU (%) or RAM (MB) usage limits.
  - *Global Hotkeys*: System-wide shortcut listener with toggle mode, continuous loop mode, hold mode, and repeat-while-held execution.
  - *File Watcher*: File and folder creation, modification, or deletion with extension filters.
  - *Local Webhooks*: Inbound HTTP REST requests (`http://127.0.0.1:48123/api/v1/trigger/{endpoint}`).
  - *Schedules & Alarms*: Fixed intervals, daily recurring schedules, monthly triggers (specific day of month), and one-shot date/time alarms.
  - *System & Power*: User idle/active state, session lock/unlock, sleep/wake, AC/battery transitions, hardware device connect/disconnect, monitor power, and network status.
  - *Trigger Library*: Interactive modal catalog with instant keyword search for selecting triggers.
- **Rule Pre-conditions**:
  - *Condition Catalog*: Categorized modal selector with instant keyword search.
  - *Gating Criteria*: Running process, active window focus, time window & days of week, system events (idle time, session lock, sleep/wake, AC/battery, display, devices, network), resource thresholds (CPU % and RAM MB per process), and file/folder existence checks with extension filters.
  - *Chainable Logic*: Combine multiple conditions with boolean AND / OR operators.
- **Action Pipelines**:
  - *Logic & Branching*: Dynamic If / Else If / Else actions with condition checks to execute conditional sub-pipelines.
  - *Screen Overlay (HUD)*: GPU-accelerated HUD overlay over apps and games (custom crosshairs with center dot, dynamic text with tokens, badges, progress bars, images, and vector shapes) with click-through, anti-capture streaming protection (`WDA_EXCLUDEFROMCAPTURE`), and interactive on-click triggers.
  - *Process Control*: Launch (custom arguments, working dir, admin, hidden mode), terminate process trees, or restart.
  - *Window Management*: Minimize, maximize, restore, hide, show, close, transparency, always-on-top, and screen positioning.
  - *Process & Memory Tuning*: Dynamically adjust CPU priority (Realtime to Idle), set core affinity masks, and trim RAM working sets with configurable soft limits and active window skip safeguards.
  - *Discord Rich Presence*: Broadcast custom game and app status directly to Discord with activity details, timers, custom asset images, interactive buttons, and auto-clear on exit.
  - *Archive Compression & Extraction*: Pack and unpack files or directories across multiple formats (Zip, 7z, Tar, Tar.gz, Rar) with overwrite protection.
  - *Filesystem Operations*: Create files and folders, copy/clone, rename/move, delete, write and append text with dynamic token interpolation.
  - *Scripting & Automation*: Embedded Lua 5.4 (`ph` API), PowerShell, and Batch scripts with token interpolation (`{pid}`, `{process}`, `{window_title}`, `{file_path}`).
  - *Macro Playback*: Recorded keyboard and mouse sequences (with in-game 3D camera turns, DirectInput scancodes, and instant stuck-key release on cancellation), speed multipliers, loop counts, and organic jitter.
  - *Audio & Device Control*: System and per-app volume/mute, microphone levels, independent L/R channel balance, audio device format switching (sample rate & bit depth), custom beep tones, and default playback/recording device switching.
  - *Alerts & Webhooks*: Windows toast notifications and outbound HTTP REST calls (POST/GET/PUT).
  - *System Power & Delays*: System lock, sleep, display off, shutdown, and configurable pause intervals between steps.
- **Execution & Safeguards**:
  - Non-blocking asynchronous task execution with per-step action testing directly in the editor.
  - Cooldowns, execution limits (`max_executions`), execution counters, rule cloning, and multi-level Undo/Redo (Ctrl+Z / Ctrl+Y).
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
- **Named Pipes IPC**: Fast, thread-safe local communication between interface and daemon (`\\.\pipe\ProcessHandler_daemon`).
- **Tray & Startup Control**: Unobtrusive system tray icon with quick actions, optional stealth mode (hidden tray), process masking in Task Manager, and one-click Windows startup registration.
- **In-App Auto-Updater**: Built-in GitHub Releases checker with zero-downtime binary updates.

</details>

<details>
<summary><b>User Interface & Workspace</b></summary>

- **Visual Studio**: Hardware-accelerated interface with Dark, Light, and Chocolate palettes, plus full English and Russian localization.
- **Interactive Rule Builder**: Drag-and-drop step reordering, real-time step execution tracking, target process icon detection, and multi-step Undo/Redo.
- **Overlay Studio**: Visual WYSIWYG editor with 1:1 monitor canvas, pan/zoom navigation, snap-to-grid, element rotation, and live calibration.
- **Trigger & Condition Catalogs**: In-context modal libraries with instant search for discovering capabilities and adding triggers or pre-conditions.
- **Macro Studio & Editor**: Built-in input recorder with desktop cursor & in-game camera capture, one-click coordinate mode toggling, timeline editing, speed/loop settings, and organic motion controls.
- **Action Catalog & Process Picker**: Fast in-context modal dialogs to browse actions and pick active processes without losing editor draft.
- **Analytics & Telemetry**: Monospace event stream with live KPI metrics (Total Executions, Active Rules, System Uptime) and category filters.
- **Built-in Documentation**: In-app GitBook-style guide with architecture overview, shortcut cheat sheets, and practical recipes.

</details>

---

## 🚀 Installation

### For End Users

#### Prerequisites
- **Windows 10 / 11** (64-bit)

To install, download the latest release from [**GitHub Releases**](https://github.com/psyattack/ProcessHandler/releases) and extract the downloaded archive to a convenient location on your drive.

> [!IMPORTANT]
> **Administrator Privileges & In-Game Hotkeys**: If you plan to trigger hotkeys, macros, or automate actions in games or programs running with elevated (administrator) privileges, ProcessHandler must also be **Run as administrator**. Under Windows UIPI (User Interface Privilege Isolation), non-elevated background services cannot intercept global keyboard shortcuts or dispatch simulated input (SendInput) into elevated windows.

> [!NOTE]
> When you run ProcessHandler for the first time, it automatically registers itself in the Windows startup list. You can always toggle the startup option in the settings window. (If you move the executable file, the startup may stop working; simply turn it off and then back on again.)

---

### For Developers

<details>
<summary><b>Development Setup</b></summary>

#### Prerequisites
- **Rust** (1.75+)

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

# Build optimized release binary
cargo build --release
```

</details>

<details>
<summary><b>Tech Stack</b></summary>

#### Frontend / UI
- **egui**
- **eframe**

#### Backend / Engine
- **Tokio**
- **mlua**
- **windows-rs**
- **reqwest**
- **Serde + Serde JSON**

</details>

<details>
<summary><b>Project Structure</b></summary>

```
ProcessHandler/
├── assets/                             # Application assets & branding
│   ├── fonts/                          # Bundled variable fonts (Archivo, GeistMono, Inter)
│   ├── app.ico                         # Win32 application icon
│   ├── app.png                         # High-res RGBA icon
│   └── app.svg                         # Vector logo
├── src/                                # Source code
│   ├── daemon/                         # Background daemon service
│   │   ├── actions.rs                  # Execution handlers (Process, Window, Tuning, Audio, Scripts, Macros)
│   │   ├── discord_rpc.rs              # Discord Rich Presence client over named pipe
│   │   ├── engine.rs                   # Central automation state engine & rule evaluator
│   │   ├── file_watcher.rs             # Filesystem polling & directory event watcher
│   │   ├── hotkeys.rs                  # Low-level global hotkey listener & hold-mode engine
│   │   ├── ipc_server.rs               # Named pipe listener & connection worker
│   │   ├── lua_runner.rs               # Embedded Lua 5.4 execution runner with custom `ph` API
│   │   ├── overlay_manager.rs          # Background overlay process lifecycle & multi-monitor manager
│   │   ├── pe_mask.rs                  # Windows PE process spoofing & Task Manager masking (RuntimeBroker.exe)
│   │   ├── system_events.rs            # Win32 idle time & AC/battery power monitor
│   │   ├── telemetry.rs                # System telemetry provider (CPU, RAM, network, battery, mic)
│   │   ├── tray.rs                     # Win32 system tray manager, menu & message loop
│   │   ├── watcher.rs                  # WMI events & Toolhelp32 process snapshotting
│   │   └── webhook_server.rs           # Local HTTP webhook server (port 48123)
│   ├── gui/                            # egui desktop user interface
│   │   ├── components/                 # UI components
│   │   │   ├── icons.rs                # Win32 shell icon extractor, image loader & texture cache
│   │   │   ├── modal.rs                # Unified animated modal window framework
│   │   │   ├── overlay_renderer.rs     # GPU-accelerated overlay HUD element renderer
│   │   │   ├── script_highlighter.rs   # PowerShell, Batch & Lua syntax highlighter
│   │   │   └── widgets.rs              # Custom canvas controls (buttons, inputs, cards, drag handles, glyphs)
│   │   ├── views/                      # Application screens & panels
│   │   │   ├── action_catalog_modal.rs # Studio modal for categorized action browsing & search
│   │   │   ├── analytics_view.rs       # Real-time event telemetry stream & KPI metric cards
│   │   │   ├── changelog_modal.rs      # GitHub release notes & changelog viewer
│   │   │   ├── condition_catalog_modal.rs # Studio modal for categorized pre-condition browsing & search
│   │   │   ├── element_catalog_modal.rs # Studio modal for categorized overlay HUD elements & widgets
│   │   │   ├── guide_modal.rs          # GitBook-style interactive documentation & recipes
│   │   │   ├── header_bar.rs           # Custom window titlebar & navigation tabs
│   │   │   ├── inspector.rs            # Rule inspector, trigger & action editor (drag-and-drop, undo/redo)
│   │   │   ├── macro_modal.rs          # Macro timeline editor & playback studio
│   │   │   ├── overlay_modal.rs        # Overlay Studio (WYSIWYG artboard, HUD elements, snap-to-grid)
│   │   │   ├── process_picker_modal.rs # In-context process picker modal with filter & sorting
│   │   │   ├── rules_view.rs           # Rule management list & cards
│   │   │   ├── script_modal.rs         # Expandable script editor modal (PowerShell, CMD, Lua)
│   │   │   ├── settings_view.rs        # Balanced 2x2 settings (System, Engine, Appearance, Updates)
│   │   │   └── trigger_catalog_modal.rs # Studio modal for categorized trigger browsing & search
│   │   ├── macro_recorder.rs           # Win32 low-level keyboard/mouse macro recorder
│   │   ├── overlay_window.rs           # Standalone borderless GPU-accelerated overlay window
│   │   ├── state.rs                    # GUI state management & IPC synchronization
│   │   └── theme.rs                    # Design system tokens (Dark, Light, Chocolate palettes)
│   ├── i18n/                           # Internationalization & localization
│   │   ├── en.rs                       # English string catalog
│   │   └── ru.rs                       # Russian string catalog
│   ├── ipc/                            # Inter-process communication
│   │   ├── client.rs                   # Named pipe IPC client
│   │   └── protocol.rs                 # Request & response message types (Serde)
│   ├── autorun.rs                      # Windows Registry autorun integration
│   ├── config.rs                       # JSON configuration persistence & validation (theme, language, updates)
│   ├── daemon.rs                       # Daemon entrypoint & service lifecycle
│   ├── gui.rs                          # GUI entrypoint & eframe application loop
│   ├── i18n.rs                         # Internationalization loader & language switcher
│   ├── models.rs                       # Domain data models (Rule, Trigger, Action, Process)
│   ├── sharing.rs                      # Portable automation compression, encoding & clipboard sharing
│   ├── updater.rs                      # GitHub releases auto-update & self-replacement engine
│   └── main.rs                         # CLI dispatcher (--daemon, --gui, default)
├── build.rs                            # Windows resource compilation (icon, manifest)
├── Cargo.toml                          # Rust manifest & dependencies
└── LICENSE                             # MIT License
```

</details>

---

## 📝 Configuration

Configuration and state files are located in: **`%LOCALAPPDATA%\ProcessHandler\`**

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).

---

## 🙏 Credits

- **UI/UX Idea:** [ArtCraft Launcher](https://github.com/storytold/craft-launcher)

---

## ⚠️ Disclaimer

ProcessHandler interacts directly with Windows system processes and administrative APIs. The developers are not liable for any errors, including those caused by the user, as also stated in the license.

---

## 💬 Support & Contributing

For issues, questions, or feature requests, please open an issue on [**GitHub Issues**](https://github.com/psyattack/ProcessHandler/issues).

---