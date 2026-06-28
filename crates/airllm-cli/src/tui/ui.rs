use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::app::App;

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(3), Constraint::Length(1)].as_ref())
        .split(frame.area());

    let output = Paragraph::new(Text::raw(app.output.clone()))
        .block(Block::default().title("Output").borders(Borders::ALL))
        .style(Style::default().fg(Color::Green));

    let status = Paragraph::new(Text::raw(app.status.clone()))
        .block(Block::default().title("Status").borders(Borders::ALL))
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    frame.render_widget(output, chunks[0]);
    frame.render_widget(status, chunks[1]);
}
