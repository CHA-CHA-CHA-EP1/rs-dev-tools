use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};
use serde_json::{self, Value};
use arboard::Clipboard;

#[derive(PartialEq)]
enum InputMode {
    Normal,
    Editing,
}

pub struct JsonUtils {
    raw_input: String,
    formatted_json: String,
    error_message: String,
    is_valid: bool,
    input_mode: InputMode,
    cursor_position: usize,
}

impl JsonUtils {
    pub fn new() -> Self {
        Self {
            raw_input: String::new(),
            formatted_json: String::new(),
            error_message: String::new(),
            is_valid: false,
            input_mode: InputMode::Normal,
            cursor_position: 0,
        }
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.cursor_position.saturating_sub(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.cursor_position.saturating_add(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        self.raw_input.insert(self.cursor_position, new_char);
        self.move_cursor_right();
        self.parse_json();
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.cursor_position != 0;
        if is_not_cursor_leftmost {
            let current_index = self.cursor_position;
            let from_left_to_current_index = current_index - 1;
            let before_char_to_delete = self.raw_input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.raw_input.chars().skip(current_index);
            self.raw_input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
            self.parse_json();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.raw_input.len())
    }

    pub fn paste_from_clipboard(&mut self) -> Result<()> {
        let mut clipboard = Clipboard::new()?;
        match clipboard.get_text() {
            Ok(text) => {
                self.raw_input = text;
                self.cursor_position = self.raw_input.len();
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

    fn parse_json(&mut self) {
        match serde_json::from_str::<Value>(&self.raw_input) {
            Ok(value) => {
                match serde_json::to_string_pretty(&value) {
                    Ok(formatted) => {
                        self.formatted_json = formatted;
                        self.is_valid = true;
                        self.error_message.clear();
                    }
                    Err(e) => {
                        self.error_message = format!("Format error: {}", e);
                        self.is_valid = false;
                    }
                }
            }
            Err(e) => {
                self.error_message = format!("Invalid JSON: {}", e);
                self.is_valid = false;
                self.formatted_json.clear();
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let raw_title = match self.input_mode {
            InputMode::Normal => "Raw Input (Press 'i' to edit, 'p' to paste from clipboard)",
            InputMode::Editing => "Raw Input (Press 'Esc' to stop editing)",
        };
        let raw_block = Block::default()
            .title(raw_title)
            .borders(Borders::ALL);
        let raw_paragraph = Paragraph::new(self.raw_input.as_str())
            .block(raw_block)
            .wrap(Wrap { trim: true })
            .style(match self.input_mode {
                InputMode::Normal => Style::default().fg(Color::Gray),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            });
        frame.render_widget(raw_paragraph, chunks[0]);

        if self.input_mode == InputMode::Editing {
            let cursor_x = self.cursor_position as u16 % (chunks[0].width.saturating_sub(2));
            let cursor_y = self.cursor_position as u16 / (chunks[0].width.saturating_sub(2));
            frame.set_cursor_position((
                chunks[0].x + cursor_x + 1,
                chunks[0].y + cursor_y + 1,
            ));
        }

        let preview_title = if self.is_valid {
            "JSON Preview"
        } else if !self.error_message.is_empty() {
            "Error"
        } else {
            "JSON Preview (Paste JSON to see preview)"
        };

        let preview_block = Block::default()
            .title(preview_title)
            .borders(Borders::ALL);

        let preview_content = if self.is_valid {
            &self.formatted_json
        } else if !self.error_message.is_empty() {
            &self.error_message
        } else {
            "No JSON data"
        };

        let preview_color = if self.is_valid {
            Color::Green
        } else if !self.error_message.is_empty() {
            Color::Red
        } else {
            Color::Gray
        };

        let preview_paragraph = Paragraph::new(preview_content)
            .block(preview_block)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(preview_color));
        frame.render_widget(preview_paragraph, chunks[1]);
    }

    pub fn handle_event(&mut self, event: Event) -> Result<bool> {
        if let Event::Key(key) = event {
            match self.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('q') if key.kind == KeyEventKind::Press => return Ok(false),
                    KeyCode::Char('i') if key.kind == KeyEventKind::Press => {
                        self.input_mode = InputMode::Editing;
                    }
                    KeyCode::Char('p') if key.kind == KeyEventKind::Press => {
                        self.paste_from_clipboard()?;
                    }
                    KeyCode::Esc if key.kind == KeyEventKind::Press => return Ok(false),
                    _ => {}
                },
                InputMode::Editing => match key.code {
                    KeyCode::Esc if key.kind == KeyEventKind::Press => {
                        self.input_mode = InputMode::Normal;
                    }
                    KeyCode::Char(c) if key.kind == KeyEventKind::Press => {
                        self.enter_char(c);
                    }
                    KeyCode::Backspace if key.kind == KeyEventKind::Press => {
                        self.delete_char();
                    }
                    KeyCode::Left if key.kind == KeyEventKind::Press => {
                        self.move_cursor_left();
                    }
                    KeyCode::Right if key.kind == KeyEventKind::Press => {
                        self.move_cursor_right();
                    }
                    KeyCode::Enter if key.kind == KeyEventKind::Press => {
                        self.enter_char('\n');
                    }
                    _ => {}
                },
            }
        }
        Ok(true)
    }
}

pub fn run_json_utils() -> Result<()> {
    let mut terminal = ratatui::init();
    let mut json_utils = JsonUtils::new();

    loop {
        terminal.draw(|frame| {
            json_utils.render(frame, frame.area());
        })?;

        let event = event::read()?;
        if !json_utils.handle_event(event)? {
            break;
        }
    }

    ratatui::restore();
    Ok(())
}