use mantra::app::App;

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    let mut terminal = ratatui::init();

    let mut app = App::init("wrench".into()).await?;
    while app.run().await? {
        terminal.draw(|frame| app.ui(frame))?;
    }

    ratatui::restore();
    Ok(())
}
