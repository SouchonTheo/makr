mod app;
mod fuzzy;
mod makefile;

use std::env;
use std::path::Path;
use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_help() {
    println!("makr v{} -- interactive Makefile explorer", VERSION);
    println!();
    println!("Usage: makr [OPTIONS] [FILE]");
    println!();
    println!("Arguments:");
    println!("  [FILE]  Path to a Makefile (default: ./Makefile)");
    println!();
    println!("Options:");
    println!("  -n, --dry-run  Start in dry-run mode (pass -n to make)");
    println!("  -h, --help     Show this help message");
    println!("  -v, --version  Show version");
    println!();
    println!("Keybindings:");
    println!("  j/k, Up/Down    Navigate targets");
    println!("  Ctrl+d/Ctrl+u   Scroll detail panel");
    println!("  /               Fuzzy search");
    println!("  Enter           Run target (with variable editor)");
    println!("  Ctrl+n (popup)  Toggle dry-run mode");
    println!("  q, Esc          Quit");
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "-v" || a == "--version") {
        println!("makr v{}", VERSION);
        return ExitCode::SUCCESS;
    }

    let dry_run = args.iter().any(|a| a == "-n" || a == "--dry-run");

    let path = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .cloned()
        .unwrap_or_else(|| "Makefile".to_string());

    if !Path::new(&path).exists() {
        eprintln!("Cannot find '{}'", path);
        return ExitCode::FAILURE;
    }

    let (variables, targets) = match makefile::parse_makefile(Path::new(&path)) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    };
    if targets.is_empty() {
        eprintln!("No targets found in '{}'", path);
        return ExitCode::FAILURE;
    }

    let mut terminal = ratatui::init();
    let result = app::App::new(variables, targets, path, dry_run).run(&mut terminal);
    ratatui::restore();
    if let Err(e) = result {
        eprintln!("{}", e);
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
