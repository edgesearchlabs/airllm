use std::io::{stdout, Write};

use airllm_orchestrator::Result as OrchResult;
use anyhow::Result;
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use futures::Stream;
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use super::app::App;
use super::ui;

pub async fn render_stream<S>(stream: &mut S) -> Result<()>
where
    S: Stream<Item = OrchResult<String>> + Unpin,
{
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::default();
    app.set_status("streaming...");

    while let Some(next) = stream.next().await {
        match next {
            Ok(chunk) => app.push(&chunk),
            Err(err) => {
                app.set_status(format!("error: {err}"));
                break;
            }
        }
        terminal.draw(|f| ui::draw(f, &app))?;
    }

    app.set_status("done");
    terminal.draw(|f| ui::draw(f, &app))?;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.backend_mut().flush()?;
    Ok(())
}
