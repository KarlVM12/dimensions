# Dimensions 🌌

**Terminal Tab Manager** - Organize your terminal workflows with an interactive TUI.

![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)

## What is Dimensions?

Dimensions is a TUI (Terminal User Interface) for managing tmux sessions and windows. It provides a visual interface to organize your terminal workflows into collapsible groups called "dimensions". Key features:

- **🔍 Fuzzy search** - Live search across all dimensions and tabs with instant results
- **🔄 Switch between dimensions** - All processes stay alive in the background
- **💾 Persistent configuration** - Dimension names, tabs, and commands saved to disk
- **⚡ Popup mode** - Quick access with Ctrl+G from anywhere (even inside vim/nvim)
- **🎨 Beautiful interface** - Clean TUI built with ratatui
- **🚀 Lightning fast** - Written in Rust, powered by tmux
- **🖥️ Works on macOS & Linux** - Any terminal emulator (iTerm2, Alacritty, Wezterm, Kitty, etc.)

## Demo

```
┌────────────────────────────────────────────────────┐
│ 🌌 Dimensions - Terminal Tab Manager               │
├────────────────────────────────────────────────────┤
│ Dimensions          │ Tabs                         │
│ ▼ dev (4 tabs) *    │ 1. Editor                    │
│ ▶ personal (2 tabs) │ 2. Server (npm run dev)      │
│ ▼ work (3 tabs)     │ 3. Tests                     │
│                     │ 4. Logs *                    │
├────────────────────────────────────────────────────┤
│ Status: Created dimension: dev                     │
├────────────────────────────────────────────────────┤
│ ↑/↓ Navigate  │ / Search │ Esc Close │ q Quit      │
└────────────────────────────────────────────────────┘
```

## Why Dimensions?

**Problem:** You have different workflows (dev, personal, work) that require different sets of terminals. Switching contexts means:
- Closing/reopening tabs manually
- Losing running processes
- Forgetting what you had open

**Solution:** Dimensions lets you:
- Group tabs into named dimensions
- Switch between dimensions instantly
- Never lose your running processes

## Installation

### Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs)
- **tmux**: `brew install tmux` (macOS) or `apt install tmux` (Linux)

### Option 1: Install from GitHub Releases

Use the installer script (recommended to pin a version):

```bash
curl -fsSL https://raw.githubusercontent.com/KarlVM12/Dimensions/v0.2.9/install.sh | sh -s -- --version v0.2.9
```
Then add `~/.local/bin` to your `PATH` if needed. <br>

Or from release version, it includes prebuilt binaries for macOS/Linux and checksum files (`.sha257` per asset and a combined `SHA256SUMS`)

1. Download the right binary for your OS/arch from the latest GitHub Release
2. Install it somewhere on your `PATH`:

```bash
chmod +x dimensions
mkdir -p ~/.local/bin
mv dimensions ~/.local/bin/dimensions
```

### Option 2: Build from Source

```bash
git clone https://github.com/KarlVM12/Dimensions.git
cd Dimensions
cargo build --release

# Binary will be at target/release/dimensions
./target/release/dimensions
```

#### Install Globally after Building from Source

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

For the best experience, add this to your `~/.tmux.conf`:

```bash
# Bind Ctrl+G to open Dimensions in a popup (works even inside nvim/vim)
bind -n C-g display-popup -E -w 80% -h 80% "dimensions"

# Window configuration for Dimensions
set -g base-index 0           # Optional: start window numbering at 0
set -g renumber-windows on    # Optional: renumber windows when one is closed
set -g mouse on               # Enable mouse support

# Optional: Minimal status bar (avoids dimension name truncation)
set -g status-left "🌌 "
```

After adding this, reload your tmux config:
```bash
tmux source-file ~/.tmux.conf
```

Now press `Ctrl+G` from anywhere (even inside nvim, Claude, or other programs) to:
- Open Dimensions in a popup overlay
- Navigate and select a dimension/tab
- Press Enter to switch (popup closes and switches to selected tab)
- Press Esc to close popup without switching
- If you launched from inside tmux, the cursor starts on your current dimension

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
- `Enter` - Switch to the selected dimension/tab
  - If the dimension's tmux session doesn't exist yet, Dimensions creates it and bootstraps any configured tabs
  - If the dimension has no configured tabs, Dimensions creates a starter tmux window named "`dimension-1`" (not saved to config)
- `n` - Create new dimension
- `t` - Add new tab to current dimension
- `d` - **Context-sensitive delete:**
  - If tab is selected (after pressing `→`): Delete that tab
  - If on dimension (no tab selected): Delete entire dimension
- `/` - **Fuzzy search** across all dimensions and tabs (live updates)
- `Esc` - Close popup without switching (when in normal mode)
- `q` - Quit TUI and **detach from tmux** (returns to normal shell)

#### Input Mode (when creating dimension/tab)
- `Enter` - Submit
- `Esc` - Cancel
- `Backspace` - Delete character

#### Search Mode (when searching with `/`)
- **Fuzzy matching** - Search updates live as you type (e.g., "edt" matches "Editor")
- Searches both **dimension names** and **tab names** across all dimensions
- Results shown as flat list: "dimension: tab_name"
- Sorted by fuzzy match score (best matches first)
- `↑/↓` - Navigate through search results
- `Enter` - Select result and switch to that dimension/tab immediately
- `Esc` - Cancel search and return to normal mode

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
4. Run `dimensions` again to switch to another dimension

#### Switch to a Specific Tab

1. Use `↑/↓` to select a dimension
2. Press `→` to enter tab selection mode
3. Use `→/←` to select a specific tab
4. Press `Enter` to jump to that tab (TUI exits and you see the terminal)

#### Create a Quick Temporary Tab

If you want a tab that isn't saved to config, create it directly in tmux (it will still show up in Dimensions while the session exists):

```bash
tmux new-window -n scratch
```

#### Use Fuzzy Search to Find Any Tab

The fastest way to switch to any tab across all dimensions:

1. Press `/` to start searching
2. Start typing (search updates live) - e.g., type "ed" to find "Editor"
3. Use `↑/↓` to navigate matching results
4. Press `Enter` to switch immediately to the selected tab
5. Or press `Esc` to cancel

**Examples:**
- Type "dev" → finds all tabs in "dev" dimension + any tab with "dev" in the name
- Type "edt" → fuzzy matches "Editor" tab
- Type "srv" → matches "Server" tab

#### Return to Dimensions TUI

When you're in a tmux session and want to switch dimensions:
1. Run `dimensions` (or `./target/release/dimensions` if not installed globally)
2. The TUI will show you all your dimensions
3. Navigate and press `Enter` to switch
4. The TUI exits and switches you to the selected dimension/tab

**Tip**: You can run `dimensions` from any tab in any dimension!

**Note**: The tmux window names (shown in the tmux status bar) will reflect your tab names (e.g., "Editor", "Server"), making it easy to identify which tab you're in.

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
          ├─ Window: zsh
          └─ Window: Codex
```

- Each **dimension** = one tmux session
- Each **tab** = one tmux window within that session
- **Collapsing** = hiding tabs in the UI (tmux session stays alive)
- **Switching** = attaching to a different tmux session

### Data Storage

Configuration is stored in an OS-specific location:
- **macOS**: `~/Library/Application\ Support/dimensions/config.json`
- **Linux**: `~/.config/dimensions/config.json`

### Update Checks

By default, Dimensions may check GitHub Releases about once per day to show a “New version available” message in the status bar.
Disable by setting `DIMENSIONS_NO_UPDATE_CHECK=1`.

You can also run:
- `dimensions --version` to print the current version
- `dimensions --update` to check for updates and (optionally) install the latest release

#### How Config and tmux Work Together

Dimensions uses a **hybrid storage approach**:

1. **Config file** = Blueprint/template for dimensions
   - Stores dimension names and **configured tabs** (name + optional command)
   - Saved when you create/delete dimensions or add/remove tabs through the UI

2. **tmux sessions** = Live running state
   - When a tmux session exists, Dimensions shows the actual windows from tmux
   - Includes both configured tabs AND extra windows you created directly in tmux

3. **Bootstrap from config**
   - When switching to a dimension without a tmux session, Dimensions creates it
   - Creates windows based on the configured tabs in your config
   - Runs any configured commands automatically

**Example:** If your config has a "Server" tab with `npm run dev`, switching to that dimension will create the tmux window and run the command automatically.

Example configuration:

```json
{
  "dimensions": [
    {
      "name": "dev",
      "tabs": [
        {"name": "Claude", "command": "claude"},
        {"name": "Server", "command": "npm run dev"}
      ]
    },
    {
      "name": "personal",
      "tabs": [
        {"name": "zsh", "command": null},
        {"name": "Codex", "command": "codex"}
      ]
    }
  ]
}
```

**Note:** Windows you create directly in tmux are not saved to config; only **configured tabs** you add via Dimensions are persisted.

## Features

- [x] Create/delete dimensions
- [x] Add/remove tabs
- [x] Switch between dimensions
- [x] Persist configuration
- [x] tmux integration
- [x] Keyboard navigation
- [x] **Fuzzy search** across all dimensions and tabs (live updates)
- [x] Visual indicators for current session/tab
- [x] Popup mode with Ctrl+G keybinding
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

- Edit dimension/tab names in place
- Support for saving/restoring working directories
- Mouse support
- Color themes
- Shell completions
- Export/import dimension configurations

## License

MIT

## Credits

- Built with [ratatui](https://github.com/ratatui-org/ratatui) for the TUI
- Powered by [tmux](https://github.com/tmux/tmux) for session management
- Fuzzy search powered by [fuzzy-matcher](https://github.com/lotabout/fuzzy-matcher)
