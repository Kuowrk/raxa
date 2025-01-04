pub mod app;
pub mod renderer;

use color_eyre::Result;
use app::App;

fn main() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();

    let mut app = App::new()?;
    app.run()?;

    Ok(())
}
