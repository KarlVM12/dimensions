# Changelog

All notable changes to Dimensions will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.1]: https://github.com/KarlVM12/Dimensions/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/KarlVM12/Dimensions/releases/tag/v0.1.0
