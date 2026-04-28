//! Channels screen: list the supported adapters (Telegram, Discord, Slack),
//! setup wizards, test & toggle.

use crate::tui::theme;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph};
use ratatui::Frame;

// ── Data types ──────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ChannelInfo {
    pub name: String,
    pub display_name: String,
    pub status: ChannelStatus,
    pub env_vars: Vec<(String, bool)>, // (var_name, is_set)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ChannelStatus {
    Ready,
    MissingEnv,
    NotConfigured,
}

// ── Channel definitions — Telegram + Discord + Slack ────────────────────────

struct ChannelDef {
    name: &'static str,
    display_name: &'static str,
    env_vars: &'static [&'static str],
    description: &'static str,
}

const CHANNEL_DEFS: &[ChannelDef] = &[
    ChannelDef {
        name: "telegram",
        display_name: "Telegram",
        env_vars: &["TELEGRAM_BOT_TOKEN"],
        description: "Telegram Bot API adapter (long-polling)",
    },
    ChannelDef {
        name: "discord",
        display_name: "Discord",
        env_vars: &["DISCORD_BOT_TOKEN"],
        description: "Discord Gateway WebSocket adapter",
    },
    ChannelDef {
        name: "slack",
        display_name: "Slack",
        env_vars: &["SLACK_APP_TOKEN", "SLACK_BOT_TOKEN"],
        description: "Slack Socket Mode adapter",
    },
];

// ── State ───────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Eq)]
pub enum ChannelSubScreen {
    List,
    Setup,
    Testing,
}

pub struct ChannelState {
    pub sub: ChannelSubScreen,
    pub channels: Vec<ChannelInfo>,
    pub list_state: ListState,
    pub loading: bool,
    pub tick: usize,
    // Setup wizard
    pub setup_channel_idx: Option<usize>,
    pub setup_field_idx: usize,
    pub setup_input: String,
    pub setup_values: Vec<(String, String)>, // collected (env_var, value) pairs
    // Test
    pub test_result: Option<(bool, String)>,
    pub status_msg: String,
}

pub enum ChannelAction {
    Continue,
    Refresh,
    TestChannel(String),
    SaveChannel(String, Vec<(String, String)>),
}

impl ChannelState {
    pub fn new() -> Self {
        Self {
            sub: ChannelSubScreen::List,
            channels: Vec::new(),
            list_state: ListState::default(),
            loading: false,
            tick: 0,
            setup_channel_idx: None,
            setup_field_idx: 0,
            setup_input: String::new(),
            setup_values: Vec::new(),
            test_result: None,
            status_msg: String::new(),
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    fn filtered_channels(&self) -> Vec<&ChannelInfo> {
        self.channels.iter().collect()
    }

    fn ready_count(&self) -> usize {
        self.channels
            .iter()
            .filter(|ch| ch.status == ChannelStatus::Ready)
            .count()
    }

    /// Build the default channel list from env var detection.
    pub fn build_default_channels(&mut self) {
        self.channels.clear();
        for def in CHANNEL_DEFS {
            let env_vars: Vec<(String, bool)> = def
                .env_vars
                .iter()
                .map(|v| (v.to_string(), std::env::var(v).is_ok()))
                .collect();
            let all_set = env_vars.is_empty() || env_vars.iter().all(|(_, set)| *set);
            let any_set = env_vars.iter().any(|(_, set)| *set);
            let status = if all_set && !env_vars.is_empty() {
                ChannelStatus::Ready
            } else if any_set {
                ChannelStatus::MissingEnv
            } else {
                ChannelStatus::NotConfigured
            };
            self.channels.push(ChannelInfo {
                name: def.name.to_string(),
                display_name: def.display_name.to_string(),
                status,
                env_vars,
            });
        }
        self.list_state.select(Some(0));
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ChannelAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return ChannelAction::Continue;
        }
        match self.sub {
            ChannelSubScreen::List => self.handle_list(key),
            ChannelSubScreen::Setup => self.handle_setup(key),
            ChannelSubScreen::Testing => self.handle_testing(key),
        }
    }

    fn handle_list(&mut self, key: KeyEvent) -> ChannelAction {
        let filtered = self.filtered_channels();
        let total = filtered.len();
        if total == 0 {
            if key.code == KeyCode::Char('r') {
                return ChannelAction::Refresh;
            }
            return ChannelAction::Continue;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.list_state.selected().unwrap_or(0);
                let next = if i == 0 { total - 1 } else { i - 1 };
                self.list_state.select(Some(next));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self.list_state.selected().unwrap_or(0);
                let next = (i + 1) % total;
                self.list_state.select(Some(next));
            }
            KeyCode::Enter => {
                if let Some(sel) = self.list_state.selected() {
                    let filtered = self.filtered_channels();
                    if let Some(ch) = filtered.get(sel) {
                        // Find the global index for this channel
                        let ch_name = ch.name.clone();
                        if let Some(idx) = self.channels.iter().position(|c| c.name == ch_name) {
                            self.setup_channel_idx = Some(idx);
                            self.setup_field_idx = 0;
                            self.setup_input.clear();
                            self.setup_values.clear();
                            self.sub = ChannelSubScreen::Setup;
                        }
                    }
                }
            }
            KeyCode::Char('t') => {
                if let Some(sel) = self.list_state.selected() {
                    let filtered = self.filtered_channels();
                    if let Some(ch) = filtered.get(sel) {
                        let name = ch.name.clone();
                        self.test_result = None;
                        self.sub = ChannelSubScreen::Testing;
                        return ChannelAction::TestChannel(name);
                    }
                }
            }
            KeyCode::Char('r') => return ChannelAction::Refresh,
            _ => {}
        }
        ChannelAction::Continue
    }

    fn handle_setup(&mut self, key: KeyEvent) -> ChannelAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = ChannelSubScreen::List;
            }
            KeyCode::Char(c) => {
                self.setup_input.push(c);
            }
            KeyCode::Backspace => {
                self.setup_input.pop();
            }
            KeyCode::Enter => {
                if let Some(idx) = self.setup_channel_idx {
                    if idx < self.channels.len() {
                        let env_vars = &CHANNEL_DEFS
                            .iter()
                            .find(|d| d.name == self.channels[idx].name)
                            .map(|d| d.env_vars)
                            .unwrap_or(&[]);

                        // Save current field value
                        if self.setup_field_idx < env_vars.len() && !self.setup_input.is_empty() {
                            self.setup_values.push((
                                env_vars[self.setup_field_idx].to_string(),
                                self.setup_input.clone(),
                            ));
                        }

                        if self.setup_field_idx + 1 < env_vars.len() {
                            self.setup_field_idx += 1;
                            self.setup_input.clear();
                        } else {
                            // All fields collected — emit save action
                            let name = self.channels[idx].name.clone();
                            let values = self.setup_values.clone();
                            self.sub = ChannelSubScreen::List;
                            if !values.is_empty() {
                                return ChannelAction::SaveChannel(name, values);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        ChannelAction::Continue
    }

    fn handle_testing(&mut self, key: KeyEvent) -> ChannelAction {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.sub = ChannelSubScreen::List;
            }
            _ => {}
        }
        ChannelAction::Continue
    }
}

// ── Drawing ─────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, area: Rect, state: &mut ChannelState) {
    let ready = state.ready_count();
    let total = state.channels.len();
    let title = format!(" Channels ({ready}/{total} ready) ");

    let block = Block::default()
        .title(Line::from(vec![Span::styled(title, theme::title_style())]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    match state.sub {
        ChannelSubScreen::List => draw_list(f, inner, state),
        ChannelSubScreen::Setup => draw_setup(f, inner, state),
        ChannelSubScreen::Testing => draw_testing(f, inner, state),
    }
}

fn draw_list(f: &mut Frame, area: Rect, state: &mut ChannelState) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // header
        Constraint::Min(3),    // list
        Constraint::Length(1), // hints
    ])
    .split(area);

    // Header
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!("  {:<18} {:<16} {}", "Channel", "Status", "Env Vars"),
            theme::table_header(),
        )])),
        chunks[0],
    );

    if state.loading {
        let spinner = theme::SPINNER_FRAMES[state.tick % theme::SPINNER_FRAMES.len()];
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("  {spinner} "), Style::default().fg(theme::CYAN)),
                Span::styled("Loading channels\u{2026}", theme::dim_style()),
            ])),
            chunks[1],
        );
    } else {
        let filtered = state.filtered_channels();
        let items: Vec<ListItem> = filtered
            .iter()
            .map(|ch| {
                let (badge, badge_style) = match ch.status {
                    ChannelStatus::Ready => ("[Ready]", theme::channel_ready()),
                    ChannelStatus::MissingEnv => ("[Missing env]", theme::channel_missing()),
                    ChannelStatus::NotConfigured => ("[Not configured]", theme::channel_off()),
                };
                let env_summary: String = ch
                    .env_vars
                    .iter()
                    .map(|(v, set)| {
                        if *set {
                            format!("\u{2714}{v}")
                        } else {
                            format!("\u{2718}{v}")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {:<18}", ch.display_name),
                        Style::default().fg(theme::CYAN),
                    ),
                    Span::styled(format!(" {:<16}", badge), badge_style),
                    Span::styled(format!(" {env_summary}"), theme::dim_style()),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(theme::selected_style())
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[1], &mut state.list_state);
    }

    let hints = Paragraph::new(Line::from(vec![Span::styled(
        "  [\u{2191}\u{2193}] Navigate  [Enter] Setup  [t] Test  [r] Refresh",
        theme::hint_style(),
    )]));
    f.render_widget(hints, chunks[2]);
}

fn draw_setup(f: &mut Frame, area: Rect, state: &ChannelState) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // title + description
        Constraint::Length(1), // separator
        Constraint::Length(2), // current field
        Constraint::Length(1), // input
        Constraint::Min(2),    // TOML preview
        Constraint::Length(1), // hints
    ])
    .split(area);

    let (ch_name, ch_display, ch_desc, env_vars) = if let Some(idx) = state.setup_channel_idx {
        if let Some(def) = CHANNEL_DEFS
            .iter()
            .find(|d| idx < state.channels.len() && d.name == state.channels[idx].name)
        {
            (def.name, def.display_name, def.description, def.env_vars)
        } else {
            ("?", "?", "", &[] as &[&str])
        }
    } else {
        ("?", "?", "", &[] as &[&str])
    };

    // Title
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![Span::styled(
                format!("  Setup: {ch_display}"),
                Style::default()
                    .fg(theme::CYAN)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                format!("  {ch_desc}"),
                theme::dim_style(),
            )]),
        ]),
        chunks[0],
    );

    // Separator
    let sep = "\u{2500}".repeat(chunks[1].width as usize);
    f.render_widget(
        Paragraph::new(Span::styled(sep, theme::dim_style())),
        chunks[1],
    );

    // Current field
    if env_vars.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                "  This channel has no secret env vars — configure via config.toml",
                theme::dim_style(),
            )])),
            chunks[2],
        );
    } else if state.setup_field_idx < env_vars.len() {
        let var = env_vars[state.setup_field_idx];
        let field_num = state.setup_field_idx + 1;
        let total = env_vars.len();
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(format!("  [{field_num}/{total}] Set ")),
                Span::styled(var, Style::default().fg(theme::YELLOW)),
                Span::raw(":"),
            ])),
            chunks[2],
        );
    }

    // Input
    let display = if state.setup_input.is_empty() {
        "paste value here..."
    } else {
        &state.setup_input
    };
    let style = if state.setup_input.is_empty() {
        theme::dim_style()
    } else {
        theme::input_style()
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  > "),
            Span::styled(display, style),
            Span::styled(
                "\u{2588}",
                Style::default()
                    .fg(theme::GREEN)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
        ])),
        chunks[3],
    );

    // TOML preview
    let mut toml_lines = vec![Line::from(Span::styled(
        "  Add to config.toml:",
        theme::dim_style(),
    ))];
    toml_lines.push(Line::from(Span::styled(
        format!("  [channels.{ch_name}]"),
        Style::default().fg(theme::YELLOW),
    )));
    for var in env_vars {
        toml_lines.push(Line::from(Span::styled(
            format!("  # {var} = \"...\""),
            Style::default().fg(theme::YELLOW),
        )));
    }
    f.render_widget(Paragraph::new(toml_lines), chunks[4]);

    // Hints
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            "  [Enter] Next field / Save  [Esc] Back",
            theme::hint_style(),
        )])),
        chunks[5],
    );
}

fn draw_testing(f: &mut Frame, area: Rect, state: &ChannelState) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(2),
        Constraint::Length(1),
    ])
    .split(area);

    let ch_name = state
        .setup_channel_idx
        .and_then(|i| state.channels.get(i))
        .map(|c| c.display_name.as_str())
        .or_else(|| {
            state.list_state.selected().and_then(|i| {
                let filtered = state.filtered_channels();
                filtered.get(i).map(|c| c.display_name.as_str())
            })
        })
        .unwrap_or("?");

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!("  Testing {ch_name}\u{2026}"),
            Style::default().fg(theme::CYAN),
        )])),
        chunks[0],
    );

    match &state.test_result {
        None => {
            let spinner = theme::SPINNER_FRAMES[state.tick % theme::SPINNER_FRAMES.len()];
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(format!("  {spinner} "), Style::default().fg(theme::CYAN)),
                    Span::styled("Checking credentials\u{2026}", theme::dim_style()),
                ])),
                chunks[1],
            );
        }
        Some((true, msg)) => {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(vec![
                        Span::styled("  \u{2714} ", Style::default().fg(theme::GREEN)),
                        Span::raw("Test passed"),
                    ]),
                    Line::from(vec![Span::styled(format!("  {msg}"), theme::dim_style())]),
                ]),
                chunks[1],
            );
        }
        Some((false, msg)) => {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(vec![
                        Span::styled("  \u{2718} ", Style::default().fg(theme::RED)),
                        Span::raw("Test failed"),
                    ]),
                    Line::from(vec![Span::styled(
                        format!("  {msg}"),
                        Style::default().fg(theme::RED),
                    )]),
                ]),
                chunks[1],
            );
        }
    }

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            "  [Enter/Esc] Back",
            theme::hint_style(),
        )])),
        chunks[2],
    );
}
