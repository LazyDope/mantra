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
    while !app.run().await? {
        terminal.draw(|frame| app.ui(frame))?;
    }
    Ok(())
}
