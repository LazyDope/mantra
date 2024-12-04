use mantra_lancer::app::App;

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    let terminal = ratatui::init();

    let app = App::init().await?;
    let app_result = app.run(terminal).await;

    ratatui::restore();
    Ok(app_result?)
}
