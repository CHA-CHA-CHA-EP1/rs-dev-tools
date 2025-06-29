use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

enum InputMode {
    Normal,
    Editing,
}

struct App {
    input: String,
    cursor_position: usize,
    input_mode: InputMode,
    selected: usize,
}

impl App {
    fn new() -> App {
        App {
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
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.len())
    }

    fn next_todo(&mut self, todos_len: usize) {
        if todos_len > 0 {
            self.selected = (self.selected + 1) % todos_len;
        }
    }

    fn previous_todo(&mut self, todos_len: usize) {
        if todos_len > 0 {
            self.selected = if self.selected == 0 {
                todos_len - 1
            } else {
                self.selected - 1
            };
        }
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let app_result = run(&mut terminal);
    ratatui::restore();
    app_result
}

fn run(terminal: &mut Terminal<impl Backend>) -> Result<()> {
    let mut app = App::new();

    loop {
        let all_todos = vec![
            "Todo 1: Learn Rust",
            "Todo 2: Build terminal app",
            "Todo 3: Master ratatui",
            "Todo 4: Create awesome UI",
            "Todo 5: Debug performance issues",
            "Todo 6: Write documentation",
        ];

        terminal.draw(|frame| {
            let area = frame.area();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(1)])
                .split(area);

            let input_title = match app.input_mode {
                InputMode::Normal => "Input (Press 'i' to edit)",
                InputMode::Editing => "Input (Press 'Esc' to exit editing)",
            };
            let input_block = Block::default().title(input_title).borders(Borders::ALL);
            let input_paragraph =
                Paragraph::new(app.input.as_str())
                    .block(input_block)
                    .style(match app.input_mode {
                        InputMode::Normal => Style::default().fg(Color::Gray),
                        InputMode::Editing => Style::default().fg(Color::Yellow),
                    });
            frame.render_widget(input_paragraph, chunks[0]);

            if matches!(app.input_mode, InputMode::Editing) {
                frame.set_cursor_position((
                    chunks[0].x + app.cursor_position as u16 + 1,
                    chunks[0].y + 1,
                ));
            }

            let filtered_todos: Vec<ListItem> = all_todos
                .iter()
                .filter(|todo| {
                    if app.input.is_empty() {
                        true
                    } else {
                        todo.to_lowercase().contains(&app.input.to_lowercase())
                    }
                })
                .map(|todo| ListItem::new(*todo))
                .collect();

            let filtered_count = filtered_todos.len();
            let todo_list = List::new(filtered_todos)
                .block(
                    Block::default()
                        .title("Todos (j/k to navigate)")
                        .borders(Borders::ALL),
                )
                .style(Style::default().fg(Color::White))
                .highlight_style(
                    Style::default()
                        .add_modifier(Modifier::ITALIC)
                        .bg(Color::Blue),
                )
                .highlight_symbol(">> ");

            let mut list_state = ratatui::widgets::ListState::default();
            if filtered_count > 0 {
                list_state.select(Some(app.selected.min(filtered_count - 1)));
            }
            frame.render_stateful_widget(todo_list, chunks[1], &mut list_state);
        })?;

        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('q') if key.kind == KeyEventKind::Press => break,
                    KeyCode::Char('i') if key.kind == KeyEventKind::Press => {
                        app.input_mode = InputMode::Editing;
                    }
                    KeyCode::Char('j') if key.kind == KeyEventKind::Press => {
                        let filtered_count = all_todos
                            .iter()
                            .filter(|todo| {
                                if app.input.is_empty() {
                                    true
                                } else {
                                    todo.to_lowercase().contains(&app.input.to_lowercase())
                                }
                            })
                            .count();
                        app.next_todo(filtered_count);
                    }
                    KeyCode::Char('k') if key.kind == KeyEventKind::Press => {
                        let filtered_count = all_todos
                            .iter()
                            .filter(|todo| {
                                if app.input.is_empty() {
                                    true
                                } else {
                                    todo.to_lowercase().contains(&app.input.to_lowercase())
                                }
                            })
                            .count();
                        app.previous_todo(filtered_count);
                    }
                    _ => {}
                },
                InputMode::Editing => match key.code {
                    KeyCode::Esc if key.kind == KeyEventKind::Press => {
                        app.input_mode = InputMode::Normal;
                    }
                    KeyCode::Char(c) if key.kind == KeyEventKind::Press => {
                        app.enter_char(c);
                    }
                    KeyCode::Backspace if key.kind == KeyEventKind::Press => {
                        app.delete_char();
                    }
                    KeyCode::Left if key.kind == KeyEventKind::Press => {
                        app.move_cursor_left();
                    }
                    KeyCode::Right if key.kind == KeyEventKind::Press => {
                        app.move_cursor_right();
                    }
                    _ => {}
                },
            }
        }
    }
    Ok(())
}
