use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Clear, List, Paragraph},
};

use super::{App, Mode, PopupState};
use crate::makefile::find_all_used_variables;
use crate::{fuzzy::fuzzy_match, makefile::MakeVariable};

pub(crate) fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

/// Render a target name with fuzzy-matched characters highlighted.
pub(crate) fn styled_fuzzy_name<'a>(name: &'a str, matched_indices: &[usize]) -> Line<'a> {
    let chars: Vec<char> = name.chars().collect();
    let mut spans = Vec::new();
    let mut normal_start = 0;

    for &mi in matched_indices {
        if mi > normal_start {
            let s: String = chars[normal_start..mi].iter().collect();
            spans.push(Span::styled(s, Style::default().fg(Color::Green)));
        }
        let s: String = chars[mi..=mi].iter().collect();
        spans.push(Span::styled(
            s,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
        normal_start = mi + 1;
    }
    if normal_start < chars.len() {
        let s: String = chars[normal_start..].iter().collect();
        spans.push(Span::styled(s, Style::default().fg(Color::Green)));
    }

    Line::from(spans)
}

fn keybind<'a>(key: &'a str, desc: &'a str) -> [Span<'a>; 2] {
    [
        Span::styled(
            key,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc, Style::default().fg(Color::DarkGray)),
    ]
}

impl App {
    pub(crate) fn draw(&mut self, frame: &mut Frame) {
        let [main_area, help_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());

        // Help bar
        let mut help_spans: Vec<Span> = match &self.mode {
            Mode::Search(_) => [
                keybind(" \u{2191}\u{2193}", " navigate  "),
                keybind("Enter", " select  "),
                keybind("Esc", " cancel"),
            ]
            .into_iter()
            .flatten()
            .collect(),
            _ => {
                let mut spans: Vec<Span> = [
                    keybind(" j/k", " navigate  "),
                    keybind("C-d/C-u", " scroll  "),
                    keybind("/", " search  "),
                ]
                .into_iter()
                .flatten()
                .collect();
                spans.push(Span::styled(
                    "s",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    " sort:",
                    Style::default().fg(Color::DarkGray),
                ));
                spans.push(Span::styled(
                    self.sort_mode.label(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled("  ", Style::default().fg(Color::DarkGray)));
                spans.extend(
                    [keybind("Enter", " run  "), keybind("q", " quit")]
                        .into_iter()
                        .flatten(),
                );
                spans
            }
        };

        // Show last execution result in help bar
        if let Some(ref result) = self.last_result {
            help_spans.push(Span::styled("  | ", Style::default().fg(Color::DarkGray)));
            help_spans.push(Span::styled(
                result.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        frame.render_widget(Paragraph::new(Line::from(help_spans)), help_area);

        let [left, right] =
            Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
                .areas(main_area);

        let [detail_area, vars_area] = Layout::new(
            Direction::Vertical,
            [Constraint::Percentage(60), Constraint::Percentage(40)],
        )
        .areas(right);

        // Left panel: target list or search
        match &mut self.mode {
            Mode::Search(search) => {
                let [search_input_area, search_list_area] =
                    Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).areas(left);

                let input_line = Line::from(vec![
                    Span::styled(
                        "/",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(&search.query),
                ]);
                let input = Paragraph::new(input_line)
                    .block(Block::bordered().title(" Search ".fg(Color::Yellow).bold()));
                frame.render_widget(input, search_input_area);

                let cursor_chars = search.query[..search.cursor_pos].chars().count() as u16;
                let cursor_x = search_input_area.x + 1 + 1 + cursor_chars;
                let cursor_y = search_input_area.y + 1;
                frame.set_cursor_position((cursor_x, cursor_y));

                let targets = &self.targets;
                let items: Vec<Line> = search
                    .filtered_indices
                    .iter()
                    .map(|&i| {
                        let target = &targets[i];
                        let name = &target.name;
                        let mut line = if let Some(matched) = fuzzy_match(&search.query, name) {
                            styled_fuzzy_name(name, &matched)
                        } else {
                            Line::styled(name.as_str(), Style::default().fg(Color::Green))
                        };
                        if target.is_phony {
                            line.spans.push(Span::styled(
                                " PHONY",
                                Style::default()
                                    .fg(Color::DarkGray)
                                    .add_modifier(Modifier::ITALIC),
                            ));
                        }
                        line
                    })
                    .collect();
                let list = List::new(items)
                    .block(
                        Block::bordered().title(
                            format!(
                                " {} match{} ",
                                search.filtered_indices.len(),
                                if search.filtered_indices.len() == 1 {
                                    ""
                                } else {
                                    "es"
                                }
                            )
                            .fg(Color::Cyan)
                            .bold(),
                        ),
                    )
                    .highlight_style(
                        Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD),
                    );
                frame.render_stateful_widget(list, search_list_area, &mut search.list_state);
            }
            _ => {
                let items: Vec<Line> = self
                    .display_order
                    .iter()
                    .map(|&i| {
                        let t = &self.targets[i];
                        let mut spans =
                            vec![Span::styled(&t.name, Style::default().fg(Color::Green))];
                        if t.is_phony {
                            spans.push(Span::styled(
                                " PHONY",
                                Style::default()
                                    .fg(Color::DarkGray)
                                    .add_modifier(Modifier::ITALIC),
                            ));
                        }
                        Line::from(spans)
                    })
                    .collect();
                let list = List::new(items)
                    .block(Block::bordered().title(" Targets ".fg(Color::Cyan).bold()))
                    .highlight_style(
                        Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD),
                    );
                frame.render_stateful_widget(list, left, &mut self.list_state);
            }
        }

        // Detail panel
        let selected_idx = self.selected_target_index();
        let detail_lines = if let Some(idx) = selected_idx {
            let target = &self.targets[idx];
            let mut lines: Vec<Line> = Vec::new();

            let mut title_spans = vec![
                Span::styled(
                    "Target: ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&target.name, Style::default().add_modifier(Modifier::BOLD)),
            ];
            if target.is_phony {
                title_spans.push(Span::styled(
                    "  [PHONY]",
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            lines.push(Line::from(title_spans));
            lines.push(Line::raw(""));

            if !target.dependencies.is_empty() {
                let mut spans = vec![Span::styled(
                    "Dependencies: ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )];
                for (i, dep) in target.dependencies.iter().enumerate() {
                    if i > 0 {
                        spans.push(Span::styled(", ", Style::default().fg(Color::DarkGray)));
                    }
                    spans.push(Span::styled(dep, Style::default().fg(Color::Blue)));
                }
                lines.push(Line::from(spans));
            } else {
                lines.push(Line::from(vec![
                    Span::styled(
                        "Dependencies: ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("(none)", Style::default().fg(Color::DarkGray)),
                ]));
            }
            lines.push(Line::raw(""));

            lines.push(Line::styled(
                "Commands:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
            if target.commands.is_empty() {
                lines.push(Line::styled(
                    "  (none)",
                    Style::default().fg(Color::DarkGray),
                ));
            } else {
                for cmd in &target.commands {
                    if let Some(rest) = cmd.strip_prefix('@') {
                        lines.push(Line::from(vec![
                            Span::styled("  @", Style::default().fg(Color::DarkGray)),
                            Span::styled(rest, Style::default().fg(Color::Yellow)),
                        ]));
                    } else if let Some(rest) = cmd.strip_prefix('-') {
                        lines.push(Line::from(vec![
                            Span::styled("  -", Style::default().fg(Color::DarkGray)),
                            Span::styled(rest, Style::default().fg(Color::Red)),
                        ]));
                    } else {
                        lines.push(Line::from(format!("  {}", cmd)));
                    }
                }
            }
            lines
        } else {
            vec![Line::styled(
                "No matching target",
                Style::default().fg(Color::DarkGray),
            )]
        };

        let detail = Paragraph::new(detail_lines)
            .block(Block::bordered().title(" Details ".fg(Color::Cyan).bold()))
            .scroll((self.detail_scroll, 0));
        frame.render_widget(detail, detail_area);

        // Variables panel
        let var_lines = if let Some(idx) = selected_idx {
            let target = &self.targets[idx];
            let used: Vec<MakeVariable> = find_all_used_variables(target, &self.variables)
                .into_iter()
                .filter(|v| !v.value.is_empty())
                .collect();
            if used.is_empty() {
                vec![Line::styled(
                    "  (no variables used)",
                    Style::default().fg(Color::DarkGray),
                )]
            } else {
                used.into_iter()
                    .map(|var| {
                        Line::from(vec![
                            Span::styled(
                                var.name,
                                Style::default()
                                    .fg(Color::Magenta)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(" = ", Style::default().fg(Color::DarkGray)),
                            Span::styled(var.value, Style::default().fg(Color::Yellow)),
                        ])
                    })
                    .collect()
            }
        } else {
            vec![Line::raw("")]
        };

        let vars = Paragraph::new(var_lines)
            .block(Block::bordered().title(" Variables ".fg(Color::Cyan).bold()));
        frame.render_widget(vars, vars_area);

        // Popup overlay
        if let Mode::Popup(ref popup) = self.mode {
            self.draw_popup(frame, popup);
        }
    }

    fn draw_popup(&self, frame: &mut Frame, popup: &PopupState) {
        let has_vars = !popup.variables.is_empty();
        let var_lines = if has_vars { popup.variables.len() } else { 1 };
        // +2 for command preview line and dry-run indicator, +5 base
        let height = (var_lines + 8) as u16;
        // Compute the name column width (at least 12, grows for longer names)
        let name_col: usize = if has_vars {
            popup
                .variables
                .iter()
                .map(|(n, _)| n.len())
                .max()
                .unwrap_or(12)
                .max(12)
        } else {
            12
        };
        let width = 60u16.max(name_col as u16 + 30); // ensure enough room
        let area = centered_rect(width, height, frame.area());
        // How many chars of the value can we display
        // Layout: border(1) + arrow(2) + name(name_col) + " = "(3) + value + border(1)
        let value_max_width = (area.width as usize).saturating_sub(name_col + 7);

        frame.render_widget(Clear, area);

        let title = format!(" Run: {} ", popup.target_name);
        let block = Block::bordered()
            .title(title)
            .style(Style::default().fg(Color::Cyan));

        let mut lines: Vec<Line> = Vec::new();

        if has_vars {
            lines.push(Line::styled(
                "Edit variables (Tab/\u{2191}\u{2193} to switch, C-n dry-run, Enter to run, Esc to cancel):",
                Style::default().fg(Color::DarkGray),
            ));
            lines.push(Line::raw(""));

            for (i, (name, value)) in popup.variables.iter().enumerate() {
                let is_selected = i == popup.selected;
                let name_span = Span::styled(
                    format!("{:>width$}", name, width = name_col),
                    Style::default().fg(Color::Magenta),
                );
                let eq_span = Span::styled(" = ", Style::default().fg(Color::DarkGray));
                let value_style = if is_selected {
                    Style::default().add_modifier(Modifier::UNDERLINED | Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Yellow)
                };
                let arrow = if is_selected { "> " } else { "  " };

                // Scroll the value horizontally so the cursor is always visible.
                // Scrolling/truncation operate on chars (not bytes) so multibyte
                // input never lands mid-codepoint when slicing. cursor_char_idx
                // is only meaningful for the selected row — popup.cursor_pos
                // indexes into the selected variable's value, so reading it
                // against any other (potentially shorter) value would panic.
                let value_chars: Vec<char> = value.chars().collect();
                let display_value: String = if is_selected && value_chars.len() > value_max_width
                {
                    let cursor_char_idx = value[..popup.cursor_pos].chars().count();
                    let scroll =
                        cursor_char_idx.saturating_sub(value_max_width.saturating_sub(1));
                    let end = (scroll + value_max_width).min(value_chars.len());
                    value_chars[scroll..end].iter().collect()
                } else if value_chars.len() > value_max_width {
                    value_chars[..value_max_width].iter().collect()
                } else {
                    value.clone()
                };

                lines.push(Line::from(vec![
                    Span::styled(arrow, Style::default().fg(Color::Cyan)),
                    name_span,
                    eq_span,
                    Span::styled(display_value, value_style),
                ]));
            }
        } else {
            lines.push(Line::styled(
                "No variables to configure.",
                Style::default().fg(Color::DarkGray),
            ));
            lines.push(Line::raw(""));
            lines.push(Line::styled(
                "Press Enter to run, Esc to cancel.",
                Style::default().fg(Color::DarkGray),
            ));
        }

        // Dry-run toggle
        lines.push(Line::raw(""));
        let dry_run_spans = vec![
            Span::styled(
                "  C-n",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" dry-run: ", Style::default().fg(Color::DarkGray)),
            if popup.dry_run {
                Span::styled(
                    "[ON]",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled("[OFF]", Style::default().fg(Color::DarkGray))
            },
        ];
        lines.push(Line::from(dry_run_spans));

        // Command preview
        lines.push(Line::raw(""));
        let request = super::RunRequest {
            target_name: popup.target_name.clone(),
            overrides: popup.variables.clone(),
            dry_run: popup.dry_run,
        };
        let cmd_preview = request.to_command_parts(&self.makefile_path).join(" ");
        lines.push(Line::from(vec![
            Span::styled("  $ ", Style::default().fg(Color::DarkGray)),
            Span::styled(cmd_preview, Style::default().fg(Color::White)),
        ]));

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);

        if has_vars {
            let value = &popup.variables[popup.selected].1;
            let value_chars_count = value.chars().count();
            let cursor_char_idx = value[..popup.cursor_pos].chars().count();
            let scroll = if value_chars_count > value_max_width {
                cursor_char_idx.saturating_sub(value_max_width.saturating_sub(1))
            } else {
                0
            };
            let visible_cursor = cursor_char_idx - scroll;
            let cursor_x = area.x + 1 + 2 + name_col as u16 + 3 + visible_cursor as u16;
            let cursor_y = area.y + 1 + 2 + popup.selected as u16;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}
