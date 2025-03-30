use std::{env, fs::File, io::Read, process::{self, exit}};

use app::state::App;

mod app;
mod ui;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: todd <file.json>");
        exit(1);
    }

    let mut file = File::open(args[1].clone())?;
    let mut file_content = String::new();
    file.read_to_string(&mut file_content)?;

    if file_content.is_empty() {
        println!("File is empty.");
        process::exit(0);
    }
    
    let terminal = ratatui::init();

    let app = App::new(&file_content)?;
    let app = app.run(terminal);
    
    ratatui::restore();
    return app;
}
