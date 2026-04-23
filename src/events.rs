use std::time::Duration;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use tokio::sync::mpsc;

use crate::app::{Task, TaskList};

#[derive(Debug)]
pub enum Event {
    Tick,
    Key(KeyEvent),
    #[allow(dead_code)]
    Mouse(MouseEvent),
    #[allow(dead_code)]
    Resize(u16, u16),
    ApiUpdate(Vec<Task>),
    ListsUpdate(Vec<TaskList>),
    NeedsAuth(String),
    Sync,
}

pub struct EventHandler {
    sender: mpsc::UnboundedSender<Event>,
    receiver: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let _sender = sender.clone();

        tokio::spawn(async move {
            let mut last_tick = std::time::Instant::now();
            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or(Duration::from_secs(0));

                if event::poll(timeout).expect("failed to poll events") {
                    match event::read().expect("failed to read event") {
                        CrosstermEvent::Key(key) => {
                            if key.kind == event::KeyEventKind::Press {
                                let _ = _sender.send(Event::Key(key));
                            }
                        }
                        CrosstermEvent::Mouse(mouse) => {
                            let _ = _sender.send(Event::Mouse(mouse));
                        }
                        CrosstermEvent::Resize(w, h) => {
                            let _ = _sender.send(Event::Resize(w, h));
                        }
                        _ => {}
                    }
                }

                if last_tick.elapsed() >= tick_rate {
                    let _ = _sender.send(Event::Tick);
                    last_tick = std::time::Instant::now();
                }
            }
        });

        Self { sender, receiver }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.receiver.recv().await
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self.sender.clone()
    }
}
