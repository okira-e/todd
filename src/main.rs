use std::{env, fs::File, io::Read, process::{self, exit}};
use app::state::App;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod app;
mod ui;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    
    // Create logs directory if it doesn't exist
    std::fs::create_dir_all("logs")?;
    
    // Setup simple blocking file logger
    let file_appender = tracing_appender::rolling::hourly("logs", "app.log");
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))  // Log everything at info level and above
        .with_writer(file_appender)               // Write directly to the file
        .with_ansi(false)
        .init();
    
    info!("Application starting");

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: todd <file.json>");
        exit(1);
    }

    let mut file = match File::open(args[1].clone()) {
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
    
    let terminal = ratatui::init();

    let app = App::new(&file_content)?;
    let app_result = app.run(terminal);
    
    ratatui::restore();
    info!("Application exiting");
    
    return app_result;
}