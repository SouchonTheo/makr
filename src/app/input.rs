use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{App, Mode, SearchState};

impl App {
    pub(crate) fn handle_key_event(&mut self, key: KeyEvent) {
        match &self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::Popup(_) => self.handle_popup_key(key),
            Mode::Search(_) => self.handle_search_key(key.code),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::Enter => self.open_popup(),
            KeyCode::Char('/') => self.open_search(),
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.detail_scroll = self.detail_scroll.saturating_add(5);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.detail_scroll = self.detail_scroll.saturating_sub(5);
            }
            _ => {}
        }
    }

    fn search_state_mut(&mut self) -> Option<&mut SearchState> {
        match &mut self.mode {
            Mode::Search(s) => Some(s),
            _ => None,
        }
    }

    fn handle_search_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                if let Some(target_idx) = self.selected_target_index() {
                    self.list_state.select(Some(target_idx));
                    self.detail_scroll = 0;
                    self.open_popup_for(target_idx);
                }
            }
            KeyCode::Up => {
                let Some(search) = self.search_state_mut() else {
                    return;
                };
                if !search.filtered_indices.is_empty() {
                    let i = match search.list_state.selected() {
                        Some(0) | None => search.filtered_indices.len() - 1,
                        Some(i) => i - 1,
                    };
                    search.list_state.select(Some(i));
                    self.detail_scroll = 0;
                }
            }
            KeyCode::Down => {
                let Some(search) = self.search_state_mut() else {
                    return;
                };
                if !search.filtered_indices.is_empty() {
                    let i = match search.list_state.selected() {
                        Some(i) if i >= search.filtered_indices.len() - 1 => 0,
                        Some(i) => i + 1,
                        None => 0,
                    };
                    search.list_state.select(Some(i));
                    self.detail_scroll = 0;
                }
            }
            KeyCode::Char(c) => {
                let Some(search) = self.search_state_mut() else {
                    return;
                };
                search.query.insert(search.cursor_pos, c);
                search.cursor_pos += 1;
                self.update_search_filter();
                self.detail_scroll = 0;
            }
            KeyCode::Backspace => {
                let Some(search) = self.search_state_mut() else {
                    return;
                };
                if search.cursor_pos > 0 {
                    search.cursor_pos -= 1;
                    search.query.remove(search.cursor_pos);
                    self.update_search_filter();
                    self.detail_scroll = 0;
                }
            }
            KeyCode::Left => {
                let Some(search) = self.search_state_mut() else {
                    return;
                };
                if search.cursor_pos > 0 {
                    search.cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                let Some(search) = self.search_state_mut() else {
                    return;
                };
                if search.cursor_pos < search.query.len() {
                    search.cursor_pos += 1;
                }
            }
            _ => {}
        }
    }

    fn handle_popup_key(&mut self, key: KeyEvent) {
        // Handle mode-changing keys first (need to reassign self.mode)
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                return;
            }
            KeyCode::Enter => {
                if let Mode::Popup(popup) = &self.mode {
                    let target_name = popup.target_name.clone();
                    let overrides = popup.variables.clone();
                    let dry_run = popup.dry_run;
                    self.run_target = Some(super::RunRequest {
                        target_name,
                        overrides,
                        dry_run,
                    });
                }
                self.mode = Mode::Normal;
                return;
            }
            _ => {}
        }

        // Handle mutation keys
        let popup = match &mut self.mode {
            Mode::Popup(p) => p,
            _ => return,
        };
        match key.code {
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                popup.dry_run = !popup.dry_run;
            }
            KeyCode::Tab | KeyCode::Down => {
                if !popup.variables.is_empty() {
                    popup.selected = (popup.selected + 1) % popup.variables.len();
                    popup.cursor_pos = popup.variables[popup.selected].1.len();
                }
            }
            KeyCode::BackTab | KeyCode::Up => {
                if !popup.variables.is_empty() {
                    popup.selected = if popup.selected == 0 {
                        popup.variables.len() - 1
                    } else {
                        popup.selected - 1
                    };
                    popup.cursor_pos = popup.variables[popup.selected].1.len();
                }
            }
            KeyCode::Char(c) => {
                if !popup.variables.is_empty() {
                    let value = &mut popup.variables[popup.selected].1;
                    value.insert(popup.cursor_pos, c);
                    popup.cursor_pos += 1;
                }
            }
            KeyCode::Backspace => {
                if !popup.variables.is_empty() && popup.cursor_pos > 0 {
                    let value = &mut popup.variables[popup.selected].1;
                    popup.cursor_pos -= 1;
                    value.remove(popup.cursor_pos);
                }
            }
            KeyCode::Left => {
                if popup.cursor_pos > 0 {
                    popup.cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                if !popup.variables.is_empty() {
                    let len = popup.variables[popup.selected].1.len();
                    if popup.cursor_pos < len {
                        popup.cursor_pos += 1;
                    }
                }
            }
            _ => {}
        }
    }
}
