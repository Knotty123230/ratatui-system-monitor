use std::{
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

use color_eyre::{Result, eyre::Ok};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
};
use sysinfo::System;

fn main() -> color_eyre::Result<()> {
    let (event_tx, event_rx) = mpsc::channel::<Event>();
    let tx_to_input_events = event_tx.clone();
    thread::spawn(move || {
        handle_input_events(tx_to_input_events);
    });
    let tx_to_input_events = event_tx.clone();
    thread::spawn(move || {
        handle_key_events(tx_to_input_events);
    });
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().run(terminal, &event_rx);
    ratatui::restore();
    result
}

fn handle_key_events(tx_to_input_events: mpsc::Sender<Event>) {
    if let crossterm::event::Event::Key(key_event) = crossterm::event::read().unwrap() {
        tx_to_input_events.send(Event::Input(key_event)).unwrap()
    }
}

fn handle_input_events(tx_to_input_events: mpsc::Sender<Event>) {
    let mut sys = System::new_all();
    loop {
        sys.refresh_all();
        let free_memory = sys.free_memory();
        if tx_to_input_events.send(Event::Memory(free_memory)).is_err() {
            break;
        }
        thread::sleep(Duration::from_millis(500));
    }
}

pub(crate) enum Event {
    Input(crossterm::event::KeyEvent), // crossterm key input event
    Memory(u64),
}

/// The main application which holds the state and logic of the application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    running: bool,
    latest_mem: Option<u64>,
}

impl App {
    /// Cnstruct a new instance of [`App`].
    pub fn new() -> Self {
        Self {
            running: true,
            latest_mem: None,
        }
    }

    /// Run the application's main loop.
    fn run(mut self, mut terminal: DefaultTerminal, evt: &Receiver<Event>) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame, &evt))?;
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame, evt: &Receiver<Event>) {
        // Process all available events to ensure we don't miss memory updates
        for event in evt.try_iter() {
            match event {
                Event::Memory(mem) => self.latest_mem = Some(mem),
                Event::Input(key_event) => self.on_key_event(key_event),
            }
        }
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(100)])
            .split(frame.area());

        if let Some(mem) = &self.latest_mem {
            self.render_memory(mem, layout[0], frame);
        }
    }

    fn render_memory(&self, mem: impl ToString, area: Rect, frame: &mut Frame) {
        let block = Block::new()
            .title("Memory Info")
            .borders(Borders::ALL)
            .style(Style::default().fg(ratatui::style::Color::Red));

        let paragraph = Paragraph::new(mem.to_string()).block(block);

        frame.render_widget(paragraph, area);
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        if let (_, KeyCode::Esc | KeyCode::Char('q')) = (key.modifiers, key.code) {
            self.quit()
        }
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}
