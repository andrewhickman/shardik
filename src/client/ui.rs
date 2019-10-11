use std::io;
use std::collections::VecDeque;

use tui::{Frame, Terminal};
use tui::backend::Backend;
use tui::layout::{Layout, Constraint, Rect};
use tui::widgets::{Widget, Block, Borders, List, Text};

use crate::{Lock, logger};

pub struct Ui {
    logs: VecDeque<String>,
}

impl Ui {
    pub fn new() -> Self {
        Ui {
            logs: VecDeque::with_capacity(10),
        }
    }

    pub fn draw<B: Backend, R>(&mut self, terminal: &mut Terminal<B>, lock: &Lock<R>) -> io::Result<()> {
        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .constraints(
                    [
                        Constraint::Min(10),
                        Constraint::Length(10),
                    ]
                    .as_ref(),
                )
                .split(f.size());
            self.draw_shards(&mut f, chunks[0]);
            self.draw_logs(&mut f, chunks[1]);
        })
    }

    fn draw_shards<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        Block::default()
            .borders(Borders::ALL)
            .title("Shards")
            .render(frame, area)
    }

    fn draw_logs<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let block = Block::default().borders(Borders::ALL).title("Logs");

        self.logs.extend(logger::drain());
        if let Some(len) = self.logs.len().checked_sub(block.inner(area).height as usize) {
            self.logs.drain(..len);
        }

        List::new(self.logs.iter().map(Text::raw))
            .block(block)
            .render(frame, area)
    }
}