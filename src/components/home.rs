use std::{collections::HashMap, path::Path, time::Duration};

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
pub struct Home {
  command_tx: Option<UnboundedSender<Action>>,
  config: Config,
  cwd: String,
  curr_index: Option<usize>,
}

impl Home {
  pub fn new() -> Self {
    Self { cwd: "/".to_string(), ..Self::default() }
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

  fn update(&mut self, action: Action) -> Result<Option<Action>> {
    match action {
      Action::Tick => {},
      Action::Help => {},
      _ => {},
    }
    Ok(None)
  }

  fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
    let mut entries = Vec::new();
    for entry in Path::new(&self.cwd).read_dir().unwrap().flatten() {
      entries.push(entry.path().to_owned());
    }

    self.curr_index = if !entries.is_empty() { Some(0) } else { None };

    let lines = Layout::default()
      .direction(Direction::Vertical)
      .constraints(entries.iter().map(|_| Constraint::Length(1)).collect::<Vec<Constraint>>())
      .split(area);

    for i in 0..entries.len() {
      let is_selected = self.curr_index.is_some() && i == self.curr_index.unwrap();
      let prefix = (if is_selected { "> " } else { "" }).to_owned();
      let widget_text = prefix + entries[i].as_path().to_str().unwrap();
      let mut paragraph = Paragraph::new(widget_text);
      if is_selected {
        paragraph = paragraph.set_style(Style::new().bg(Color::Magenta));
      }
      f.render_widget(paragraph, lines[i]);
    }

    Ok(())
  }
}
