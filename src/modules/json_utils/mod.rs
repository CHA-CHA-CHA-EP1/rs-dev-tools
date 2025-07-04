use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use serde_json::{self, Value};
use arboard::Clipboard;
use std::fs;
use std::process::Command;
use tempfile::NamedTempFile;
use notify::{Watcher, RecursiveMode, Result as NotifyResult};
use std::sync::mpsc;
use std::time::Duration;

#[derive(PartialEq)]
enum ViewMode {
    Raw,
    Tree,
}

#[derive(Clone)]
struct JsonTreeNode {
    key: String,
    value: Value,
    expanded: bool,
    depth: usize,
    #[allow(dead_code)]
    path: String,
}

pub struct JsonUtils {
    raw_input: String,
    formatted_json: String,
    error_message: String,
    is_valid: bool,
    view_mode: ViewMode,
    json_tree: Vec<JsonTreeNode>,
    selected_node: usize,
    parsed_value: Option<Value>,
    temp_file: Option<NamedTempFile>,
    file_watcher_rx: Option<mpsc::Receiver<NotifyResult<notify::Event>>>,
    needs_terminal_reinit: bool,
    scroll_offset: usize,
}

impl JsonUtils {
    pub fn new() -> Self {
        Self {
            raw_input: String::new(),
            formatted_json: String::new(),
            error_message: String::new(),
            is_valid: false,
            view_mode: ViewMode::Raw,
            json_tree: Vec::new(),
            selected_node: 0,
            parsed_value: None,
            temp_file: None,
            file_watcher_rx: None,
            needs_terminal_reinit: false,
            scroll_offset: 0,
        }
    }

    pub fn paste_from_clipboard(&mut self) -> Result<()> {
        let mut clipboard = Clipboard::new()?;
        match clipboard.get_text() {
            Ok(text) => {
                self.raw_input = text;
                self.parse_json();
            }
            Err(e) => {
                self.error_message = format!("Failed to get clipboard: {}", e);
                self.is_valid = false;
                self.formatted_json.clear();
            }
        }
        Ok(())
    }

    pub fn copy_to_clipboard(&mut self) -> Result<()> {
        if self.is_valid && !self.formatted_json.is_empty() {
            let mut clipboard = Clipboard::new()?;
            match clipboard.set_text(&self.formatted_json) {
                Ok(_) => {
                    self.error_message = "Copied formatted JSON to clipboard".to_string();
                }
                Err(e) => {
                    self.error_message = format!("Failed to copy to clipboard: {}", e);
                }
            }
        }
        Ok(())
    }

    pub fn copy_minified_to_clipboard(&mut self) -> Result<()> {
        if self.is_valid {
            if let Some(ref value) = self.parsed_value {
                match serde_json::to_string(value) {
                    Ok(minified) => {
                        let mut clipboard = Clipboard::new()?;
                        match clipboard.set_text(&minified) {
                            Ok(_) => {
                                self.error_message = "Copied minified JSON to clipboard".to_string();
                            }
                            Err(e) => {
                                self.error_message = format!("Failed to copy to clipboard: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        self.error_message = format!("Failed to minify JSON: {}", e);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn create_temp_file_for_editing(&mut self) -> Result<()> {
        if self.raw_input.is_empty() {
            self.error_message = "No JSON content to edit".to_string();
            return Ok(());
        }

        let temp_file = NamedTempFile::new()?;
        fs::write(temp_file.path(), &self.raw_input)?;

        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        watcher.watch(temp_file.path(), RecursiveMode::NonRecursive)?;

        self.error_message = format!("Edit this file: {}\nFile is being watched for changes...", temp_file.path().display());
        
        self.temp_file = Some(temp_file);
        self.file_watcher_rx = Some(rx);
        
        Ok(())
    }

    pub fn open_in_neovim(&mut self) -> Result<()> {
        if self.raw_input.is_empty() {
            self.error_message = "No JSON content to edit".to_string();
            return Ok(());
        }

        let temp_file = NamedTempFile::new()?;
        fs::write(temp_file.path(), &self.raw_input)?;

        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        watcher.watch(temp_file.path(), RecursiveMode::NonRecursive)?;

        ratatui::restore();
        
        let status = Command::new("nvim")
            .arg(temp_file.path())
            .status()?;

        if !status.success() {
            self.error_message = "Failed to open Neovim".to_string();
        }

        let updated_content = fs::read_to_string(temp_file.path())?;
        if updated_content != self.raw_input {
            self.raw_input = updated_content;
            self.parse_json();
        }

        self.temp_file = Some(temp_file);
        self.file_watcher_rx = Some(rx);
        self.needs_terminal_reinit = true;
        
        Ok(())
    }

    pub fn check_file_changes(&mut self) -> Result<()> {
        if let Some(ref rx) = self.file_watcher_rx {
            if let Ok(_event) = rx.try_recv() {
                if let Some(ref temp_file) = self.temp_file {
                    match fs::read_to_string(temp_file.path()) {
                        Ok(content) => {
                            if content != self.raw_input {
                                self.raw_input = content;
                                self.parse_json();
                            }
                        }
                        Err(e) => {
                            self.error_message = format!("Failed to read file: {}", e);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn parse_json(&mut self) {
        match serde_json::from_str::<Value>(&self.raw_input) {
            Ok(value) => {
                match serde_json::to_string_pretty(&value) {
                    Ok(formatted) => {
                        self.formatted_json = formatted;
                        self.is_valid = true;
                        self.error_message.clear();
                        self.parsed_value = Some(value.clone());
                        self.build_tree(&value);
                        self.scroll_offset = 0;
                    }
                    Err(e) => {
                        self.error_message = format!("Format error: {}", e);
                        self.is_valid = false;
                        self.parsed_value = None;
                    }
                }
            }
            Err(e) => {
                self.error_message = format!("Invalid JSON: {}", e);
                self.is_valid = false;
                self.formatted_json.clear();
                self.parsed_value = None;
                self.json_tree.clear();
            }
        }
    }

    fn build_tree(&mut self, value: &Value) {
        self.json_tree.clear();
        self.selected_node = 0;
        self.build_tree_recursive(value, "", 0, "root");
    }

    fn build_tree_recursive(&mut self, value: &Value, key: &str, depth: usize, path: &str) {
        let node = JsonTreeNode {
            key: key.to_string(),
            value: value.clone(),
            expanded: depth < 2, // Auto-expand first 2 levels
            depth,
            path: path.to_string(),
        };
        self.json_tree.push(node);

        if let Some(obj) = value.as_object() {
            for (k, v) in obj {
                let new_path = if path == "root" { k.clone() } else { format!("{}.{}", path, k) };
                self.build_tree_recursive(v, k, depth + 1, &new_path);
            }
        } else if let Some(arr) = value.as_array() {
            for (i, v) in arr.iter().enumerate() {
                let new_path = if path == "root" { format!("[{}]", i) } else { format!("{}[{}]", path, i) };
                self.build_tree_recursive(v, &format!("[{}]", i), depth + 1, &new_path);
            }
        }
    }

    fn toggle_node(&mut self) {
        if self.selected_node < self.json_tree.len() {
            let node = &mut self.json_tree[self.selected_node];
            if node.value.is_object() || node.value.is_array() {
                node.expanded = !node.expanded;
            }
        }
    }

    fn move_selection_up(&mut self) {
        let visible_nodes = self.get_visible_nodes();
        if !visible_nodes.is_empty() {
            let current_visible_index = visible_nodes.iter().position(|node| {
                self.json_tree.iter().position(|n| std::ptr::eq(*node, n)) == Some(self.selected_node)
            }).unwrap_or(0);
            
            if current_visible_index > 0 {
                let new_visible_index = current_visible_index - 1;
                if let Some(new_node) = visible_nodes.get(new_visible_index) {
                    if let Some(new_index) = self.json_tree.iter().position(|n| std::ptr::eq(*new_node, n)) {
                        self.selected_node = new_index;
                    }
                }
            }
        }
    }

    fn move_selection_down(&mut self) {
        let visible_nodes = self.get_visible_nodes();
        if !visible_nodes.is_empty() {
            let current_visible_index = visible_nodes.iter().position(|node| {
                self.json_tree.iter().position(|n| std::ptr::eq(*node, n)) == Some(self.selected_node)
            }).unwrap_or(0);
            
            if current_visible_index < visible_nodes.len() - 1 {
                let new_visible_index = current_visible_index + 1;
                if let Some(new_node) = visible_nodes.get(new_visible_index) {
                    if let Some(new_index) = self.json_tree.iter().position(|n| std::ptr::eq(*new_node, n)) {
                        self.selected_node = new_index;
                    }
                }
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Full screen - either raw or tree view
        match self.view_mode {
            ViewMode::Raw => self.render_raw_preview(frame, area),
            ViewMode::Tree => self.render_tree_view(frame, area),
        }
    }

    fn render_raw_preview(&self, frame: &mut Frame, area: Rect) {
        let preview_title = if self.is_valid {
            "JSON Viewer - 'p': paste, 'n': neovim, 't': tree, 'c': copy, 'C': copy minified, 'j/k': scroll, 'q': quit"
        } else if !self.error_message.is_empty() && self.error_message.contains("Edit this file:") {
            "File Created - 'p': paste, 'n': neovim, 't': tree view, 'q': quit"
        } else if !self.error_message.is_empty() {
            "JSON Viewer - 'p': paste, 'n': neovim, 't': tree view, 'q': quit"
        } else {
            "JSON Viewer - 'p': paste, 'n': neovim, 't': tree view, 'q': quit"
        };

        let preview_block = Block::default()
            .title(preview_title)
            .borders(Borders::ALL);

        let preview_content = if self.is_valid {
            &self.formatted_json
        } else if !self.error_message.is_empty() {
            &self.error_message
        } else {
            "Press 'p' to paste JSON from clipboard or 'n' to create new JSON in Neovim"
        };

        let preview_color = if self.is_valid {
            Color::Green
        } else if !self.error_message.is_empty() && self.error_message.contains("Edit this file:") {
            Color::Yellow
        } else if !self.error_message.is_empty() {
            Color::Red
        } else {
            Color::Cyan
        };

        let preview_paragraph = Paragraph::new(preview_content)
            .block(preview_block)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset as u16, 0))
            .style(Style::default().fg(preview_color));
        frame.render_widget(preview_paragraph, area);
    }

    fn render_tree_view(&self, frame: &mut Frame, area: Rect) {
        let tree_title = "JSON Tree - 'p': paste, 'n': neovim, 't': raw, 'c': copy, 'C': copy minified, Space: expand, ↑/↓ j/k: navigate, 'q': quit";
        let tree_block = Block::default()
            .title(tree_title)
            .borders(Borders::ALL);

        if !self.is_valid || self.json_tree.is_empty() {
            let error_paragraph = Paragraph::new("No valid JSON to display")
                .block(tree_block)
                .style(Style::default().fg(Color::Red));
            frame.render_widget(error_paragraph, area);
            return;
        }

        let visible_nodes = self.get_visible_nodes();
        let items: Vec<ListItem> = visible_nodes
            .iter()
            .enumerate()
            .map(|(_i, node)| {
                let indent = "  ".repeat(node.depth);
                let icon = if node.value.is_object() || node.value.is_array() {
                    if node.expanded { "▼" } else { "▶" }
                } else {
                    " "
                };
                
                let value_preview = match &node.value {
                    Value::Object(obj) => format!("{{ {} keys }}", obj.len()),
                    Value::Array(arr) => format!("[ {} items ]", arr.len()),
                    Value::String(s) => format!("\"{}\"", s),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => "null".to_string(),
                };

                let display_key = if node.key.is_empty() { "root".to_string() } else { node.key.clone() };
                let content = format!("{}{} {}: {}", indent, icon, display_key, value_preview);
                
                // Check if this visible node is the currently selected node
                let is_selected = self.json_tree.iter().position(|n| std::ptr::eq(*node, n)) == Some(self.selected_node);
                let style = if is_selected {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(content).style(style)
            })
            .collect();

        let tree_list = List::new(items)
            .block(tree_block)
            .highlight_symbol(">> ");

        frame.render_widget(tree_list, area);
    }

    fn get_visible_nodes(&self) -> Vec<&JsonTreeNode> {
        let mut visible = Vec::new();
        let mut skip_depth = None;

        for node in &self.json_tree {
            if let Some(depth) = skip_depth {
                if node.depth > depth {
                    continue;
                } else {
                    skip_depth = None;
                }
            }

            visible.push(node);

            if (node.value.is_object() || node.value.is_array()) && !node.expanded {
                skip_depth = Some(node.depth);
            }
        }

        visible
    }

    pub fn handle_event(&mut self, event: Event) -> Result<bool> {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('q') if key.kind == KeyEventKind::Press => return Ok(false),
                KeyCode::Char('p') if key.kind == KeyEventKind::Press => {
                    self.paste_from_clipboard()?;
                }
                KeyCode::Char('e') if key.kind == KeyEventKind::Press => {
                    self.create_temp_file_for_editing()?;
                }
                KeyCode::Char('n') if key.kind == KeyEventKind::Press => {
                    self.open_in_neovim()?;
                }
                KeyCode::Char('t') if key.kind == KeyEventKind::Press => {
                    self.view_mode = if self.view_mode == ViewMode::Tree {
                        ViewMode::Raw
                    } else {
                        ViewMode::Tree
                    };
                }
                KeyCode::Up | KeyCode::Char('k') if key.kind == KeyEventKind::Press => {
                    if self.view_mode == ViewMode::Tree {
                        self.move_selection_up();
                    } else {
                        if self.scroll_offset > 0 {
                            self.scroll_offset -= 1;
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') if key.kind == KeyEventKind::Press => {
                    if self.view_mode == ViewMode::Tree {
                        self.move_selection_down();
                    } else {
                        self.scroll_offset += 1;
                    }
                }
                KeyCode::Char(' ') if key.kind == KeyEventKind::Press && self.view_mode == ViewMode::Tree => {
                    self.toggle_node();
                }
                KeyCode::Enter if key.kind == KeyEventKind::Press && self.view_mode == ViewMode::Tree => {
                    self.toggle_node();
                }
                KeyCode::Char('c') if key.kind == KeyEventKind::Press => {
                    self.copy_to_clipboard()?;
                }
                KeyCode::Char('C') if key.kind == KeyEventKind::Press => {
                    self.copy_minified_to_clipboard()?;
                }
                KeyCode::Esc if key.kind == KeyEventKind::Press => return Ok(false),
                _ => {}
            }
        }
        Ok(true)
    }
}

pub fn run_json_utils() -> Result<()> {
    let mut terminal = ratatui::init();
    let mut json_utils = JsonUtils::new();

    loop {
        json_utils.check_file_changes()?;

        if json_utils.needs_terminal_reinit {
            terminal = ratatui::init();
            json_utils.needs_terminal_reinit = false;
        }

        terminal.draw(|frame| {
            json_utils.render(frame, frame.area());
        })?;

        if event::poll(Duration::from_millis(100))? {
            let event = event::read()?;
            if !json_utils.handle_event(event)? {
                break;
            }
        }
    }

    ratatui::restore();
    Ok(())
}