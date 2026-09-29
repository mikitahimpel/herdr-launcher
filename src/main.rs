mod config;
mod ensure;
mod herdr;

use std::io;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::{Alignment, Position, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use serde_json::Value;

use config::Button;

const ACCENT: Color = Color::Rgb(203, 166, 247);
const RED: Color = Color::Rgb(243, 139, 168);
const TILE: Color = Color::Rgb(40, 42, 58);
const HOVER: Color = Color::Rgb(69, 71, 90);
const PRESSED: Color = Color::Rgb(88, 91, 112);
const TILE_WIDTH: u16 = 7;
const TILE_HEIGHT: u16 = 3;
const TAB_CHECK: Duration = Duration::from_secs(3);
const FLASH: Duration = Duration::from_millis(400);

struct Dock {
    pane_id: Option<String>,
    buttons: Vec<Button>,
    config_error: Option<String>,
    agent_pane: Option<String>,
    hover: Option<Position>,
    flash: Option<(usize, bool, Instant)>,
}

impl Dock {
    fn reload(&mut self) {
        match config::load() {
            Ok(buttons) => {
                self.buttons = buttons;
                self.config_error = None;
            }
            Err(error) => self.config_error = Some(error),
        }
    }

    /// Returns false when the dock is the last pane left in its tab.
    fn refresh_tab(&mut self) -> bool {
        let Some(pane_id) = &self.pane_id else { return true };
        let Some(neighbors) = herdr::tab_neighbors(pane_id) else { return true };
        if neighbors.is_empty() {
            return false;
        }
        let agents: Vec<&Value> = neighbors.iter().filter(|p| !herdr::field(p, "agent").is_empty()).collect();
        self.agent_pane = agents
            .iter()
            .find(|p| p.get("focused").and_then(Value::as_bool) == Some(true))
            .or_else(|| agents.first())
            .map(|p| herdr::field(p, "pane_id").to_string());
        true
    }

    fn fill(&self, value: &str) -> String {
        value.replace("{agent_pane}", self.agent_pane.as_deref().unwrap_or(""))
    }

    fn launch(&self, button: &Button) -> bool {
        if let (Some(plugin), Some(entrypoint)) = (&button.plugin, &button.entrypoint) {
            let mut args = vec!["plugin", "pane", "open", "--plugin", plugin, "--entrypoint", entrypoint];
            for (flag, value) in [("--placement", &button.placement), ("--width", &button.width), ("--height", &button.height)] {
                if let Some(value) = value {
                    args.extend([flag, value.as_str()]);
                }
            }
            let env: Vec<String> = button
                .env
                .iter()
                .map(|(key, value)| format!("{key}={}", self.fill(value)))
                .filter(|pair| !pair.ends_with('='))
                .collect();
            for pair in &env {
                args.extend(["--env", pair.as_str()]);
            }
            return Command::new(herdr::bin()).args(&args).output().is_ok_and(|o| o.status.success());
        }
        let Some((program, rest)) = button.command.as_ref().and_then(|c| c.split_first()) else { return false };
        Command::new(self.fill(program))
            .args(rest.iter().map(|arg| self.fill(arg)))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok()
    }

    fn button_area(&self, area: Rect, index: usize) -> Rect {
        let width = TILE_WIDTH.min(area.width);
        Rect {
            x: area.x + (area.width - width) / 2,
            y: area.y + 1 + index as u16 * (TILE_HEIGHT + 1),
            width,
            height: TILE_HEIGHT,
        }
    }

    fn button_at(&self, area: Rect, position: Position) -> Option<usize> {
        (0..self.buttons.len()).find(|&i| self.button_area(area, i).contains(position))
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        for (index, button) in self.buttons.iter().enumerate() {
            let rect = self.button_area(area, index);
            if rect.bottom() > area.bottom() {
                break;
            }
            let flash = self.flash.filter(|(i, _, at)| *i == index && at.elapsed() < FLASH);
            let hovered = self.hover.is_some_and(|p| rect.contains(p));
            let color = button.color.as_deref().and_then(parse_color).unwrap_or(ACCENT);
            let (background, icon) = match flash {
                Some((_, false, _)) => (PRESSED, RED),
                Some(_) => (PRESSED, color),
                None if hovered => (HOVER, color),
                None => (TILE, color),
            };
            let blank = Line::from("");
            frame.render_widget(
                Paragraph::new(vec![blank.clone(), Line::from(button.icon.clone()).fg(icon).bold(), blank])
                    .alignment(Alignment::Center)
                    .style(Style::new().bg(background)),
                rect,
            );
        }
        if self.config_error.is_some() {
            let rect = Rect { y: area.bottom().saturating_sub(1), height: 1, ..area };
            frame.render_widget(Paragraph::new(Line::from("!").fg(RED)).alignment(Alignment::Center), rect);
        }
    }
}

fn parse_color(hex: &str) -> Option<Color> {
    let hex = hex.strip_prefix('#')?;
    let value = u32::from_str_radix(hex, 16).ok().filter(|_| hex.len() == 6)?;
    Some(Color::Rgb((value >> 16) as u8, (value >> 8) as u8, value as u8))
}

fn run_dock() -> io::Result<()> {
    let mut dock = Dock {
        pane_id: std::env::var("HERDR_PANE_ID").ok().filter(|p| !p.is_empty()),
        buttons: Vec::new(),
        config_error: None,
        agent_pane: None,
        hover: None,
        flash: None,
    };
    if let Some(pane) = &dock.pane_id {
        herdr::call(&["pane", "rename", pane, "--clear"]);
    }
    dock.reload();
    if !dock.refresh_tab() {
        return Ok(());
    }

    let mut terminal = ratatui::init();
    execute!(io::stdout(), EnableMouseCapture)?;
    let mut checked = Instant::now();
    let result = (|| -> io::Result<()> {
        loop {
            terminal.draw(|frame| dock.draw(frame))?;
            if checked.elapsed() >= TAB_CHECK {
                checked = Instant::now();
                dock.reload();
                if !dock.refresh_tab() {
                    return Ok(());
                }
            }
            if !event::poll(Duration::from_millis(100))? {
                continue;
            }
            let area = terminal.get_frame().area();
            match event::read()? {
                Event::Mouse(mouse) => {
                    let position = Position { x: mouse.column, y: mouse.row };
                    match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            if let Some(index) = dock.button_at(area, position) {
                                dock.refresh_tab();
                                let ok = dock.launch(&dock.buttons[index].clone());
                                dock.flash = Some((index, ok, Instant::now()));
                            }
                        }
                        MouseEventKind::Moved => dock.hover = Some(position),
                        _ => {}
                    }
                }
                Event::Key(key) if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(());
                }
                _ => {}
            }
        }
    })();
    let _ = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result
}

fn main() -> io::Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("ensure") => std::process::exit(ensure::run()),
        Some("fit") => std::process::exit(ensure::fit_all()),
        _ => run_dock(),
    }
}
