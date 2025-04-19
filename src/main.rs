use std::{env, fs::{self, OpenOptions}, io::Read, process::{self, exit}};
use app::state::App;

mod app;
mod ui;
mod utils;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: todd <file.json>");
        exit(1);
    }

    let file_path = args[1].clone();
    let mut file = match OpenOptions::new()
        .read(true)       // Allow reading
        .write(true)      // Allow writing
        .open(&file_path) {
            Ok(val) => val,
            Err(err) => {
                eprintln!("Failed to open file: {}", err);
                exit(1);
            },
        };
    let mut file_content = String::new();
    file.read_to_string(&mut file_content)?;

    if file_content.is_empty() {
        println!("File is empty.");
        process::exit(0);
    }
    
    let file_metadata = fs::metadata(file_path)?;
    
    let terminal = ratatui::init();

    let app = App::new(
        &file_content, 
        Some(file_metadata), 
        Some(&mut file),
        terminal.size().unwrap().clone(),
    )?;

    let app_result = app.run(terminal);
    
    ratatui::restore();
    
    return app_result;
}