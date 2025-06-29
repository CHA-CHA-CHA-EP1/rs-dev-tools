use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::io::Stdout;

mod modules;

enum InputMode {
    Normal,
    Editing,
}

struct MainMenu {
    input: String,
    cursor_position: usize,
    input_mode: InputMode,
    selected: usize,
}

impl MainMenu {
    fn new() -> MainMenu {
        MainMenu {
            input: String::new(),
            cursor_position: 0,
            input_mode: InputMode::Normal,
            selected: 0,
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
        self.input.insert(self.cursor_position, new_char);
        self.move_cursor_right();
        self.selected = 0;
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.cursor_position != 0;
        if is_not_cursor_leftmost {
            let current_index = self.cursor_position;
            let from_left_to_current_index = current_index - 1;
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input.chars().skip(current_index);
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
            self.selected = 0;
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.len())
    }

    fn next_item(&mut self, items_len: usize) {
        if items_len > 0 {
            self.selected = (self.selected + 1) % items_len;
        }
    }

    fn previous_item(&mut self, items_len: usize) {
        if items_len > 0 {
            self.selected = if self.selected == 0 {
                items_len - 1
            } else {
                self.selected - 1
            };
        }
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let app_result = run_main_menu(&mut terminal);
    ratatui::restore();
    app_result
}

fn run_main_menu(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    let mut menu = MainMenu::new();
    let all_programs = vec![
        ("JSON Utils", "JSON viewer, formatter, and validator"),
        ("Base64 Tools", "Base64 encode/decode utilities"),
        ("String Utils", "String manipulation tools"),
        ("File Tools", "File operations and utilities"),
    ];

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(3),
                ])
                .split(area);

            let title = Paragraph::new("🚀 Dev Tools Menu")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center);
            frame.render_widget(title, chunks[0]);

            let input_title = match menu.input_mode {
                InputMode::Normal => "Filter Programs (Press 'i' to search, 'q' to quit)",
                InputMode::Editing => "Filter Programs (Press 'Esc' to stop searching)",
            };
            let input_block = Block::default().title(input_title).borders(Borders::ALL);
            let input_paragraph = Paragraph::new(menu.input.as_str())
                .block(input_block)
                .style(match menu.input_mode {
                    InputMode::Normal => Style::default().fg(Color::Gray),
                    InputMode::Editing => Style::default().fg(Color::Yellow),
                });
            frame.render_widget(input_paragraph, chunks[1]);

            if matches!(menu.input_mode, InputMode::Editing) {
                frame.set_cursor_position((
                    chunks[1].x + menu.cursor_position as u16 + 1,
                    chunks[1].y + 1,
                ));
            }

            let filtered_programs: Vec<(&str, &str)> = all_programs
                .iter()
                .filter(|(name, _desc)| {
                    if menu.input.is_empty() {
                        true
                    } else {
                        name.to_lowercase().contains(&menu.input.to_lowercase())
                    }
                })
                .copied()
                .collect();

            let program_list: Vec<ListItem> = filtered_programs
                .iter()
                .enumerate()
                .map(|(i, (name, desc))| {
                    let style = if i == menu.selected {
                        Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ListItem::new(format!("{} - {}", name, desc)).style(style)
                })
                .collect();

            let program_menu = List::new(program_list)
                .block(
                    Block::default()
                        .title("Available Programs (↑/↓ or j/k to navigate, Enter to select)")
                        .borders(Borders::ALL)
                )
                .highlight_symbol(">> ");

            frame.render_widget(program_menu, chunks[2]);

            let help = Paragraph::new("i: search, ↑/↓ j/k: navigate, Enter: select, q: quit")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
            frame.render_widget(help, chunks[3]);
        })?;

        if let Event::Key(key) = event::read()? {
            match menu.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('q') if key.kind == KeyEventKind::Press => break,
                    KeyCode::Char('i') if key.kind == KeyEventKind::Press => {
                        menu.input_mode = InputMode::Editing;
                    }
                    KeyCode::Up | KeyCode::Char('k') if key.kind == KeyEventKind::Press => {
                        let filtered_count = all_programs
                            .iter()
                            .filter(|(name, _)| {
                                if menu.input.is_empty() {
                                    true
                                } else {
                                    name.to_lowercase().contains(&menu.input.to_lowercase())
                                }
                            })
                            .count();
                        menu.previous_item(filtered_count);
                    }
                    KeyCode::Down | KeyCode::Char('j') if key.kind == KeyEventKind::Press => {
                        let filtered_count = all_programs
                            .iter()
                            .filter(|(name, _)| {
                                if menu.input.is_empty() {
                                    true
                                } else {
                                    name.to_lowercase().contains(&menu.input.to_lowercase())
                                }
                            })
                            .count();
                        menu.next_item(filtered_count);
                    }
                    KeyCode::Enter if key.kind == KeyEventKind::Press => {
                        let filtered_programs: Vec<_> = all_programs
                            .iter()
                            .filter(|(name, _)| {
                                if menu.input.is_empty() {
                                    true
                                } else {
                                    name.to_lowercase().contains(&menu.input.to_lowercase())
                                }
                            })
                            .collect();
                        
                        if menu.selected < filtered_programs.len() {
                            let selected_program = filtered_programs[menu.selected].0;
                            match selected_program {
                                "JSON Utils" => {
                                    ratatui::restore();
                                    modules::json_utils::run_json_utils()?;
                                    *terminal = ratatui::init();
                                }
                                _ => {
                                    // TODO: Implement other programs
                                }
                            }
                        }
                    }
                    _ => {}
                },
                InputMode::Editing => match key.code {
                    KeyCode::Esc if key.kind == KeyEventKind::Press => {
                        menu.input_mode = InputMode::Normal;
                    }
                    KeyCode::Char(c) if key.kind == KeyEventKind::Press => {
                        menu.enter_char(c);
                    }
                    KeyCode::Backspace if key.kind == KeyEventKind::Press => {
                        menu.delete_char();
                    }
                    KeyCode::Left if key.kind == KeyEventKind::Press => {
                        menu.move_cursor_left();
                    }
                    KeyCode::Right if key.kind == KeyEventKind::Press => {
                        menu.move_cursor_right();
                    }
                    _ => {}
                },
            }
        }
    }
    Ok(())
}

