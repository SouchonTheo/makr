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
    /// Values as parsed from the Makefile, parallel to `variables`. Only
    /// variables whose value differs from this are passed to make, so
    /// untouched ones keep make's own semantics (env vars, built-ins like
    /// `CC`, conditionals, `+=`).
    pub original_values: Vec<String>,
    pub selected: usize,
    pub cursor_pos: usize,
    pub dry_run: bool,
}

impl PopupState {
    /// Variables the user changed in the editor, as `(name, value)` pairs.
    pub fn overrides(&self) -> Vec<(String, String)> {
        self.variables
            .iter()
            .zip(&self.original_values)
            .filter(|((_, value), original)| value != *original)
            .map(|(var, _)| var.clone())
            .collect()
    }

    pub fn run_request(&self) -> RunRequest {
        RunRequest {
            target_name: self.target_name.clone(),
            overrides: self.overrides(),
            dry_run: self.dry_run,
        }
    }
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
    pub fn to_command_parts(&self, make_program: &str, makefile_path: &str) -> Vec<String> {
        let mut parts = vec![
            make_program.to_string(),
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

#[derive(Clone, Copy)]
pub(crate) enum SortMode {
    Default,
    AlphaAsc,
    AlphaDesc,
}

impl SortMode {
    fn next(self) -> Self {
        match self {
            Self::Default => Self::AlphaAsc,
            Self::AlphaAsc => Self::AlphaDesc,
            Self::AlphaDesc => Self::Default,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::AlphaAsc => "a-z",
            Self::AlphaDesc => "z-a",
        }
    }
}

pub struct App {
    pub(crate) variables: Vec<MakeVariable>,
    pub(crate) targets: Vec<MakeTarget>,
    pub(crate) list_state: ListState,
    pub(crate) mode: Mode,
    pub(crate) should_quit: bool,
    pub(crate) run_target: Option<RunRequest>,
    pub(crate) makefile_path: String,
    pub(crate) make_program: String,
    pub(crate) detail_scroll: u16,
    pub(crate) dry_run: bool,
    pub(crate) last_result: Option<String>,
    /// Forces a `terminal.clear()` before the next draw. Set when leaving an
    /// overlay mode (popup) or returning from an external command, to wipe
    /// any stale cells / cursor state we might inherit from the prior screen.
    pub(crate) needs_clear: bool,
    pub(crate) sort_mode: SortMode,
    /// Maps display positions to indices into `targets`. Drives the normal-mode
    /// list rendering and selection — search has its own filtered_indices.
    pub(crate) display_order: Vec<usize>,
}

impl App {
    pub fn new(
        variables: Vec<MakeVariable>,
        targets: Vec<MakeTarget>,
        makefile_path: String,
        make_program: String,
        dry_run: bool,
    ) -> Self {
        let mut list_state = ListState::default();
        if !targets.is_empty() {
            list_state.select(Some(0));
        }
        let display_order: Vec<usize> = (0..targets.len()).collect();
        Self {
            variables,
            targets,
            list_state,
            mode: Mode::Normal,
            should_quit: false,
            run_target: None,
            makefile_path,
            make_program,
            detail_scroll: 0,
            dry_run,
            last_result: None,
            needs_clear: false,
            sort_mode: SortMode::Default,
            display_order,
        }
    }

    fn apply_sort(&mut self) {
        let mut order: Vec<usize> = (0..self.targets.len()).collect();
        match self.sort_mode {
            SortMode::Default => {}
            SortMode::AlphaAsc => {
                order.sort_by(|&a, &b| self.targets[a].name.cmp(&self.targets[b].name));
            }
            SortMode::AlphaDesc => {
                order.sort_by(|&a, &b| self.targets[b].name.cmp(&self.targets[a].name));
            }
        }
        self.display_order = order;
    }

    pub(crate) fn cycle_sort(&mut self) {
        // Preserve which target the user has highlighted across the re-sort.
        let current_target = self.selected_target_index();
        self.sort_mode = self.sort_mode.next();
        self.apply_sort();
        if let Some(target_idx) = current_target {
            self.select_target(target_idx);
        }
        self.detail_scroll = 0;
    }

    /// Highlight the given target (an index into `targets`) in the normal-mode
    /// list, translating it to its position in the current sort order.
    pub(crate) fn select_target(&mut self, target_idx: usize) {
        if let Some(pos) = self.display_order.iter().position(|&i| i == target_idx) {
            self.list_state.select(Some(pos));
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
        let parts = request.to_command_parts(&self.make_program, &self.makefile_path);
        let target_name = request.target_name.clone();
        let dry_run = request.dry_run;

        // Build Command from parts: parts[0] is the make program, rest are args
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
        let cursor_pos = variables.first().map_or(0, |(_, v)| v.len());
        let original_values = variables.iter().map(|(_, v)| v.clone()).collect();
        self.mode = Mode::Popup(PopupState {
            target_name: target.name.clone(),
            variables,
            original_values,
            selected: 0,
            cursor_pos,
            dry_run: self.dry_run,
        });
    }

    pub(crate) fn open_popup(&mut self) {
        if let Some(idx) = self.selected_target_index() {
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
            _ => self
                .list_state
                .selected()
                .and_then(|i| self.display_order.get(i).copied()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::makefile::AssignmentKind;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn target(name: &str, commands: &[&str]) -> MakeTarget {
        MakeTarget {
            name: name.to_string(),
            dependencies: Vec::new(),
            commands: commands.iter().map(|c| (*c).to_string()).collect(),
            is_phony: true,
            description: None,
            variables: Vec::new(),
        }
    }

    fn var(name: &str, value: &str) -> MakeVariable {
        MakeVariable {
            name: name.to_string(),
            value: value.to_string(),
            assignment: AssignmentKind::Recursive,
        }
    }

    fn press(app: &mut App, code: KeyCode) {
        app.handle_key_event(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn type_str(app: &mut App, s: &str) {
        for c in s.chars() {
            press(app, KeyCode::Char(c));
        }
    }

    fn popup(app: &App) -> &PopupState {
        match &app.mode {
            Mode::Popup(p) => p,
            _ => panic!("expected popup mode"),
        }
    }

    #[test]
    fn unedited_variables_are_not_overridden() {
        // CC is undefined in the Makefile (make's built-in), OUT is defined.
        let mut app = App::new(
            vec![var("OUT", "build")],
            vec![target("all", &["$(CC) -o $(OUT)/app main.c"])],
            "Makefile".into(),
            "make".into(),
            false,
        );
        press(&mut app, KeyCode::Enter);
        assert_eq!(popup(&app).variables.len(), 2);
        assert!(popup(&app).overrides().is_empty());

        let parts = popup(&app)
            .run_request()
            .to_command_parts("make", "Makefile");
        assert_eq!(parts, ["make", "-f", "Makefile", "all"]);
    }

    #[test]
    fn edited_variable_is_overridden() {
        let mut app = App::new(
            vec![var("OUT", "build")],
            vec![target("all", &["$(CC) -o $(OUT)/app main.c"])],
            "Makefile".into(),
            "make".into(),
            false,
        );
        press(&mut app, KeyCode::Enter);
        // Variables are listed in reference order: CC first, then OUT.
        press(&mut app, KeyCode::Tab);
        type_str(&mut app, "2");
        assert_eq!(
            popup(&app).overrides(),
            vec![("OUT".to_string(), "build2".to_string())]
        );

        press(&mut app, KeyCode::Enter);
        let req = app.run_target.as_ref().expect("run requested");
        assert_eq!(
            req.to_command_parts("make", "Makefile"),
            ["make", "-f", "Makefile", "OUT=build2", "all"]
        );
    }

    #[test]
    fn search_selection_respects_sort_order() {
        let mut app = App::new(
            Vec::new(),
            vec![
                target("zeta", &[]),
                target("alpha", &[]),
                target("mid", &[]),
            ],
            "Makefile".into(),
            "make".into(),
            false,
        );
        press(&mut app, KeyCode::Char('s')); // a-z: alpha, mid, zeta
        press(&mut app, KeyCode::Char('/'));
        type_str(&mut app, "zeta");
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Esc); // close popup, back to normal mode

        let idx = app.selected_target_index().unwrap();
        assert_eq!(app.targets[idx].name, "zeta");
    }

    #[test]
    fn ctrl_chords_are_not_typed_into_inputs() {
        let mut app = App::new(
            vec![var("OUT", "build")],
            vec![target("all", &["echo $(OUT)"])],
            "Makefile".into(),
            "make".into(),
            false,
        );
        press(&mut app, KeyCode::Enter);
        app.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
        assert_eq!(popup(&app).variables[0].1, "build");

        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Char('/'));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
        match &app.mode {
            Mode::Search(s) => assert!(s.query.is_empty()),
            _ => panic!("expected search mode"),
        }
    }
}
