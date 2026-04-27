mod input;
mod ui;

use std::io::{self, Write};
use std::process::Command;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{DefaultTerminal, widgets::ListState};

use crate::makefile::{MakeTarget, MakeVariable, find_all_used_variables};

pub(crate) struct PopupState {
    pub target_name: String,
    pub variables: Vec<(String, String)>,
    pub selected: usize,
    pub cursor_pos: usize,
    pub dry_run: bool,
}

pub(crate) struct SearchState {
    pub query: String,
    pub cursor_pos: usize,
    pub filtered_indices: Vec<usize>,
    pub list_state: ListState,
}

pub(crate) struct RunRequest {
    pub target_name: String,
    pub overrides: Vec<(String, String)>,
    pub dry_run: bool,
}

impl RunRequest {
    pub fn to_command_parts(&self, makefile_path: &str) -> Vec<String> {
        let mut parts = vec![
            "make".to_string(),
            "-f".to_string(),
            makefile_path.to_string(),
        ];
        if self.dry_run {
            parts.push("-n".to_string());
        }
        for (n, v) in &self.overrides {
            parts.push(format!("{}={}", n, v));
        }
        parts.push(self.target_name.clone());
        parts
    }
}

pub(crate) enum Mode {
    Normal,
    Popup(PopupState),
    Search(SearchState),
}

pub struct App {
    pub(crate) variables: Vec<MakeVariable>,
    pub(crate) targets: Vec<MakeTarget>,
    pub(crate) list_state: ListState,
    pub(crate) mode: Mode,
    pub(crate) should_quit: bool,
    pub(crate) run_target: Option<RunRequest>,
    pub(crate) makefile_path: String,
    pub(crate) detail_scroll: u16,
    pub(crate) dry_run: bool,
    pub(crate) last_result: Option<String>,
    /// Forces a `terminal.clear()` before the next draw. Set when leaving an
    /// overlay mode (popup) or returning from an external command, to wipe
    /// any stale cells / cursor state we might inherit from the prior screen.
    pub(crate) needs_clear: bool,
}

impl App {
    pub fn new(
        variables: Vec<MakeVariable>,
        targets: Vec<MakeTarget>,
        makefile_path: String,
        dry_run: bool,
    ) -> Self {
        let mut list_state = ListState::default();
        if !targets.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            variables,
            targets,
            list_state,
            mode: Mode::Normal,
            should_quit: false,
            run_target: None,
            makefile_path,
            detail_scroll: 0,
            dry_run,
            last_result: None,
            needs_clear: false,
        }
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            if self.needs_clear {
                terminal.clear()?;
                self.needs_clear = false;
            }
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;

            if self.should_quit {
                break;
            }

            if let Some(request) = self.run_target.take() {
                self.run_make(request, terminal)?;
            }
        }
        Ok(())
    }

    fn run_make(&mut self, request: RunRequest, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let parts = request.to_command_parts(&self.makefile_path);
        let target_name = request.target_name.clone();
        let dry_run = request.dry_run;

        // Build Command from parts: parts[0] is "make", rest are args
        let mut cmd = Command::new(&parts[0]);
        for arg in &parts[1..] {
            cmd.arg(arg);
        }

        ratatui::restore();

        let prefix = if dry_run { " [DRY RUN]" } else { "" };
        println!("\n--- Running{}: {} ---\n", prefix, parts.join(" "));

        let status = cmd.status();
        let result_msg = match &status {
            Ok(s) => {
                println!("\n--- make exited with {} ---", s);
                if s.success() {
                    format!("{}: OK", target_name)
                } else {
                    format!("{}: exit {}", target_name, s.code().unwrap_or(-1))
                }
            }
            Err(e) => {
                println!("\n--- Failed to run make: {} ---", e);
                format!("{}: error: {}", target_name, e)
            }
        };
        self.last_result = Some(result_msg);

        print!("Press Enter to return to the TUI...");
        io::stdout().flush().ok();
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).ok();

        *terminal = ratatui::init();
        // External program may have left the alt-screen / cursor in any
        // state — force a full redraw on the next loop tick.
        self.needs_clear = true;
        Ok(())
    }

    pub(crate) fn open_popup_for(&mut self, target_idx: usize) {
        let target = &self.targets[target_idx];
        let used = find_all_used_variables(target, &self.variables);
        let variables: Vec<(String, String)> =
            used.into_iter().map(|v| (v.name, v.value)).collect();
        let cursor_pos = variables.first().map(|(_, v)| v.len()).unwrap_or(0);
        self.mode = Mode::Popup(PopupState {
            target_name: target.name.clone(),
            variables,
            selected: 0,
            cursor_pos,
            dry_run: self.dry_run,
        });
    }

    pub(crate) fn open_popup(&mut self) {
        if let Some(idx) = self.list_state.selected() {
            self.open_popup_for(idx);
        }
    }

    pub(crate) fn open_search(&mut self) {
        let all_indices: Vec<usize> = (0..self.targets.len()).collect();
        let mut list_state = ListState::default();
        if !all_indices.is_empty() {
            list_state.select(Some(0));
        }
        self.mode = Mode::Search(SearchState {
            query: String::new(),
            cursor_pos: 0,
            filtered_indices: all_indices,
            list_state,
        });
    }

    pub(crate) fn update_search_filter(&mut self) {
        use crate::fuzzy::fuzzy_score;

        let search = match &mut self.mode {
            Mode::Search(s) => s,
            _ => return,
        };

        if search.query.is_empty() {
            search.filtered_indices = (0..self.targets.len()).collect();
        } else {
            let mut scored: Vec<(usize, u32)> = self
                .targets
                .iter()
                .enumerate()
                .filter_map(|(i, t)| fuzzy_score(&search.query, &t.name).map(|s| (i, s)))
                .collect();
            scored.sort_by_key(|(_, s)| *s);
            search.filtered_indices = scored.into_iter().map(|(i, _)| i).collect();
        }

        if search.filtered_indices.is_empty() {
            search.list_state.select(None);
        } else {
            search.list_state.select(Some(0));
        }
    }

    pub(crate) fn selected_target_index(&self) -> Option<usize> {
        match &self.mode {
            Mode::Search(search) => search
                .list_state
                .selected()
                .and_then(|i| search.filtered_indices.get(i).copied()),
            _ => self.list_state.selected(),
        }
    }

    pub(crate) fn select_next(&mut self) {
        if self.targets.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) if i >= self.targets.len() - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.list_state.select(Some(i));
        self.detail_scroll = 0;
    }

    pub(crate) fn select_previous(&mut self) {
        if self.targets.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(0) | None => self.targets.len() - 1,
            Some(i) => i - 1,
        };
        self.list_state.select(Some(i));
        self.detail_scroll = 0;
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    return Ok(());
                }
                self.handle_key_event(key);
            }
            Event::Resize(_, _) => {
                // Terminal will redraw on next loop iteration
            }
            _ => {}
        }
        Ok(())
    }
}
