use std::{
    cmp::min,
    collections::{HashMap, VecDeque},
    default,
    hash::Hash,
    path::Path,
    time::Duration,
    vec,
};

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use super::{Component, Frame};
use crate::{
    action::Action,
    config::{Config, KeyBindings},
};

#[derive(Default, Clone)]
struct DirEntry {
    path: String,
    is_dir: bool,
    size: Option<usize>,
}

#[derive(Default, Clone)]
struct WorkingDirectory {
    path: String,
    children: Vec<Option<DirEntry>>,
    curr_index: usize,
}

#[derive(Default)]
pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    cwd: Option<WorkingDirectory>,
    history_backward: Vec<WorkingDirectory>,
    history_forward: Vec<WorkingDirectory>,
    selected: bool,
}

impl Home {
    pub fn new() -> Self {
        Self {
            cwd: Some(WorkingDirectory { path: "\\".to_string(), children: vec![], curr_index: 0 }),
            ..Self::default()
        }
    }
}

fn get_dir_entry_icon(dir_entry_text: &str) -> String {
    if dir_entry_text.ends_with(".mp3") {
        '🎹'.to_string()
    } else if dir_entry_text.starts_with(".") {
        '⚙'.to_string()
    } else if dir_entry_text.ends_with(".exe") {
        '💾'.to_string()
    } else if dir_entry_text.ends_with(".zip") {
        '🗜'.to_string()
    } else {
        '📃'.to_string()
    }
}

impl Component for Home {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        let cwd = self.cwd.as_mut().unwrap();
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if cwd.curr_index != cwd.children.len() {
                    cwd.curr_index = cwd.curr_index + 1;
                }
            },
            KeyCode::Char('k') | KeyCode::Up => {
                if 0 != cwd.curr_index {
                    cwd.curr_index = cwd.curr_index - 1;
                }
            },
            KeyCode::Enter => {
                self.selected = true;
            },
            KeyCode::Left => {
                if key.modifiers & KeyModifiers::ALT == KeyModifiers::ALT {
                    if let Some(history_item) = self.history_backward.pop() {
                        self.history_forward.push(self.cwd.take().unwrap());
                        self.cwd = Some(history_item);
                    }
                }
            },
            KeyCode::Right => {
                if key.modifiers & KeyModifiers::ALT == KeyModifiers::ALT {
                    if let Some(history_item) = self.history_forward.pop() {
                        self.history_backward.push(self.cwd.take().unwrap());
                        self.cwd = Some(history_item);
                    }
                }
            },
            _ => {},
        }

        Ok(None)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                // if self.selected {
                //   self.cwd =
                // }
            },
            Action::Help => {},
            _ => {},
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        let cwd = self.cwd.as_mut().unwrap();

        if self.selected {
            let selected_item = cwd.children[cwd.curr_index].as_ref().unwrap().path.clone();
            if Path::new(&selected_item).is_dir() {
                let cwd = self.cwd.take().unwrap();
                self.cwd = Some(WorkingDirectory { path: selected_item, children: vec![], curr_index: cwd.curr_index });
                self.history_backward.push(cwd);
            }
            self.selected = false;
        }

        let cwd = self.cwd.as_mut().unwrap();

        if cwd.children.is_empty() {
            cwd.children.push(Some(DirEntry { path: "..".to_owned(), ..DirEntry::default() }));
            if let Ok(res) = Path::new(&cwd.path).read_dir() {
                for entry in res.flatten() {
                    let path = entry.path().to_str().unwrap().to_string();
                    cwd.children.push(Some(DirEntry {
                        is_dir: Path::new(&path).is_dir(),
                        path,
                        ..DirEntry::default()
                    }));
                }
            }
        }

        cwd.curr_index = min(cwd.curr_index, cwd.children.len() - 1);

        let lines = Layout::default()
            .direction(Direction::Vertical)
            .constraints(cwd.children.iter().map(|_| Constraint::Length(1)).collect::<Vec<Constraint>>())
            .split(area);

        for i in 0..cwd.children.len() {
            let dir_entry = cwd.children[i].as_ref().unwrap();
            let mut dir_entry_text = dir_entry.path.clone();
            if dir_entry.is_dir {
                dir_entry_text = "📂".to_owned() + &dir_entry_text;
            } else {
                dir_entry_text = get_dir_entry_icon(&dir_entry_text) + &dir_entry_text;
            }

            let mut paragraph = Paragraph::new(dir_entry_text.as_str());
            let is_selected = i == cwd.curr_index;
            if is_selected {
                paragraph = paragraph.set_style(Style::new().bg(Color::Magenta));
            }
            f.render_widget(paragraph, lines[i]);
        }

        Ok(())
    }
}
