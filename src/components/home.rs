use std::{
    cmp::{max, min},
    collections::{HashMap, VecDeque},
    default,
    fs::File,
    hash::Hash,
    io::{Read, Write},
    path::Path,
    time::Duration,
    vec,
};

use color_eyre::{eyre::Result, owo_colors::OwoColorize};
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
    children: Vec<DirEntry>,
    curr_index: usize,
}

#[derive(Default, Serialize, Deserialize, Clone)]
struct Tab {
    cwd: WorkingDirectory,
    history_backward: Vec<WorkingDirectory>,
    history_forward: Vec<WorkingDirectory>,
    selected: bool,
}

#[derive(Default, Serialize, Deserialize)]
struct State {
    tabs: Vec<Tab>,
    curr_tab_index: usize,
}

#[derive(Default)]
pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    state: State,
}

const SETTINGS_FILE_NAME: &str = "fe-rs-settings.json";
const UI_REGION_TABS_BAR: usize = 0;
const UI_REGION_ADDRESS_BAR: usize = 1;
const UI_REGION_DIR_ENTRIES: usize = 2;
const UI_TAB_WIDTH: u16 = 10;
const UI_SPACE_BETWEEN_TABS: u16 = 2;

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
            tabs: vec![Tab {
                cwd: WorkingDirectory { path: "\\".to_string(), children: vec![], curr_index: 0 },
                ..Tab::default()
            }],
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

        let cwd = &self.state.tabs[self.state.curr_tab_index].cwd;
        let child = &cwd.children[cwd.curr_index];
        if std::process::Command::new("cmd")
            .args(["/C", "start", Path::new(&child.path).to_str().unwrap()])
            .spawn()
            .is_err()
        {
            // TODO: error dialog
        }
    }

    fn run_explorer_cmd(&self) {
        if !cfg!(target_os = "windows") {
            // TODO: Support other platforms
        }

        let cwd = &self.state.tabs[self.state.curr_tab_index].cwd;
        if std::process::Command::new("explorer").arg(Path::new(&cwd.path)).spawn().is_err() {
            // TODO: error dialog
        }
    }

    fn run_shell_cmd(&self) {
        if !cfg!(target_os = "windows") {
            // TODO: Support other platforms
            return;
        }

        let cwd = &self.state.tabs[self.state.curr_tab_index].cwd;
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
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let cwd = &mut self.state.tabs[self.state.curr_tab_index].cwd;
                if cwd.curr_index != cwd.children.len() {
                    cwd.curr_index += 1;
                }
            },
            KeyCode::Char('k') | KeyCode::Up => {
                if key.modifiers & KeyModifiers::ALT == KeyModifiers::ALT {
                    if let Some(path) = Path::new(&self.state.tabs[self.state.curr_tab_index].cwd.path).parent() {
                        let new_path = path.to_str().unwrap().to_string();
                        let current_cwd = self.state.tabs[self.state.curr_tab_index].cwd.clone();
                        self.state.tabs[self.state.curr_tab_index].history_backward.push(current_cwd);
                        self.state.tabs[self.state.curr_tab_index].cwd.path = new_path;
                        self.state.tabs[self.state.curr_tab_index].cwd.children.clear();
                        Home::save_settings(&self.state);
                    }
                } else {
                    let cwd = &mut self.state.tabs[self.state.curr_tab_index].cwd;
                    if 0 != cwd.curr_index {
                        cwd.curr_index -= 1;
                    }
                }
            },
            KeyCode::Enter => {
                self.state.tabs[self.state.curr_tab_index].selected = true;
            },
            KeyCode::Left | KeyCode::Char('h') => {
                if key.modifiers & KeyModifiers::ALT == KeyModifiers::ALT {
                    if let Some(history_item) = self.state.tabs[self.state.curr_tab_index].history_backward.pop() {
                        let old_cwd = self.state.tabs[self.state.curr_tab_index].cwd.clone();
                        self.state.tabs[self.state.curr_tab_index].history_forward.push(old_cwd);
                        self.state.tabs[self.state.curr_tab_index].cwd = history_item;
                    }
                } else if self.state.curr_tab_index != 0 {
                    self.state.curr_tab_index -= 1;
                }
                Home::save_settings(&self.state);
            },
            KeyCode::Right | KeyCode::Char('l') => {
                if key.modifiers & KeyModifiers::ALT == KeyModifiers::ALT {
                    if let Some(history_item) = self.state.tabs[self.state.curr_tab_index].history_forward.pop() {
                        let old_cwd = self.state.tabs[self.state.curr_tab_index].cwd.clone();
                        self.state.tabs[self.state.curr_tab_index].history_backward.push(old_cwd);
                        self.state.tabs[self.state.curr_tab_index].cwd = history_item;
                    }
                } else if self.state.curr_tab_index != self.state.tabs.len() - 1 {
                    self.state.curr_tab_index += 1;
                }
                Home::save_settings(&self.state);
            },
            KeyCode::Char('e') => {
                self.run_explorer_cmd();
            },
            KeyCode::Char('s') => {
                self.run_shell_cmd();
            },
            KeyCode::Char('t') => {
                self.state.tabs.push(self.state.tabs[self.state.curr_tab_index].clone());
                Home::save_settings(&self.state);
            },
            KeyCode::Char('x') => {
                self.state.tabs.remove(self.state.curr_tab_index);
                if self.state.curr_tab_index >= self.state.tabs.len() && self.state.curr_tab_index != 0 {
                    self.state.curr_tab_index -= 1;
                }
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
        let num_tabs = self.state.tabs.len();

        if self.state.tabs[self.state.curr_tab_index].selected {
            self.state.tabs[self.state.curr_tab_index].selected = false;
            let selected_item = self.state.tabs[self.state.curr_tab_index].cwd.children
                [self.state.tabs[self.state.curr_tab_index].cwd.curr_index]
                .path
                .clone();
            if Path::new(&selected_item).is_dir() {
                let old_cwd = self.state.tabs[self.state.curr_tab_index].cwd.clone();
                self.state.tabs[self.state.curr_tab_index].history_backward.push(old_cwd);
                self.state.tabs[self.state.curr_tab_index].cwd = WorkingDirectory {
                    path: selected_item,
                    children: vec![],
                    curr_index: self.state.tabs[self.state.curr_tab_index].cwd.curr_index,
                };
            } else {
                self.run_launch_cmd();
            }
            Home::save_settings(&self.state);
        }

        if self.state.tabs[self.state.curr_tab_index].cwd.children.is_empty() {
            if let Ok(res) = Path::new(&self.state.tabs[self.state.curr_tab_index].cwd.path).read_dir() {
                for entry in res.flatten() {
                    let path = entry.path().to_str().unwrap().to_string();
                    if path.is_empty() {
                        continue;
                    }
                    self.state.tabs[self.state.curr_tab_index].cwd.children.push(DirEntry {
                        is_dir: Path::new(&path).is_dir(),
                        path,
                        ..DirEntry::default()
                    });
                }
            }
        }

        let regions = {
            // render address bar and dir entries
            let mut constraints =
                [Constraint::Length(1) /* tabs bar */, Constraint::Length(3) /* address bar */].to_vec();
            constraints.append(
                &mut self.state.tabs[self.state.curr_tab_index]
                    .cwd
                    .children
                    .iter()
                    .map(|_| Constraint::Length(1))
                    .collect::<Vec<Constraint>>(),
            );

            let regions = Layout::default().direction(Direction::Vertical).constraints(constraints).split(area);

            self.state.tabs[self.state.curr_tab_index].cwd.curr_index = min(
                self.state.tabs[self.state.curr_tab_index].cwd.curr_index,
                self.state.tabs[self.state.curr_tab_index].cwd.children.len() - 1,
            );

            let address_bar = Paragraph::new(self.state.tabs[self.state.curr_tab_index].cwd.path.as_str()).block(
                Block::default()
                    .title("Current location")
                    .border_style(Style::new().light_magenta())
                    .borders(Borders::TOP | Borders::BOTTOM),
            );
            f.render_widget(address_bar, regions[UI_REGION_ADDRESS_BAR]);
            regions
        };

        {
            // Render tabs
            let ui_region_tabs = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    (0..num_tabs)
                        .map(|_| Constraint::Length(UI_TAB_WIDTH - UI_SPACE_BETWEEN_TABS))
                        .collect::<Vec<Constraint>>(),
                )
                .split(regions[UI_REGION_TABS_BAR]);

            for i in 0..self.state.tabs.len() {
                let tab_path = &self.state.tabs[i].cwd.path;
                let mut block = Block::default()
                    .title(Path::new(Path::new(&tab_path)).components().last().unwrap().as_os_str().to_str().unwrap())
                    .title_alignment(Alignment::Center);

                if i == self.state.curr_tab_index {
                    block = block.set_style(Style::new().bg(Color::LightMagenta));
                }

                let ui_tab = Paragraph::new("").block(block);

                f.render_widget(ui_tab, ui_region_tabs[i]);
            }
        }

        for i in 0..self.state.tabs[self.state.curr_tab_index].cwd.children.len() {
            let dir_entry = &self.state.tabs[self.state.curr_tab_index].cwd.children[i];
            if let Some(file_name) = Path::new(&dir_entry.path.clone()).file_name() {
                if let Some(file_name) = file_name.to_str() {
                    let mut dir_entry_text = file_name.to_string();
                    if dir_entry.is_dir {
                        dir_entry_text = "📂 ".to_owned() + &dir_entry_text;
                    } else {
                        dir_entry_text = get_dir_entry_icon(&dir_entry_text) + &dir_entry_text;
                    }

                    let mut paragraph = Paragraph::new(dir_entry_text.as_str());
                    if i == self.state.tabs[self.state.curr_tab_index].cwd.curr_index {
                        paragraph = paragraph.set_style(Style::new().bg(Color::LightMagenta));
                    }
                    f.render_widget(paragraph, regions[i + UI_REGION_DIR_ENTRIES]);
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
