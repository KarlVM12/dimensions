# Dimensions 🌌

**Visual tmux session manager with collapsible tab groups** - Organize your terminal workflows with an interactive TUI.

![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)

## What is Dimensions?

Dimensions is a TUI (Terminal User Interface) for managing tmux sessions and windows. It provides a visual interface to organize your terminal workflows into collapsible groups called "dimensions". Key features:

- **✨ Visually collapse/expand tab groups** - Hide tabs you're not using
- **🔄 Switch between dimensions** - All processes stay alive in the background
- **💾 Persistent configuration** - Dimension names, tabs, and commands saved to disk
- **🎨 Beautiful interface** - Clean TUI built with ratatui
- **🚀 Lightning fast** - Written in Rust, powered by tmux
- **🖥️ Works on macOS & Linux** - Any terminal emulator (iTerm2, Alacritty, Wezterm, Kitty, etc.)

## Demo

```
┌────────────────────────────────────────────────────┐
│ 🌌 Dimensions - Visual Tmux Session Manager        │
├────────────────────────────────────────────────────┤
│ Dimensions          │ Tabs                         │
│ ▼ dev (4 tabs)      │ 1. Editor                    │
│   [active]          │ 2. Server (npm run dev)      │
│ ▶ personal (2 tabs) │ 3. Tests                     │
│ ▼ work (3 tabs)     │ 4. Logs                      │
├────────────────────────────────────────────────────┤
│ Status: Created dimension: dev                     │
├────────────────────────────────────────────────────┤
│ ↑/↓ Navigate  │ Space Collapse                     │
│ Enter Switch  │ n New │ t Add Tab │ q Quit         │
└────────────────────────────────────────────────────┘
```

## Why Dimensions?

**Problem:** You have different workflows (dev, personal, work) that require different sets of terminals. Switching contexts means:
- Closing/reopening tabs manually
- Losing running processes
- Forgetting what you had open

**Solution:** Dimensions lets you:
- Group tabs into named dimensions
- Collapse dimensions to hide them (processes keep running via tmux)
- Switch between dimensions instantly
- Never lose your running processes

## Installation

### Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs)
- **tmux**: `brew install tmux` (macOS) or `apt install tmux` (Linux)

### Build from Source

```bash
git clone https://github.com/KarlVM12/Dimensions.git
cd Dimensions
cargo build --release

# Binary will be at target/release/dimensions
./target/release/dimensions
```

### Install Globally

```bash
# Install via cargo (recommended)
cargo install --path .

# Make sure ~/.cargo/bin is in your PATH
# Add to ~/.zshrc (or ~/.bashrc):
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# Now you can run from anywhere:
dimensions
```

## Usage

### Recommended tmux Configuration

For the best experience, add this to your `~/.tmux.conf` to open Dimensions with a keybinding:

```bash
# Bind Ctrl+G to open Dimensions in a popup (works even inside nvim/vim)
bind -n C-g display-popup -E -w 80% -h 80% "dimensions"
```

After adding this, reload your tmux config:
```bash
tmux source-file ~/.tmux.conf
```

Now press `Ctrl+G` from anywhere (even inside nvim, Claude, or other programs) to:
- Open Dimensions in a popup overlay
- Navigate and select a dimension/tab
- Press Enter to switch
- Popup closes automatically

**Alternative keybindings:**
```bash
# Use Ctrl+D instead
bind -n C-d display-popup -E -w 80% -h 80% "dimensions"

# Use Prefix + Space (Ctrl+B then Space)
bind Space display-popup -E -w 80% -h 80% "dimensions"
```

### Launch Dimensions Manually

```bash
dimensions
```

### Keyboard Shortcuts

#### Normal Mode
- `↑/k` - Previous dimension
- `↓/j` - Next dimension
- `→/l` - Navigate right to select a tab
- `←/h` - Navigate left (back to dimension)
- `Space` - Collapse/expand current dimension
- `Enter` - **Smart switching:**
  - If on dimension (no tab selected): Create ad-hoc tab like "dev-1"
  - If on a specific tab: Switch to that tab
- `n` - Create new dimension
- `t` - Add new tab to current dimension
- `d` - Delete current dimension
- `x` - Remove selected tab
- `/` - Search/filter tabs by name
- `q` - Quit TUI and **detach from tmux** (returns to normal shell)

#### Input Mode (when creating dimension/tab)
- `Enter` - Submit
- `Esc` - Cancel
- `Backspace` - Delete character

#### Search Mode (when searching with `/`)
- Type to filter tabs by name (case-insensitive)
- `Enter` - Apply filter and stay in search mode
- `Esc` - Clear search and return to normal mode

### Workflows

#### Create a Development Dimension

1. Press `n` to create a new dimension
2. Type `dev` and press `Enter`
3. Press `t` to add a tab
4. Type `Claude` and press `Enter` (tab with no command)
5. Press `t` again
6. Type `Server:npm run dev` and press `Enter` (tab that runs a command)
7. Press `Enter` to switch to the dimension (TUI exits, you see the terminal)

#### Switch Between Dimensions

1. Use `↑/↓` to select a dimension
2. Press `Enter` to switch to it (TUI exits and attaches to tmux session)
3. Your previous dimension's tabs stay running in the background
4. Run `./dim` again to switch to another dimension

#### Switch to a Specific Tab

1. Use `↑/↓` to select a dimension
2. Press `→` to enter tab selection mode
3. Use `→/←` to select a specific tab
4. Press `Enter` to jump to that tab (TUI exits and you see the terminal)

#### Create Ad-Hoc Tabs

Sometimes you need a quick terminal without configuring it:

1. Use `↑/↓` to select a dimension
2. **Don't** press `→` to select a tab - stay on the dimension
3. Press `Enter` - Creates a new ad-hoc tab named "dimension-N"
4. These tabs don't appear in your configured tabs list
5. Perfect for quick commands or temporary work

#### Collapse a Dimension

1. Select a dimension with `↑/↓`
2. Press `Space` to collapse/expand it
3. Collapsed dimensions hide their tabs in the UI

#### Return to Dimensions TUI

When you're in a tmux session and want to switch dimensions:
1. Run `./dim` (or `dimensions` if installed globally)
2. The TUI will show you all your dimensions
3. Navigate and press `Enter` to switch
4. The TUI exits and switches you to the selected dimension/tab

**Tip**: You can run `./dim` from any tab in any dimension!

**Note**: The tmux window names (shown in the tmux status bar) will reflect your tab names, not just "dim" or shell names.

## How It Works

### Architecture

```
┌──────────────┐
│  Dimensions  │  (Rust TUI)
│     TUI      │
└──────┬───────┘
       │
       │ Creates/manages
       │
┌──────▼───────┐
│    tmux      │  (Session manager)
│   Sessions   │
└──────┬───────┘
       │
       ├─ Session: dev
       │  ├─ Window: Claude
       │  ├─ Window: Editor
       │  └─ Window: Server
       │
       └─ Session: personal
          ├─ Window: Email
          └─ Window: Music
```

- Each **dimension** = one tmux session
- Each **tab** = one tmux window within that session
- **Collapsing** = hiding tabs in the UI (tmux session stays alive)
- **Switching** = attaching to a different tmux session

### Data Storage

Configuration is stored in an OS-specific location:
- **macOS**: `~/Library/Application Support/dimensions/config.json`
- **Linux**: `~/.config/dimensions/config.json`

Example configuration:

```json
{
  "dimensions": [
    {
      "name": "dev",
      "collapsed": false,
      "tabs": [
        {"name": "Claude", "command": null},
        {"name": "Server", "command": "npm run dev"}
      ]
    }
  ],
  "active_dimension": "dev"
}
```

## Features

- [x] Create/delete dimensions
- [x] Add/remove tabs
- [x] Visual collapse/expand
- [x] Switch between dimensions
- [x] Persist configuration
- [x] tmux integration
- [x] Keyboard navigation
- [ ] Fuzzy search dimensions
- [ ] Edit dimension/tab names
- [ ] Import/export configurations
- [ ] Custom keybindings
- [ ] Themes

## Troubleshooting

### tmux not found
```bash
brew install tmux  # macOS
apt install tmux   # Ubuntu/Debian
```

### Can't switch dimensions
- Make sure you're running Dimensions inside a terminal (not already in tmux)
- Or run it from within tmux, it will use `switch-client`

### Processes not persisting
- Dimensions uses tmux sessions - they persist until you explicitly delete the dimension or kill the tmux session

## Contributing

PRs welcome! Some ideas:

- Add fuzzy search with `skim`/`fzf` integration
- Support for saving/restoring working directories
- Mouse support
- Color themes
- Shell completions

## License

MIT

## Credits

- Built with [ratatui](https://github.com/ratatui-org/ratatui) for the TUI
- Powered by [tmux](https://github.com/tmux/tmux) for session management
