# Changelog

All notable changes to Dimensions will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.2.4]: https://github.com/KarlVM12/Dimensions/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/KarlVM12/Dimensions/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/KarlVM12/Dimensions/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/KarlVM12/Dimensions/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/KarlVM12/Dimensions/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/KarlVM12/Dimensions/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/KarlVM12/Dimensions/releases/tag/v0.1.0
