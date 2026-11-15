mod app;
mod fuzzy;
mod makefile;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{ArgAction, Parser};

/// Files make looks for when none is given, in make's own order.
const DEFAULT_MAKEFILES: &[&str] = &["GNUmakefile", "makefile", "Makefile"];

const KEYBINDINGS: &str = "\
Keybindings:
  j/k, Up/Down    Navigate targets
  Ctrl+d/Ctrl+u   Scroll detail panel
  /               Fuzzy search
  s               Cycle sort order
  Enter           Run target (with variable editor)
  Ctrl+n (popup)  Toggle dry-run mode
  q, Esc          Quit";

/// Interactive Makefile explorer
#[derive(Parser)]
#[command(
    version,
    disable_version_flag = true,
    after_help = KEYBINDINGS,
    help_template = "makr v{version} -- {about}\n\n{usage-heading} {usage}\n\n{all-args}{after-help}"
)]
struct Cli {
    /// Path to a Makefile [default: GNUmakefile, makefile or Makefile]
    #[arg(value_name = "FILE", conflicts_with = "file_flag")]
    file: Option<PathBuf>,

    /// Path to a Makefile (same as FILE, like `make -f`)
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    file_flag: Option<PathBuf>,

    /// Change to DIR before reading the Makefile and running targets
    #[arg(short = 'C', long, value_name = "DIR")]
    directory: Option<PathBuf>,

    /// Start in dry-run mode (pass -n to make)
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Make program to run, e.g. `gmake`
    #[arg(long = "make", value_name = "PROGRAM", default_value = "make")]
    make_program: String,

    /// Show version
    #[arg(short = 'v', long, action = ArgAction::Version)]
    version: Option<bool>,
}

fn find_default_makefile() -> Option<PathBuf> {
    DEFAULT_MAKEFILES
        .iter()
        .map(PathBuf::from)
        .find(|p| p.is_file())
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Some(dir) = &cli.directory
        && let Err(e) = std::env::set_current_dir(dir)
    {
        eprintln!("Cannot change to directory '{}': {e}", dir.display());
        return ExitCode::FAILURE;
    }

    let Some(path) = cli.file.or(cli.file_flag).or_else(find_default_makefile) else {
        eprintln!(
            "No makefile found (looked for {})",
            DEFAULT_MAKEFILES.join(", ")
        );
        return ExitCode::FAILURE;
    };
    if !path.exists() {
        eprintln!("Cannot find '{}'", path.display());
        return ExitCode::FAILURE;
    }

    let (variables, targets) = match makefile::parse_makefile(Path::new(&path)) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    if targets.is_empty() {
        eprintln!("No targets found in '{}'", path.display());
        return ExitCode::FAILURE;
    }

    let mut terminal = ratatui::init();
    let result = app::App::new(
        variables,
        targets,
        path.display().to_string(),
        cli.make_program,
        cli.dry_run,
    )
    .run(&mut terminal);
    ratatui::restore();
    if let Err(e) = result {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_definition_is_valid() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }

    #[test]
    fn file_can_be_positional_or_flag() {
        let cli = Cli::try_parse_from(["makr", "a.mk"]).unwrap();
        assert_eq!(cli.file, Some(PathBuf::from("a.mk")));
        let cli = Cli::try_parse_from(["makr", "-f", "b.mk", "-n"]).unwrap();
        assert_eq!(cli.file_flag, Some(PathBuf::from("b.mk")));
        assert!(cli.dry_run);
        assert!(Cli::try_parse_from(["makr", "a.mk", "-f", "b.mk"]).is_err());
    }

    #[test]
    fn unknown_flags_are_rejected() {
        assert!(Cli::try_parse_from(["makr", "--dryrun"]).is_err());
    }
}
