use std::io;

use crossterm::{
    terminal::{self, EnterAlternateScreen},
    ExecutableCommand,
};
use mantra::App;
use ratatui::{backend::CrosstermBackend, Terminal};

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    terminal::enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let app = App::init().await?;
    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|frame| app.ui(frame))?;
        should_quit = app.run().await?;
    }
    Ok(())
}
