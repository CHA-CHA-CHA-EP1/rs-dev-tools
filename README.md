# Terminal Todo App

A terminal-based todo application built with Rust and Ratatui.

## Features

- **VIM-style Navigation**: Navigate through the interface using familiar VIM keybindings
- **Text Search/Filter**: Filter todos in real-time as you type
- **Modal Interface**: Switch between Normal and Editing modes
- **TODO List Mock**: Display and navigate through a mock list of todos

## Controls

### Normal Mode (Default)
- `i` - Enter editing mode
- `j` - Move down in todo list
- `k` - Move up in todo list  
- `q` - Quit application

### Editing Mode
- `Esc` - Exit editing mode back to normal
- Type to search/filter todos
- `←/→` - Move cursor left/right
- `Backspace` - Delete character

## Usage

```bash
cargo run
```

The application starts in Normal mode. Press `i` to start typing and filter todos, then `Esc` to return to Normal mode for navigation.