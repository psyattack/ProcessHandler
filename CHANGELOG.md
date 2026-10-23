# Changelog

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
