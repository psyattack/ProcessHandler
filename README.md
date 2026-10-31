<p align="center">
  <img src="assets/app.svg" alt="ProcessHandler Logo" width="120" height="120">
</p>

<h1 align="center">ProcessHandler</h1>

<p align="center">Lightweight, fast, native Windows process automation service and task monitor built in 100% pure Rust.</p>

<p align="center">
  <a href="https://github.com/psyattack/ProcessHandler/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/psyattack/ProcessHandler"></a>
  <a href="https://github.com/psyattack/ProcessHandler/releases"><img alt="Downloads" src="https://img.shields.io/github/downloads/psyattack/ProcessHandler/total"></a>
  <a href="LICENSE"><img alt="Licence: MIT" src="https://img.shields.io/badge/license-MIT-blue"></a>
</p>

<p align="center">
  <a href="https://github.com/psyattack/ProcessHandler/releases/latest">Download</a>
  ·
  <a href="https://github.com/psyattack/ProcessHandler/issues/new">Issues</a>
</p>

> [!WARNING]
> ProcessHandler interacts directly with Windows system processes and administrative APIs. The developers are not liable for any errors, including those caused by the user, as also stated in the license.

<p align="center">
  <img src="https://github.com/user-attachments/assets/777a1893-7a49-45bf-ad52-84375a5ef808">
</p>

## Features

**ProcessHandler** is a lightweight, high-performance Windows automation suite and process supervisor built in 100% pure Rust. It decouples background execution from user interaction, giving you full control over your apps, workflows, and system performance with near-zero resource utilization.

* **Dual-Tier Architecture**: A silent, low-overhead system tray daemon executes rules and monitors events 24/7 in the background, while the hardware-accelerated desktop GUI opens only when you need to configure or monitor tasks.
* **Instant & Reactive**: Built on native Windows event hooks to react instantaneously to process launches, window focus, hardware changes, and hotkeys without battery-draining polling loops.
* **Accessible Yet Deep**: Create automated pipelines visually with drag-and-drop actions and full undo/redo, or take full programmatic control with embedded Lua 5.4, PowerShell, and Batch scripting.
* **Companion Process Mode**: Bind helper tools, custom overlays, or performance recorders to primary games and applications—they launch automatically on start and cleanly terminate upon exit.
* **Overlay Studio (HUD)**: Build GPU-accelerated on-screen HUDs, custom crosshairs, telemetry badges, and interactive widgets with anti-capture streaming protection.
* **Macro Studio**: Record and replay precise keyboard and mouse sequences with 3D camera rotation support, humanized motion jitter, and game-compatible DirectInput scancodes.
* **Resource Optimization**: Protect your system from rogue memory leaks by automatically trimming process working sets and dynamically adjusting CPU priorities and affinity.
* **Audio & Device Automation**: Control master and per-app volumes, balance channels, switch audio endpoints, or trigger automations from hardware and power events.
* **Smart Rules & Branching**: Combine multi-condition pre-checks (running apps, active window, schedules, webhooks, system idle) with dynamic If/Else execution flows.
* **Portable Sharing**: Export and import complete automations instantly via compact clipboard codes for effortless backup and sharing.

## Installation

#### Prerequisites
- **Windows 10 / 11** (x64)

To install, download the latest release from [**GitHub Releases**](https://github.com/psyattack/ProcessHandler/releases) and extract the downloaded archive to a convenient location on your drive.

> [!IMPORTANT]
> **Administrator Privileges**: Run ProcessHandler as administrator to control or send hotkeys and macros to elevated games and applications.

> [!NOTE]
> Automatically registers for Windows startup on first launch (configurable in Settings).

## Development

<details>
<summary><b>Setup</b></summary>

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
<summary><b>Stack</b></summary>

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
<summary><b>Structure</b></summary>

```
ProcessHandler/
├── assets/                                # Application assets & branding
│   ├── fonts/                             # Bundled variable fonts (Archivo, GeistMono, Inter)
│   ├── app.ico                            # Win32 application icon
│   ├── app.png                            # High-res RGBA icon
│   └── app.svg                            # Vector logo
├── src/                                   # Source code
│   ├── daemon/                            # Background daemon service
│   │   ├── actions.rs                     # Execution handlers (Process, Window, Tuning, Audio, Scripts, Macros)
│   │   ├── discord_rpc.rs                 # Discord Rich Presence client over named pipe
│   │   ├── engine.rs                      # Central automation state engine & rule evaluator
│   │   ├── file_watcher.rs                # Filesystem polling & directory event watcher
│   │   ├── hotkeys.rs                     # Low-level global hotkey listener & hold-mode engine
│   │   ├── ipc_server.rs                  # Named pipe listener & connection worker
│   │   ├── lua_runner.rs                  # Embedded Lua 5.4 execution runner with custom `ph` API
│   │   ├── overlay_manager.rs             # Background overlay process lifecycle & multi-monitor manager
│   │   ├── pe_mask.rs                     # Windows PE process spoofing & Task Manager masking (RuntimeBroker.exe)
│   │   ├── system_events.rs               # Win32 idle time & AC/battery power monitor
│   │   ├── telemetry.rs                   # System telemetry provider (CPU, RAM, network, battery, mic)
│   │   ├── tray.rs                        # Win32 system tray manager, menu & message loop
│   │   ├── watcher.rs                     # WMI events & Toolhelp32 process snapshotting
│   │   └── webhook_server.rs              # Local HTTP webhook server (port 48123)
│   ├── gui/                               # egui desktop user interface
│   │   ├── components/                    # UI components
│   │   │   ├── icons.rs                   # Win32 shell icon extractor, image loader & texture cache
│   │   │   ├── modal.rs                   # Unified animated modal window framework
│   │   │   ├── overlay_renderer.rs        # GPU-accelerated overlay HUD element renderer
│   │   │   ├── script_highlighter.rs      # PowerShell, Batch & Lua syntax highlighter
│   │   │   └── widgets.rs                 # Custom canvas controls (buttons, inputs, cards, drag handles, glyphs)
│   │   ├── views/                         # Application screens & panels
│   │   │   ├── action_catalog_modal.rs    # Studio modal for categorized action browsing & search
│   │   │   ├── analytics_view.rs          # Real-time event telemetry stream & KPI metric cards
│   │   │   ├── changelog_modal.rs         # GitHub release notes & changelog viewer
│   │   │   ├── condition_catalog_modal.rs # Studio modal for categorized pre-condition browsing & search
│   │   │   ├── element_catalog_modal.rs   # Studio modal for categorized overlay HUD elements & widgets
│   │   │   ├── guide_modal.rs             # GitBook-style interactive documentation & recipes
│   │   │   ├── header_bar.rs              # Custom window titlebar & navigation tabs
│   │   │   ├── inspector.rs               # Rule inspector, trigger & action editor (drag-and-drop, undo/redo)
│   │   │   ├── macro_modal.rs             # Macro timeline editor & playback studio
│   │   │   ├── overlay_modal.rs           # Overlay Studio (WYSIWYG artboard, HUD elements, snap-to-grid)
│   │   │   ├── process_picker_modal.rs    # In-context process picker modal with filter & sorting
│   │   │   ├── rules_view.rs              # Rule management list & cards
│   │   │   ├── script_modal.rs            # Expandable script editor modal (PowerShell, CMD, Lua)
│   │   │   ├── settings_view.rs           # Balanced 2x2 settings (System, Engine, Appearance, Updates)
│   │   │   └── trigger_catalog_modal.rs   # Studio modal for categorized trigger browsing & search
│   │   ├── macro_recorder.rs              # Win32 low-level keyboard/mouse macro recorder
│   │   ├── overlay_window.rs              # Standalone borderless GPU-accelerated overlay window
│   │   ├── state.rs                       # GUI state management & IPC synchronization
│   │   └── theme.rs                       # Design system tokens (Dark, Light, Chocolate palettes)
│   ├── i18n/                              # Internationalization & localization
│   │   ├── en.rs                          # English string catalog
│   │   └── ru.rs                          # Russian string catalog
│   ├── ipc/                               # Inter-process communication
│   │   ├── client.rs                      # Named pipe IPC client
│   │   └── protocol.rs                    # Request & response message types (Serde)
│   ├── autorun.rs                         # Windows Registry autorun integration
│   ├── config.rs                          # JSON configuration persistence & validation (theme, language, updates)
│   ├── daemon.rs                          # Daemon entrypoint & service lifecycle
│   ├── gui.rs                             # GUI entrypoint & eframe application loop
│   ├── i18n.rs                            # Internationalization loader & language switcher
│   ├── models.rs                          # Domain data models (Rule, Trigger, Action, Process)
│   ├── sharing.rs                         # Portable automation compression, encoding & clipboard sharing
│   ├── updater.rs                         # GitHub releases auto-update & self-replacement engine
│   └── main.rs                            # CLI dispatcher (--daemon, --gui, default)
├── build.rs                               # Windows resource compilation (icon, manifest)
├── Cargo.toml                             # Rust manifest & dependencies
└── LICENSE                                # License
```

</details>

## Configuration

All files are located in: **`%LOCALAPPDATA%\ProcessHandler\`**

## License

This project is licensed under the [MIT License](LICENSE). Third-party components: [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

## Credits

- **UI/UX Idea:** [ArtCraft Launcher](https://github.com/storytold/craft-launcher)

## Support & Contributing

For issues, questions, or feature requests, please open an issue on [**GitHub Issues**](https://github.com/psyattack/ProcessHandler/issues).
