mod config;
mod herdr;

use std::io;
use std::process::{Command, Stdio};
use std::time::Duration;

use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton,
    MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::{Constraint, Layout, Margin, Position, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

use config::Panel;

const TEXT: Color = Color::Rgb(205, 214, 244);
const MUTED: Color = Color::Rgb(108, 112, 134);
const BORDER: Color = Color::Rgb(69, 71, 90);
const ACCENT: Color = Color::Rgb(203, 166, 247);
const RED: Color = Color::Rgb(243, 139, 168);
const SELECTED: Color = Color::Rgb(49, 50, 68);
const ITEM_HEIGHT: u16 = 3;

struct Menu {
    panels: Vec<Panel>,
    error: Option<String>,
    selected: usize,
    agent_pane: Option<String>,
}

impl Menu {
    fn fill(&self, value: &str) -> String {
        value.replace("{agent_pane}", self.agent_pane.as_deref().unwrap_or(""))
    }

    fn launch(&self, panel: &Panel) -> Result<(), String> {
        if let (Some(plugin), Some(entrypoint)) = (&panel.plugin, &panel.entrypoint) {
            let placement = panel.placement.clone().unwrap_or_else(|| "overlay".to_string());
            let mut args: Vec<String> =
                ["plugin", "pane", "open", "--plugin", plugin, "--entrypoint", entrypoint, "--placement", &placement]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
            for (flag, value) in [("--width", &panel.width), ("--height", &panel.height)] {
                if let Some(value) = value {
                    args.extend([flag.to_string(), value.clone()]);
                }
            }
            for (key, value) in &panel.env {
                let value = self.fill(value);
                if !value.is_empty() {
                    args.extend(["--env".to_string(), format!("{key}={value}")]);
                }
            }
            let output = Command::new(herdr::bin()).args(&args).output().map_err(|e| e.to_string())?;
            if output.status.success() {
                return Ok(());
            }
            let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap_or_default();
            return Err(error
                .pointer("/error/message")
                .and_then(|m| m.as_str())
                .unwrap_or("couldn't open this panel")
                .to_string());
        }
        let Some((program, rest)) = panel.command.as_ref().and_then(|c| c.split_first()) else {
            return Err("panel has no plugin or command".into());
        };
        Command::new(self.fill(program))
            .args(rest.iter().map(|arg| self.fill(arg)))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn areas(area: Rect) -> (Rect, Rect) {
        let inner = Block::bordered().inner(area).inner(Margin::new(1, 1));
        let [list, _, footer] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1), Constraint::Length(1)]).areas(inner);
        (list, footer)
    }

    fn item_area(list: Rect, index: usize) -> Rect {
        Rect { y: list.y + index as u16 * ITEM_HEIGHT, height: ITEM_HEIGHT - 1, ..list }
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        frame.render_widget(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(BORDER))
                .title(Line::from(vec![Span::from(" ◆ ").fg(ACCENT), Span::from("Panels ").fg(TEXT).bold()])),
            area,
        );
        let (list, footer) = Self::areas(area);

        for (index, panel) in self.panels.iter().enumerate() {
            let rect = Self::item_area(list, index);
            if rect.bottom() > list.bottom() {
                break;
            }
            let selected = index == self.selected;
            let color = panel.color.as_deref().and_then(parse_color).unwrap_or(ACCENT);
            let marker = if selected { "▌" } else { " " };
            let number = if index < 9 { (index + 1).to_string() } else { String::new() };
            let name_width = rect.width.saturating_sub(6 + number.len() as u16) as usize;
            let name = format!("{:<name_width$}", panel.name);
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(vec![
                        Span::from(marker).fg(color),
                        Span::from(format!(" {}  ", panel.icon)).fg(color).bold(),
                        Span::from(name).fg(TEXT).bold(),
                        Span::from(number).fg(MUTED),
                    ]),
                    Line::from(vec![Span::from(marker).fg(color), Span::from(format!("    {}", panel.description)).fg(MUTED)]),
                ])
                .style(if selected { Style::new().bg(SELECTED) } else { Style::new() }),
                rect,
            );
        }

        let hint = match &self.error {
            Some(error) => Line::from(error.clone()).fg(RED),
            None => Line::from("↑↓ select · enter open · 1-9 · esc close").fg(MUTED),
        };
        frame.render_widget(Paragraph::new(hint), footer);
    }
}

fn parse_color(hex: &str) -> Option<Color> {
    let hex = hex.strip_prefix('#')?;
    let value = u32::from_str_radix(hex, 16).ok().filter(|_| hex.len() == 6)?;
    Some(Color::Rgb((value >> 16) as u8, (value >> 8) as u8, value as u8))
}

fn run_menu() -> io::Result<()> {
    let (panels, error) = match config::load() {
        Ok(panels) => (panels, None),
        Err(error) => (Vec::new(), Some(format!("launcher.toml: {error}"))),
    };
    let mut menu = Menu { panels, error, selected: 0, agent_pane: herdr::agent_pane() };
    let count = menu.panels.len().max(1);

    let mut terminal = ratatui::init();
    execute!(io::stdout(), EnableMouseCapture)?;
    let result = (|| -> io::Result<()> {
        loop {
            terminal.draw(|frame| menu.draw(frame))?;
            if !event::poll(Duration::from_millis(250))? {
                continue;
            }
            let area = terminal.get_frame().area();
            let chosen = match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(()),
                    KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                        menu.selected = (menu.selected + 1) % count;
                        None
                    }
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => {
                        menu.selected = (menu.selected + count - 1) % count;
                        None
                    }
                    KeyCode::Enter => Some(menu.selected).filter(|&i| i < menu.panels.len()),
                    KeyCode::Char(c @ '1'..='9') => Some(c as usize - '1' as usize).filter(|&i| i < menu.panels.len()),
                    _ => None,
                },
                Event::Mouse(mouse) => {
                    let (list, _) = Menu::areas(area);
                    let position = Position { x: mouse.column, y: mouse.row };
                    let hit = (0..menu.panels.len()).find(|&i| Menu::item_area(list, i).contains(position));
                    match mouse.kind {
                        MouseEventKind::Moved => {
                            if let Some(index) = hit {
                                menu.selected = index;
                            }
                            None
                        }
                        MouseEventKind::Down(MouseButton::Left) => hit,
                        _ => None,
                    }
                }
                _ => None,
            };
            if let Some(index) = chosen {
                match menu.launch(&menu.panels[index].clone()) {
                    Ok(()) => return Ok(()),
                    Err(error) => menu.error = Some(error),
                }
            }
        }
    })();
    let _ = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result
}

fn main() -> io::Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("open") => std::process::exit(herdr::open_menu()),
        _ => run_menu(),
    }
}
