# Changelog

## [2.4.0](https://github.com/psyattack/ProcessHandler/compare/v2.3.0...v2.4.0) - 2026-10-25

### Added
- Discord Rich Presence: Added actions to broadcast custom game and application status directly to Discord via local IPC, including activity details, custom image assets, elapsed session timers, interactive buttons, and automatic status clearing when a target process exits.
- Archive Compression and Extraction: Added an archive management action supporting packing and unpacking files and directories across ZIP, 7Z, TAR, TAR.GZ, and RAR formats with configurable compression levels and overwrite protection.
- Filesystem Operations: Added automated file and directory management actions, including creating files or folders, copying, moving, renaming, deleting, and writing or appending text with dynamic token interpolation.
- Process Memory Trimming: Added a memory optimization action that forcefully flushes process working set RAM to free up system memory for targeted applications.
- Monthly Schedule Trigger: Added monthly recurring triggers allowing rules to execute automatically on a designated day of the month at a specific time.

### Changed
- Expanded the Action Catalog and Rule Inspector with dedicated visual editors for Discord status, file archiving, filesystem management, and memory trimming.
- Updated the in-app user guide, architecture documentation, and README with guides for the new automation actions.
- Expanded English and Russian localization catalogs across all new actions, triggers, and configuration options.

<details><summary>Detailed Changelog</summary>

### Added
- **actions:** add discord rich presence, archive compression, filesystem ops, and memory trimming (by @psyattack in efe4028)

</details>


## [2.3.0](https://github.com/psyattack/ProcessHandler/compare/v2.2.0...v2.3.0) - 2026-10-24

### Added
- Extended Rule Pre-conditions: Added system event checks (user idle duration, session lock/unlock, AC/battery power) and resource thresholds (monitoring sustained process CPU % and RAM MB) to rule prerequisites and conditional branches.
- File Watcher Extension Filtering: File pre-conditions now support directory monitoring with specific file extension filters (such as *.log or *.txt).
- Audio Format Switching: Added an audio format action allowing dynamic switching of sample rates (Hz), bit depths, and channel configurations on audio playback and recording devices.
- Independent Channel Balance: Added separate Left and Right channel volume controls for fine-grained audio balance adjustment.
- Unified Application Audio: Per-process volume adjustments and mute toggling are now integrated directly into the unified Audio Control action.
- Customizable Beep Tone Action: Added tone generator actions with configurable frequency (Hz) and duration (ms) alongside standard audio file playback.
- Macro Studio Anti-Bot Humanization: Added a one-click Humanize mode in Macro Studio that injects natural Gaussian delays, key hold variance, and mouse tremor across recorded sequences.
- Global Hotkey Continuous Loops: Hotkey triggers now support toggling continuous execution loops in addition to hold-and-repeat mode.
- Contextual Empty State Hints: Added helpful guidance messages when rule search or filter queries yield no results.

### Changed
- Unified action models across audio controls, default device switching, and script execution to streamline rule creation.
- Updated in-app user guide and documentation with guidance on Windows UIPI privilege elevation for global game hotkeys.
- Expanded English and Russian localization catalogs across all new pre-conditions, audio formats, and macro settings.

<details><summary>Detailed Changelog</summary>

### Added
- **rules:** expand pre-conditions, advance audio controls, and add macro humanization (by @psyattack in d68978b)

</details>


## [2.2.0](https://github.com/psyattack/ProcessHandler/compare/v2.1.0...v2.2.0) - 2026-10-23

### Added
- Rule Pre-conditions: Rules can now be gated by prerequisites before running. Supported checks include running process status, active foreground window focus, time and day-of-week schedule windows, power source (AC power vs battery), and file or directory existence.
- Logical AND / OR Condition Chaining: Pre-conditions and branch checks support customizable boolean AND / OR operators for combining multiple conditions.
- Conditional Branching (If / Else If / Else): Added a dedicated branching action allowing pipelines to evaluate conditions dynamically at runtime and execute corresponding action branches, including repeatable Else If branches and a fallback Else sequence.
- Nested Drag-and-Drop Pipeline Reordering: Full drag-and-drop reordering with animated drop laser guides, auto-scrolling, and floating card previews is now supported directly within nested If / Else action branches.
- Macro Studio Organic Mouse Smoothing: Added an organic motion mode for smooth mouse movements, simulating natural human hand movement and speed fluctuations.
- Expandable Mouse Move Groups in Macro Timeline: Grouped mouse movement blocks in the Macro Studio timeline can now be expanded into individual steps for detailed coordinate inspection and fine-tuning.
- Extended Process Picker Integration: The running process selector modal now connects directly with rule pre-conditions, nested branch conditions, and sub-actions inside If / Else blocks.

### Changed
- Action catalog now restricts recursive logic nesting to keep pipelines structured and clean.
- Updated the interactive user guide, architecture documentation, and README with conditional automation walkthroughs.
- Expanded English and Russian localization catalogs across all new conditions, branching tools, and Macro Studio controls.

<details><summary>Detailed Changelog</summary>

### Added
- **rules:** add pre-conditions and dynamic if-else branching with dnd pipeline (by @psyattack in 3b9ba64)

</details>


## [2.1.0](https://github.com/psyattack/ProcessHandler/compare/v2.0.0...v2.1.0) - 2026-10-23

### Added
- In-game 3D camera turn and relative mouse movement support in macro recorder and playback.
- Hardware keyboard scan codes support for reliable input in games and DirectInput applications.
- Hotkey hold mode with continuous execution loops while the key combination is held down.
- Integrated automatic update checker and in-app updater via GitHub Releases.
- Safety key release on macro cancel or stop to prevent stuck keys during playback.

### Changed
- Improved macro playback timing and hold durations to ensure smooth detection in high-refresh and frame-capped games.
- Enhanced settings, analytics, and process inspector user interface layouts.
- Updated in-app documentation and rule guides with the latest automation capabilities.

<details><summary>Detailed Changelog</summary>

### Added
- **macro:** support in-game camera turns, hotkey hold loops, and centralize i18n (by @psyattack in 4497659)

</details>
