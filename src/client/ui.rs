use std::collections::VecDeque;
use std::io;

use tui::backend::Backend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::widgets::{Block, Borders, List, Text, Widget};
use tui::{Frame, Terminal};

use crate::{logger, Lock};

pub struct Ui {
    logs: VecDeque<String>,
}

impl Ui {
    pub fn new() -> Self {
        Ui {
            logs: VecDeque::with_capacity(10),
        }
    }

    pub fn draw<B: Backend, R>(
        &mut self,
        terminal: &mut Terminal<B>,
        lock: &Lock<R>,
    ) -> io::Result<()> {
        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Max(20), Constraint::Percentage(50)])
                .split(f.size());
            self.draw_shards(&mut f, chunks[0], lock);
            self.draw_logs(&mut f, chunks[1]);
        })
    }

    fn draw_shards<B: Backend, R>(&mut self, frame: &mut Frame<B>, area: Rect, lock: &Lock<R>) {
        let block = Block::default().borders(Borders::ALL).title("Shards");
        List::new(lock.dump_shards().map(Text::raw))
            .block(block)
            .render(frame, area)
    }

    fn draw_logs<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let block = Block::default().borders(Borders::ALL).title("Logs");

        self.logs.extend(logger::drain());
        if let Some(len) = self
            .logs
            .len()
            .checked_sub(block.inner(area).height as usize)
        {
            self.logs.drain(..len);
        }

        List::new(self.logs.iter().map(Text::raw))
            .block(block)
            .render(frame, area)
    }
}
