//! Interactive TUI dashboard for AirLLM — monitor models, tokens, VRAM, and chat.

use std::io::{stdout, Write};
use std::time::{Duration, Instant};

use airllm_ollama::{ChatOptions, ChatMetrics, Message, ModelInfo, OllamaClient};
use anyhow::Result;
use crossterm::event::{poll, read, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Terminal;
use tokio::sync::mpsc;

/// Dashboard state.
pub struct Dashboard {
    ollama: OllamaClient,
    models: Vec<ModelInfo>,
    selected_model: usize,
    temperature: f32,
    top_p: f32,
    top_k: u32,
    num_ctx: u32,
    input: String,
    output: String,
    metrics: Option<ChatMetrics>,
    status: String,
    history: Vec<HistoryEntry>,
    vram_total_mb: u64,
    vram_used_mb: u64,
    models_loaded: Vec<String>,
    focus: Focus,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Input,
    ModelList,
    Params,
}

#[allow(dead_code)]
struct HistoryEntry {
    prompt: String,
    response: String,
    metrics: ChatMetrics,
}

impl Dashboard {
    pub fn new(ollama_url: &str) -> Self {
        Self {
            ollama: OllamaClient::new(ollama_url),
            models: Vec::new(),
            selected_model: 0,
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            num_ctx: 4096,
            input: String::new(),
            output: String::new(),
            metrics: None,
            status: "Initializing...".into(),
            history: Vec::new(),
            vram_total_mb: 0,
            vram_used_mb: 0,
            models_loaded: Vec::new(),
            focus: Focus::Input,
        }
    }

    async fn refresh_models(&mut self) {
        match self.ollama.list_models().await {
            Ok(models) => {
                self.status = format!("Loaded {} models", models.len());
                self.models = models;
            }
            Err(e) => {
                self.status = format!("Error loading models: {e}");
            }
        }
    }

    async fn refresh_system(&mut self) {
        // Query /api/ps for loaded models
        let url = format!("{}/api/ps", self.ollama.base_url());
        if let Ok(resp) = reqwest::get(&url).await {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(models) = json.get("models").and_then(|m| m.as_array()) {
                    self.models_loaded = models
                        .iter()
                        .filter_map(|m| {
                            m.get("name")
                                .and_then(|n| n.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect();
                }
            }
        }

        // Query nvidia-smi for VRAM (if available)
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .args([
                "--query-gpu=memory.total,memory.used",
                "--format=csv,noheader,nounits",
            ])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = text.lines().next() {
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 2 {
                        self.vram_total_mb = parts[0].parse().unwrap_or(0);
                        self.vram_used_mb = parts[1].parse().unwrap_or(0);
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    async fn send_chat(&mut self) {
        if self.input.trim().is_empty() || self.models.is_empty() {
            return;
        }
        let model = self.models[self.selected_model].name.clone();
        let prompt = self.input.clone();
        self.input.clear();
        self.status = format!("Sending to {model}...");
        self.output.clear();

        let messages = vec![
            Message::system("You are a helpful coding assistant."),
            Message::user(&prompt),
        ];
        let options = ChatOptions {
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            num_ctx: self.num_ctx,
        };

        match self
            .ollama
            .chat_with_metrics(&model, &messages, options)
            .await
        {
            Ok((response, metrics)) => {
                self.output = response.clone();
                self.status = format!(
                    "Done: {:.1} tok/s | {}ms | {} out tokens",
                    metrics.tokens_per_second,
                    metrics.latency_ms,
                    metrics.output_tokens
                );
                self.metrics = Some(metrics.clone());
                self.history.push(HistoryEntry {
                    prompt,
                    response,
                    metrics,
                });
            }
            Err(e) => {
                self.status = format!("Error: {e}");
            }
        }
    }

    fn handle_key(&mut self, key: KeyCode) -> bool {
        match self.focus {
            Focus::Input => match key {
                KeyCode::Enter => {
                    // Send chat in async context — we'll handle via channel
                    self.status = "Sending...".into();
                    true
                }
                KeyCode::Tab => {
                    self.focus = Focus::ModelList;
                    self.status = "Focus: Model List".into();
                    false
                }
                KeyCode::Char(c) => {
                    self.input.push(c);
                    false
                }
                KeyCode::Backspace => {
                    self.input.pop();
                    false
                }
                KeyCode::Esc => true,
                _ => false,
            },
            Focus::ModelList => match key {
                KeyCode::Up => {
                    if self.selected_model > 0 {
                        self.selected_model -= 1;
                    }
                    false
                }
                KeyCode::Down => {
                    if self.selected_model + 1 < self.models.len() {
                        self.selected_model += 1;
                    }
                    false
                }
                KeyCode::Tab => {
                    self.focus = Focus::Params;
                    self.status = "Focus: Parameters".into();
                    false
                }
                KeyCode::Enter => {
                    self.focus = Focus::Input;
                    self.status = "Focus: Input".into();
                    false
                }
                KeyCode::Esc => true,
                _ => false,
            },
            Focus::Params => match key {
                KeyCode::Up => {
                    self.temperature = (self.temperature + 0.1).min(2.0);
                    false
                }
                KeyCode::Down => {
                    self.temperature = (self.temperature - 0.1).max(0.0);
                    false
                }
                KeyCode::Left => {
                    self.top_p = (self.top_p - 0.05).max(0.0);
                    false
                }
                KeyCode::Right => {
                    self.top_p = (self.top_p + 0.05).min(1.0);
                    false
                }
                KeyCode::Tab => {
                    self.focus = Focus::Input;
                    self.status = "Focus: Input".into();
                    false
                }
                KeyCode::Esc => true,
                _ => false,
            },
        }
    }
}

/// Run the dashboard TUI.
pub async fn run_dashboard(ollama_url: &str) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut dashboard = Dashboard::new(ollama_url);
    dashboard.refresh_models().await;
    dashboard.refresh_system().await;
    dashboard.status = "Ready. Type and press Enter to chat. Tab to switch focus.".into();

    // Channel for async chat results
    let (chat_tx, mut chat_rx) = mpsc::channel::<(String, ChatMetrics)>(10);
    let (error_tx, mut error_rx) = mpsc::channel::<String>(10);

    let mut last_refresh = Instant::now();
    let tick_ms = 100u64;

    loop {
        // Check for async results
        if let Ok((response, metrics)) = chat_rx.try_recv() {
            dashboard.output = response.clone();
            dashboard.status = format!(
                "Done: {:.1} tok/s | {}ms | {} out | {} in",
                metrics.tokens_per_second,
                metrics.latency_ms,
                metrics.output_tokens,
                metrics.input_tokens
            );
            dashboard.metrics = Some(metrics.clone());
            dashboard.history.push(HistoryEntry {
                prompt: dashboard.input.clone(),
                response,
                metrics,
            });
            dashboard.input.clear();
        }
        if let Ok(err) = error_rx.try_recv() {
            dashboard.status = format!("Error: {err}");
        }

        // Periodic refresh (every 5s)
        if last_refresh.elapsed() > Duration::from_secs(5) {
            dashboard.refresh_system().await;
            last_refresh = Instant::now();
        }

        // Draw
        terminal.draw(|f| draw_dashboard(f, &dashboard))?;

        // Poll for input (non-blocking)
        if poll(Duration::from_millis(tick_ms))? {
            if let Event::Key(key) = read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Handle Enter specially for chat
                if key.code == KeyCode::Enter && dashboard.focus == Focus::Input {
                    if !dashboard.input.trim().is_empty() && !dashboard.models.is_empty() {
                        let model = dashboard.models[dashboard.selected_model].name.clone();
                        let prompt = dashboard.input.clone();
                        let ollama = dashboard.ollama.clone();
                        let temp = dashboard.temperature;
                        let top_p_val = dashboard.top_p;
                        let top_k_val = dashboard.top_k;
                        let ctx = dashboard.num_ctx;
                        let tx = chat_tx.clone();
                        let err_tx = error_tx.clone();
                        dashboard.status = format!("Sending to {model}...");
                        tokio::spawn(async move {
                            let messages = vec![
                                Message::system("You are a helpful coding assistant."),
                                Message::user(&prompt),
                            ];
                            let options = ChatOptions {
                                temperature: temp,
                                top_p: top_p_val,
                                top_k: top_k_val,
                                num_ctx: ctx,
                            };
                            match ollama.chat_with_metrics(&model, &messages, options).await {
                                Ok((resp, metrics)) => {
                                    let _ = tx.send((resp, metrics)).await;
                                }
                                Err(e) => {
                                    let _ = err_tx.send(e.to_string()).await;
                                }
                            }
                        });
                        dashboard.input.clear();
                    }
                    continue;
                }

                let should_quit = dashboard.handle_key(key.code);
                if should_quit {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.backend_mut().flush()?;

    Ok(())
}

fn draw_dashboard(f: &mut ratatui::Frame<'_>, d: &Dashboard) {
    let area = f.area();

    // Main layout: left panel (models + params) | right panel (chat + stats)
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(40), Constraint::Min(60)].as_ref())
        .split(area);

    // Left column: models list + params + system info
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),   // Models
            Constraint::Length(10), // Params
            Constraint::Length(8),  // System
        ].as_ref())
        .split(main[0]);

    // Right column: input + output + stats + status
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Input
            Constraint::Min(10),    // Output
            Constraint::Length(8),  // Stats
            Constraint::Length(1),  // Status
        ].as_ref())
        .split(main[1]);

    // ── Models list ──
    let model_items: Vec<ListItem> = d
        .models
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let prefix = if i == d.selected_model { "▶ " } else { "  " };
            let loaded = if d.models_loaded.contains(&m.name) { " ●" } else { "" };
            ListItem::new(format!("{prefix}{name} ({size}, {quant}){loaded}",
                name = m.name, size = m.size, quant = m.quantization))
        })
        .collect();
    let model_list = List::new(model_items)
        .block(
            Block::default()
                .title("Models (↑↓ select, Tab next)")
                .borders(Borders::ALL)
                .border_style(if d.focus == Focus::ModelList {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                }),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(model_list, left[0]);

    // ── Parameters ──
    let params_text = vec![
        Line::from(vec![
            Span::styled("Temperature: ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{:.1}", d.temperature)),
            Span::raw("  (↑↓)"),
        ]),
        Line::from(vec![
            Span::styled("Top-p:       ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{:.2}", d.top_p)),
            Span::raw("  (←→)"),
        ]),
        Line::from(vec![
            Span::styled("Top-k:       ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{}", d.top_k)),
        ]),
        Line::from(vec![
            Span::styled("Context:     ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{} tokens", d.num_ctx)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "↑↓ temp | ←→ top_p | Tab cycle",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    let params = Paragraph::new(Text::from(params_text))
        .block(
            Block::default()
                .title("Parameters")
                .borders(Borders::ALL)
                .border_style(if d.focus == Focus::Params {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                }),
        );
    f.render_widget(params, left[1]);

    // ── System info ──
    let vram_pct = if d.vram_total_mb > 0 {
        d.vram_used_mb as f64 / d.vram_total_mb as f64 * 100.0
    } else {
        0.0
    };
    let vram_bar_len = 20usize;
    let vram_filled = (vram_pct / 100.0 * vram_bar_len as f64) as usize;
    let vram_bar: String = "█".repeat(vram_filled) + &"░".repeat(vram_bar_len - vram_filled);

    let sys_text = vec![
        Line::from(vec![
            Span::styled("VRAM: ", Style::default().fg(Color::Green)),
            Span::raw(format!(
                "{used}MB / {total}MB ({pct:.0}%)",
                used = d.vram_used_mb,
                total = d.vram_total_mb,
                pct = vram_pct
            )),
        ]),
        Line::from(format!(" {vram_bar}")),
        Line::from(""),
        Line::from(vec![
            Span::styled("Loaded: ", Style::default().fg(Color::Green)),
            Span::raw(if d.models_loaded.is_empty() {
                "(none)".into()
            } else {
                d.models_loaded.join(", ")
            }),
        ]),
    ];
    let sys_info = Paragraph::new(Text::from(sys_text))
        .block(Block::default().title("System").borders(Borders::ALL));
    f.render_widget(sys_info, left[2]);

    // ── Input ──
    let input_style = if d.focus == Focus::Input {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let input = Paragraph::new(format!("> {}", d.input))
        .block(
            Block::default()
                .title("Input (Enter to send, Tab to switch)")
                .borders(Borders::ALL)
                .border_style(input_style),
        );
    f.render_widget(input, right[0]);

    // ── Output ──
    let output_text = if d.output.is_empty() {
        Text::raw("(output will appear here)")
    } else {
        Text::raw(d.output.clone())
    };
    let output = Paragraph::new(output_text)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title("Output")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        );
    f.render_widget(output, right[1]);

    // ── Stats ──
    let stats_text = if let Some(ref m) = d.metrics {
        vec![
            Line::from(vec![
                Span::styled("Model:        ", Style::default().fg(Color::Cyan)),
                Span::raw(&m.model),
            ]),
            Line::from(vec![
                Span::styled("Latency:      ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{} ms", m.latency_ms)),
            ]),
            Line::from(vec![
                Span::styled("Tokens/s:     ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("{:.1}", m.tokens_per_second),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Output tokens:", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}", m.output_tokens)),
            ]),
            Line::from(vec![
                Span::styled("Input tokens: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}", m.input_tokens)),
            ]),
            Line::from(vec![
                Span::styled("Context:      ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{} tokens", m.num_ctx)),
            ]),
        ]
    } else {
        vec![Line::from("(no metrics yet — send a message)")]
    };
    let stats = Paragraph::new(Text::from(stats_text))
        .block(Block::default().title("Metrics").borders(Borders::ALL));
    f.render_widget(stats, right[2]);

    // ── Status bar ──
    let status = Paragraph::new(format!(" {} ", d.status))
        .style(Style::default().fg(Color::Black).bg(Color::Yellow));
    f.render_widget(status, right[3]);
}