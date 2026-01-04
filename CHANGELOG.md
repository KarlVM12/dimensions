# Changelog

All notable changes to Dimensions will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.17] - 2026-01-03

### Added
- Jumping to a tab based on the dimension you are hovering on in TUI via ":#"
- When a session is running, gives small preview of tab

## [0.2.16] - 2026-01-03

### Changed
- Initial window is now automatically saved as a tab when creating a dimension
- Work done in the initial window is preserved across restarts (previously lost)

## [0.2.15] - 2026-01-03

### Added
- Base directory support for dimensions - set a default working directory when creating dimensions
- Two-step dimension creation flow: prompt for name, then base directory
- Shell-style directory tab completion with Tab/Shift+Tab navigation
- Support for ~, relative paths (../), and environment variables ($VAR) in directory input
- Visual path display: dimensions and tabs header show working directory in faded text
- Paths display with ~ instead of full /Users/username/ for cleaner UI

### Changed
- Tabs now inherit their dimension's base directory automatically
- Removed all dead code and unused functions for cleaner codebase

## [0.2.14] - 2026-01-02

### Added
- Working directory persistence - tabs now remember and restore the directory they were created in
- Tabs created from different directories will open in their original location after restart

## [0.2.13] - 2025-12-25

### Fixed
- Linux binaries now use musl instead of glibc for universal compatibility across all distributions (fixes "GLIBC_2.39 not found" errors on older systems)

## [0.2.12] - 2025-12-25

### Added
- Tab deletion now prompts for confirmation (y/n), matching dimension deletion behavior
- Shell aliases now work in tab commands (uses user's `$SHELL -i` instead of `sh`)

### Changed
- Simplified README - removed build from source, architecture diagrams, features checklist, troubleshooting, and license sections
- GitHub release notes now only show changes (removed installation instructions)

## [0.2.11] - 2025-12-23

### Fixed
- Command execution now properly handles arguments and spaces using shell wrapper (e.g., `npm run dev`, `ls -la` work correctly)
- Windows with one-shot commands (like `ls`) now stay open after execution instead of disappearing

## [0.2.10] - 2025-12-23

### Fixed
- GitHub Actions release workflow YAML (release generation was failing)

## [0.2.9] - 2025-12-23

### Fixed
- `dimensions --update` now updates the currently-running binary location to avoid PATH-precedence surprises

## [0.2.8] - 2025-12-23

### Fixed
- Installer checksum verification now works reliably across release assets

## [0.2.7] - 2025-12-23

### Added
- Background update check (cached daily) that shows “New version available” in the status bar; disable with `DIMENSIONS_NO_UPDATE_CHECK=1`
- `--version` and interactive `--update` flags (`--update` installs the latest release after confirmation)

## [0.2.6] - 2025-12-23

### Added
- GitHub Actions release builds for macOS/Linux and an `install.sh` installer script

## [0.2.5] - 2025-12-23

### Changed
- Removed the collapse feature and related keybinding/UI
- Updated README tagline to “Terminal Tab Manager”
- Improved truncation for small terminals by shortening long list rows with `…`
- Improved the “launched inside another TUI” error to recommend using your tmux popup keybinding (example provided)

### Internal
- Removed unused functions that triggered dead-code warnings
- Added `unicode-width` to truncate by display width

## [0.2.4] - 2025-12-23

### Changed
- Improved the “launched inside another TUI” error to recommend using your tmux popup keybinding (example provided)

## [0.2.3] - 2025-12-23

### Fixed
- Robust tab selection/deletion/switching when tmux window indices are sparse or renumbered (selection tracks tmux window index, not list position)
- Deleting the currently-active tab keeps Dimensions selection/current markers in sync with tmux

### Changed
- Dimensions no longer persists an automatic "`dimension-1`" starter tab into config; empty dimensions remain empty unless you add a configured tab via the UI

## [0.2.2] - 2025-12-23

### Internal
- Config tabs are now explicitly treated as **configured tabs** (a template); tmux windows are the live state

## [0.2.1] - 2025-12-23

### Fixed
- **Creating tabs no longer switches your tmux client in the background**
- Fixed tmux window creation target to avoid `create_window failed: index 0 in use` in some setups

### Changed
- Selection now uses a highlight background (current session/tab stays green with `*`)
- Opening Dimensions inside tmux starts the cursor on your current dimension
- Search results only highlight the current tab in green (with `*`), not every tab in the current dimension
- Updated README to match current behavior


## [0.2.0] - 2025-12-23

### Added
- **Fuzzy search** across all dimensions and tabs with live updates
  - Press `/` to search, results update instantly as you type
  - Non-consecutive character matching (e.g., "edt" matches "Editor")
  - Searches both dimension names and tab names simultaneously
  - Flat results view showing "dimension: tab_name" format
  - Results sorted by fuzzy match score (best matches first)
  - Navigate results with `↑/↓`, select with `Enter`
- **Visual indicators** for current session and tab
  - Green color + "*" marker shows your current dimension/tab
  - Helps orient yourself when using Ctrl+G popup
- Added `fuzzy-matcher` crate dependency for search functionality

### Changed
- **Close popup keybinding** changed from `c` to `Esc`
  - `Esc` in normal mode: closes popup without switching
  - `Esc` in search mode: cancels search and returns to normal mode
  - More intuitive "escape" behavior
- **Delete key `d` is now context-sensitive**
  - If tab is selected: deletes the tab
  - If on dimension (no tab selected): deletes the dimension
  - Removed `x` key (now just use `d` for all deletions)
- **Dimension creation** no longer auto-creates tmux session
  - Creates dimension in config only
  - tmux session created when you first switch to it
  - Prevents unwanted default `0:zsh` window
- Search now works globally across all dimensions (not just current dimension)
- Search results show tabs from all dimensions in a single flat list
- Improved search UX with live filtering instead of "apply filter" step

### Technical
- Added `MatchType` and `SearchResult` data structures
- Implemented search result caching to avoid recalculation on every render
- Updated UI to switch between two-column and single-column layouts based on search state
- Enhanced input handling to support live search updates

## [0.1.1] - 2025-12-22

### Fixed
- Better error handling when launching from within nvim/vim
- Added helpful error messages when terminal initialization fails
- Improved error reporting for users attempting to run `:!dimensions` from nvim

### Changed
- Terminal setup now provides clear guidance when run from incompatible environments
- Error messages now suggest practical workarounds (exit nvim, use tmux keybinding, etc.)

## [0.1.0] - 2025-12-22

### Added
- Initial release of Dimensions TUI
- Visual dimension (tab group) management with collapse/expand
- Smart tab switching and ad-hoc tab creation
- Process persistence via tmux sessions
- Keyboard-driven navigation (vim-style + arrow keys)
- Persistent configuration saved to `~/.config/dimensions/config.json`
- Support for creating/deleting dimensions
- Support for adding/removing tabs within dimensions
- Ad-hoc tab creation (e.g., "dev-1", "dev-2") when pressing Enter on dimension
- Quit with detach (`q`) or switch to dimension/tab (`Enter`)
- Tab names properly shown in tmux status bar

### Features
- `↑/↓` or `j/k` - Navigate dimensions
- `→/←` or `l/h` - Navigate tabs
- `Enter` - Smart switching (dimension creates ad-hoc tab, tab switches to it)
- `Space` - Collapse/expand dimensions
- `n` - Create new dimension
- `t` - Add new tab to dimension
- `d` - Delete dimension
- `x` - Remove tab
- `q` - Quit and detach from tmux

[0.2.17]: https://github.com/KarlVM12/Dimensions/compare/v0.2.16...v0.2.17
[0.2.16]: https://github.com/KarlVM12/Dimensions/compare/v0.2.15...v0.2.16
[0.2.15]: https://github.com/KarlVM12/Dimensions/compare/v0.2.14...v0.2.15
[0.2.14]: https://github.com/KarlVM12/Dimensions/compare/v0.2.13...v0.2.14
[0.2.13]: https://github.com/KarlVM12/Dimensions/compare/v0.2.12...v0.2.13
[0.2.12]: https://github.com/KarlVM12/Dimensions/compare/v0.2.11...v0.2.12
[0.2.11]: https://github.com/KarlVM12/Dimensions/compare/v0.2.10...v0.2.11
[0.2.10]: https://github.com/KarlVM12/Dimensions/compare/v0.2.9...v0.2.10
[0.2.9]: https://github.com/KarlVM12/Dimensions/compare/v0.2.8...v0.2.9
[0.2.8]: https://github.com/KarlVM12/Dimensions/compare/v0.2.7...v0.2.8
[0.2.7]: https://github.com/KarlVM12/Dimensions/compare/v0.2.6...v0.2.7
[0.2.6]: https://github.com/KarlVM12/Dimensions/compare/v0.2.5...v0.2.6
[0.2.5]: https://github.com/KarlVM12/Dimensions/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/KarlVM12/Dimensions/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/KarlVM12/Dimensions/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/KarlVM12/Dimensions/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/KarlVM12/Dimensions/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/KarlVM12/Dimensions/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/KarlVM12/Dimensions/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/KarlVM12/Dimensions/releases/tag/v0.1.0
