use std::{
    cmp::min,
    collections::{HashMap, VecDeque},
    default,
    fs::File,
    hash::Hash,
    io::{Read, Write},
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

#[derive(Default, Clone, Serialize, Deserialize)]
struct DirEntry {
    path: String,
    is_dir: bool,
    size: Option<usize>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
struct WorkingDirectory {
    path: String,
    children: Vec<Option<DirEntry>>,
    curr_index: usize,
}

#[derive(Default, Serialize, Deserialize)]
struct State {
    cwd: Option<WorkingDirectory>,
    history_backward: Vec<WorkingDirectory>,
    history_forward: Vec<WorkingDirectory>,
    selected: bool,
}

#[derive(Default)]
pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    state: State,
}

const SETTINGS_FILE_NAME: &str = "fe-rs-settings.json";

impl Home {
    pub fn new() -> Self {
        Self { state: Home::load_settings().unwrap(), ..Self::default() }
    }

    fn load_settings() -> Result<State> {
        if let Ok(mut fl) = File::open(SETTINGS_FILE_NAME) {
            let mut settings_file_data = String::new();
            if let Ok(num_bytes) = fl.read_to_string(&mut settings_file_data) {
                if let Ok(state) = serde_json::from_str(&settings_file_data) {
                    return Ok(state);
                }
            }
        }

        Ok(State {
            cwd: Some(WorkingDirectory { path: "\\".to_string(), children: vec![], curr_index: 0 }),
            ..State::default()
        })
    }

    fn save_settings(state: &State) {
        if let Ok(state_serialized) = serde_json::to_string(state) {
            if let Ok(mut fl) = File::create(SETTINGS_FILE_NAME) {
                fl.write_all(state_serialized.as_bytes()).unwrap();
            }
        }
    }

    fn run_launch_cmd(&self) {
        if !cfg!(target_os = "windows") {
            // TODO: Support other platforms
            return;
        }

        let cwd = self.state.cwd.as_ref().unwrap();
        if let Some(child) = &cwd.children[cwd.curr_index] {
            if std::process::Command::new("cmd")
                .args(["/C", "start", Path::new(&child.path).to_str().unwrap()])
                .spawn()
                .is_err()
            {
                // TODO: error dialog
            }
        }
    }

    fn run_explorer_cmd(&self) {
        if !cfg!(target_os = "windows") {
            // TODO: Support other platforms
        }

        let cwd = self.state.cwd.as_ref().unwrap();
        if std::process::Command::new("explorer").arg(Path::new(&cwd.path)).spawn().is_err() {
            // TODO: error dialog
        }
    }

    fn run_shell_cmd(&self) {
        if !cfg!(target_os = "windows") {
            // TODO: Support other platforms
            return;
        }

        let cwd = self.state.cwd.as_ref().unwrap();
        if std::process::Command::new("cmd").current_dir(&cwd.path).args(["/C", "start", "powershell"]).spawn().is_err()
        {
            // TODO: error dialog
        }
    }
}

fn get_dir_entry_icon(dir_entry_text: &str) -> String {
    if dir_entry_text.ends_with(".mp3") {
        "🎹 ".to_string()
    } else if dir_entry_text.starts_with(".") {
        "⚙ ".to_string()
    } else if dir_entry_text.ends_with(".exe") {
        "💾 ".to_string()
    } else if dir_entry_text.ends_with(".zip") {
        "🗜 ".to_string()
    } else if dir_entry_text.ends_with(".png")
        || dir_entry_text.ends_with(".jpg")
        || dir_entry_text.ends_with(".jpeg")
        || dir_entry_text.ends_with(".bmp")
    {
        "🖼 ".to_string()
    } else {
        "📃 ".to_string()
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
        let cwd = self.state.cwd.as_mut().unwrap();
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if cwd.curr_index != cwd.children.len() {
                    cwd.curr_index += 1;
                    Home::save_settings(&self.state);
                }
            },
            KeyCode::Char('k') | KeyCode::Up => {
                if 0 != cwd.curr_index {
                    cwd.curr_index -= 1;
                    Home::save_settings(&self.state);
                }
            },
            KeyCode::Enter => {
                self.state.selected = true;
            },
            KeyCode::Left | KeyCode::Char('h') => {
                if key.modifiers & KeyModifiers::ALT == KeyModifiers::ALT {
                    if let Some(history_item) = self.state.history_backward.pop() {
                        self.state.history_forward.push(self.state.cwd.take().unwrap());
                        self.state.cwd = Some(history_item);
                        Home::save_settings(&self.state);
                    }
                }
            },
            KeyCode::Right | KeyCode::Char('l') => {
                if key.modifiers & KeyModifiers::ALT == KeyModifiers::ALT {
                    if let Some(history_item) = self.state.history_forward.pop() {
                        self.state.history_backward.push(self.state.cwd.take().unwrap());
                        self.state.cwd = Some(history_item);
                        Home::save_settings(&self.state);
                    }
                }
            },
            KeyCode::Char('e') => {
                self.run_explorer_cmd();
            },
            KeyCode::Char('s') => {
                self.run_shell_cmd();
            },
            _ => {},
        }

        Ok(None)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {},
            Action::Help => {},
            _ => {},
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        let cwd = self.state.cwd.as_mut().unwrap();

        if self.state.selected {
            let selected_item = cwd.children[cwd.curr_index].as_ref().unwrap().path.clone();
            if Path::new(&selected_item).is_dir() {
                let cwd = self.state.cwd.take().unwrap();
                self.state.cwd =
                    Some(WorkingDirectory { path: selected_item, children: vec![], curr_index: cwd.curr_index });
                self.state.history_backward.push(cwd);
            } else {
                self.run_launch_cmd();
            }
            self.state.selected = false;
            Home::save_settings(&self.state);
        }

        let cwd = self.state.cwd.as_mut().unwrap();

        if cwd.children.is_empty() {
            if let Ok(res) = Path::new(&cwd.path).read_dir() {
                for entry in res.flatten() {
                    let path = entry.path().to_str().unwrap().to_string();
                    if path.is_empty() {
                        continue;
                    }
                    cwd.children.push(Some(DirEntry {
                        is_dir: Path::new(&path).is_dir(),
                        path,
                        ..DirEntry::default()
                    }));
                }
            }
        }

        cwd.curr_index = min(cwd.curr_index, cwd.children.len() - 1);

        let mut constraints = [Constraint::Length(3) /* top bar */].to_vec();
        constraints.append(&mut cwd.children.iter().map(|_| Constraint::Length(1)).collect::<Vec<Constraint>>());
        let regions = Layout::default().direction(Direction::Vertical).constraints(constraints).split(area);

        let top_bar = Paragraph::new(cwd.path.as_str()).block(
            Block::default()
                .title("Current location")
                .border_style(Style::new().light_magenta())
                .borders(Borders::TOP | Borders::BOTTOM),
        );
        f.render_widget(top_bar, regions[0 /* top bar */]);

        for i in 0..cwd.children.len() {
            let dir_entry = cwd.children[i].as_ref().unwrap();
            if let Some(file_name) = Path::new(&dir_entry.path.clone()).file_name() {
                if let Some(file_name) = file_name.to_str() {
                    let mut dir_entry_text = file_name.to_string();
                    if dir_entry.is_dir {
                        dir_entry_text = "📂 ".to_owned() + &dir_entry_text;
                    } else {
                        dir_entry_text = get_dir_entry_icon(&dir_entry_text) + &dir_entry_text;
                    }

                    let mut paragraph = Paragraph::new(dir_entry_text.as_str());
                    let is_selected = i == cwd.curr_index;
                    if is_selected {
                        paragraph = paragraph.set_style(Style::new().bg(Color::LightMagenta));
                    }
                    f.render_widget(paragraph, regions[i + 1]);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Home;
}
