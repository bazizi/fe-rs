use std::{cmp::min, collections::HashMap, default, hash::Hash, path::Path, time::Duration, vec};

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use super::{Component, Frame};
use crate::{
  action::Action,
  config::{Config, KeyBindings},
};

#[derive(Default)]
struct DirEntry {
  path: String,
  is_dir: bool,
  size: Option<usize>,
}

#[derive(Default)]
pub struct Home {
  command_tx: Option<UnboundedSender<Action>>,
  config: Config,
  cwd: (String, Vec<Option<DirEntry>>),
  curr_index: Option<usize>,
  selected: bool,
}

impl Home {
  pub fn new() -> Self {
    Self { cwd: ("/".to_string(), vec![]), ..Self::default() }
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
        if self.curr_index.is_some() {
          self.curr_index.replace(self.curr_index.unwrap() + 1);
        }
      },
      KeyCode::Char('k') | KeyCode::Up => {
        if self.curr_index.is_some() && Some(0) != self.curr_index {
          self.curr_index.replace(self.curr_index.unwrap() - 1);
        }
      },
      KeyCode::Enter => self.selected = true,
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
    if self.selected && self.curr_index.is_some() {
      self.cwd = (self.cwd.1[self.curr_index.unwrap()].take().unwrap().path, vec![]);
      self.selected = false;
    }

    if self.cwd.1.is_empty() {
      self.cwd.1.push(Some(DirEntry { path: "..".to_owned(), ..DirEntry::default() }));
      if let Ok(res) = Path::new(&self.cwd.0).read_dir() {
        for entry in res.flatten() {
          let path = entry.path().to_str().unwrap().to_string();
          self.cwd.1.push(Some(DirEntry { is_dir: Path::new(&path).is_dir(), path, ..DirEntry::default() }));
        }
      }
    }

    if self.curr_index.is_none() {
      if !self.cwd.1.is_empty() {
        self.curr_index = Some(0);
      }
    } else {
      self.curr_index = Some(min(*self.curr_index.as_ref().unwrap(), self.cwd.1.len() - 1));
    }

    let lines = Layout::default()
      .direction(Direction::Vertical)
      .constraints(self.cwd.1.iter().map(|_| Constraint::Length(1)).collect::<Vec<Constraint>>())
      .split(area);

    for i in 0..self.cwd.1.len() {
      let dir_entry = self.cwd.1[i].as_ref().unwrap();
      let mut dir_entry_text = dir_entry.path.clone();
      if dir_entry.is_dir {
        dir_entry_text = "📂".to_owned() + &dir_entry_text;
      } else {
        dir_entry_text = "📄".to_owned() + &dir_entry_text;
      }

      let mut paragraph = Paragraph::new(dir_entry_text.as_str());
      let is_selected = self.curr_index.is_some() && Some(i) == self.curr_index;
      if is_selected {
        paragraph = paragraph.set_style(Style::new().bg(Color::Magenta));
      }
      f.render_widget(paragraph, lines[i]);
    }

    Ok(())
  }
}
