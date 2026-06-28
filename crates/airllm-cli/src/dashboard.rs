//! Interactive TUI dashboard for AirLLM — clickable, branded, with execution indicators and API config.

use std::io::{stdout, Write};
use std::time::{Duration, Instant};

use airllm_ollama::{ChatOptions, ChatMetrics, Message, ModelInfo, OllamaClient};
use airllm_orchestrator::{CodeRequest, Orchestrator};
use anyhow::Result;
use crossterm::event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseEvent, MouseEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Terminal;
use tokio::sync::mpsc;

// ── Branding ────────────────────────────────────────────────────────────────

const EDGESEARCH_BRAND: &str = "⬡ EdgeSearch";

// ── Modes ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Chat,
    Code,
    Review,
    Test,
    Refactor,
    Autonomous,
}

impl Mode {
    pub fn all() -> &'static [Mode] {
        &[Mode::Chat, Mode::Code, Mode::Review, Mode::Test, Mode::Refactor, Mode::Autonomous]
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::Chat => "Chat",
            Mode::Code => "Code",
            Mode::Review => "Review",
            Mode::Test => "Test",
            Mode::Refactor => "Refactor",
            Mode::Autonomous => "Autonomous",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Mode::Chat => "Simple chat with selected model",
            Mode::Code => "Generate code via orchestrator (decompose → execute → consolidate)",
            Mode::Review => "Review files for bugs, security, performance",
            Mode::Test => "Generate comprehensive tests for files",
            Mode::Refactor => "Refactor code with a specific goal",
            Mode::Autonomous => "Run agent in continuous loop (execute → evaluate → decide → repeat)",
        }
    }

    #[allow(dead_code)]
    pub fn next(self) -> Self {
        let all = Self::all();
        let idx = all.iter().position(|m| *m == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }

    #[allow(dead_code)]
    pub fn prev(self) -> Self {
        let all = Self::all();
        let idx = all.iter().position(|m| *m == self).unwrap_or(0);
        all[(idx + all.len() - 1) % all.len()]
    }
}

// ── Agents ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct AgentDef {
    pub name: &'static str,
    pub model: &'static str,
    pub description: &'static str,
}

impl AgentDef {
    pub fn all() -> Vec<AgentDef> {
        vec![
            AgentDef { name: "Coder", model: "qwen3.6:27b", description: "Implements code from task description" },
            AgentDef { name: "Reviewer", model: "qwen3.6:27b", description: "Reviews code for bugs, security, performance" },
            AgentDef { name: "Tester", model: "qwen3.5:4b", description: "Generates comprehensive test suites" },
            AgentDef { name: "Architect", model: "qwen3-coder-next:q8_0", description: "Designs module structure and decomposition" },
            AgentDef { name: "Debugger", model: "qwen3-coder-next:q8_0", description: "Analyzes errors and proposes fixes" },
            AgentDef { name: "Refactorer", model: "qwen3.6:27b", description: "Improves code quality without changing behavior" },
            AgentDef { name: "Documenter", model: "qwen3.5:4b", description: "Generates clear documentation" },
            AgentDef { name: "Auto (Router)", model: "", description: "Let ModelRouter choose based on complexity" },
        ]
    }
}

// ── Focus ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Focus {
    Input,
    ModeBar,
    ModelList,
    AgentList,
    Params,
    ApiConfig,
}

impl Focus {
    #[allow(dead_code)]
    pub fn next(self) -> Self {
        match self {
            Focus::Input => Focus::ModeBar,
            Focus::ModeBar => Focus::ModelList,
            Focus::ModelList => Focus::AgentList,
            Focus::AgentList => Focus::Params,
            Focus::Params => Focus::ApiConfig,
            Focus::ApiConfig => Focus::Input,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Focus::Input => "Input",
            Focus::ModeBar => "Mode",
            Focus::ModelList => "Models",
            Focus::AgentList => "Agents",
            Focus::Params => "Params",
            Focus::ApiConfig => "APIs",
        }
    }
}

// ── Execution State ─────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum ExecState {
    Idle,
    Running { started: Instant, #[allow(dead_code)] mode_label: String },
    Done,
}

impl ExecState {
    pub fn is_running(&self) -> bool {
        matches!(self, ExecState::Running { .. })
    }

    pub fn spinner(&self) -> char {
        match self {
            ExecState::Running { started, .. } => {
                let elapsed = started.elapsed().as_millis();
                let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                frames[(elapsed / 100) as usize % frames.len()]
            }
            ExecState::Done => '✓',
            ExecState::Idle => '○',
        }
    }

    pub fn elapsed_secs(&self) -> f64 {
        match self {
            ExecState::Running { started, .. } => started.elapsed().as_secs_f64(),
            _ => 0.0,
        }
    }
}

// ── API Config ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct ApiConfig {
    pub name: String,
    pub env_var: String,
    pub configured: bool,
    pub description: String,
}

impl ApiConfig {
    pub fn all() -> Vec<ApiConfig> {
        vec![
            ApiConfig { name: "Twitter/X".into(), env_var: "SOCIAL_API_KEY".into(), configured: std::env::var("SOCIAL_API_KEY").is_ok(), description: "Post to Twitter/X".into() },
            ApiConfig { name: "LinkedIn".into(), env_var: "LINKEDIN_API_KEY".into(), configured: std::env::var("LINKEDIN_API_KEY").is_ok(), description: "Post to LinkedIn".into() },
            ApiConfig { name: "Telegram".into(), env_var: "TELEGRAM_BOT_TOKEN".into(), configured: std::env::var("TELEGRAM_BOT_TOKEN").is_ok(), description: "Send Telegram messages".into() },
            ApiConfig { name: "Slack".into(), env_var: "SLACK_WEBHOOK_URL".into(), configured: std::env::var("SLACK_WEBHOOK_URL").is_ok(), description: "Send Slack messages".into() },
            ApiConfig { name: "Discord".into(), env_var: "DISCORD_WEBHOOK_URL".into(), configured: std::env::var("DISCORD_WEBHOOK_URL").is_ok(), description: "Send Discord messages".into() },
            ApiConfig { name: "SMTP Email".into(), env_var: "SMTP_HOST".into(), configured: std::env::var("SMTP_HOST").is_ok(), description: "Send emails via SMTP".into() },
            ApiConfig { name: "WebSearch".into(), env_var: "FIRECRAWL_API_KEY".into(), configured: std::env::var("FIRECRAWL_API_KEY").is_ok(), description: "Web search via Firecrawl".into() },
            ApiConfig { name: "GitHub".into(), env_var: "GITHUB_TOKEN".into(), configured: std::env::var("GITHUB_TOKEN").is_ok(), description: "GitHub API access".into() },
        ]
    }
}

// ── History ────────────────────────────────────────────────────────────────

#[allow(dead_code)]
struct HistoryEntry {
    mode: Mode,
    agent: String,
    model: String,
    prompt: String,
    response: String,
    metrics: ChatMetrics,
}

// ── Dashboard State ─────────────────────────────────────────────────────────

pub struct Dashboard {
    ollama: OllamaClient,
    orchestrator: Orchestrator,
    models: Vec<ModelInfo>,
    agents: Vec<AgentDef>,
    apis: Vec<ApiConfig>,
    selected_model: usize,
    selected_agent: usize,
    selected_api: usize,
    mode: Mode,
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
    autonomous_running: bool,
    autonomous_cycles: u64,
    exec_state: ExecState,
    tick: u64,
}

impl Dashboard {
    pub fn new(ollama_url: &str) -> Self {
        let ollama = OllamaClient::new(ollama_url);
        let orchestrator = Orchestrator::new(ollama.clone());
        Self {
            ollama,
            orchestrator,
            models: Vec::new(),
            agents: AgentDef::all(),
            apis: ApiConfig::all(),
            selected_model: 0,
            selected_agent: 0,
            selected_api: 0,
            mode: Mode::Chat,
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
            autonomous_running: false,
            autonomous_cycles: 0,
            exec_state: ExecState::Idle,
            tick: 0,
        }
    }

    async fn refresh_models(&mut self) {
        match self.ollama.list_models().await {
            Ok(models) => {
                self.status = format!("Loaded {} models, {} agents, {} APIs", models.len(), self.agents.len(), self.apis.iter().filter(|a| a.configured).count());
                self.models = models;
            }
            Err(e) => {
                self.status = format!("Error loading models: {e}");
            }
        }
    }

    async fn refresh_system(&mut self) {
        let url = format!("{}/api/ps", self.ollama.base_url());
        if let Ok(resp) = reqwest::get(&url).await {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(models) = json.get("models").and_then(|m| m.as_array()) {
                    self.models_loaded = models
                        .iter()
                        .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                        .collect();
                }
            }
        }

        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=memory.total,memory.used", "--format=csv,noheader,nounits"])
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

    fn current_model(&self) -> String {
        let agent = &self.agents[self.selected_agent];
        if agent.model.is_empty() {
            if !self.models.is_empty() {
                self.models[self.selected_model].name.clone()
            } else {
                "qwen3.5:4b".into()
            }
        } else {
            let found = self.models.iter().find(|m| m.name == agent.model);
            if let Some(m) = found {
                m.name.clone()
            } else if !self.models.is_empty() {
                self.models[self.selected_model].name.clone()
            } else {
                agent.model.into()
            }
        }
    }

    fn current_agent_name(&self) -> &str {
        self.agents[self.selected_agent].name
    }

    fn handle_key(&mut self, key: KeyCode) -> bool {
        match self.focus {
            Focus::Input => match key {
                KeyCode::Tab => {
                    self.focus = Focus::ModeBar;
                    self.status = format!("Focus: {} — ←→ to change mode", self.focus.label());
                    false
                }
                KeyCode::Char(c) => { self.input.push(c); false }
                KeyCode::Backspace => { self.input.pop(); false }
                KeyCode::Esc => true,
                _ => false,
            },
            Focus::ModeBar => match key {
                KeyCode::Left => {
                    self.mode = self.mode.prev();
                    self.status = format!("Mode: {} — {}", self.mode.label(), self.mode.description());
                    false
                }
                KeyCode::Right => {
                    self.mode = self.mode.next();
                    self.status = format!("Mode: {} — {}", self.mode.label(), self.mode.description());
                    false
                }
                KeyCode::Tab => {
                    self.focus = Focus::ModelList;
                    self.status = format!("Focus: {} — ↑↓ to select", self.focus.label());
                    false
                }
                KeyCode::Enter => {
                    self.focus = Focus::Input;
                    self.status = "Focus: Input — type and Enter".into();
                    false
                }
                KeyCode::Esc => true,
                _ => false,
            },
            Focus::ModelList => match key {
                KeyCode::Up => { if self.selected_model > 0 { self.selected_model -= 1; } false }
                KeyCode::Down => { if self.selected_model + 1 < self.models.len() { self.selected_model += 1; } false }
                KeyCode::Tab => { self.focus = Focus::AgentList; self.status = format!("Focus: {}", self.focus.label()); false }
                KeyCode::Enter => { self.focus = Focus::Input; self.status = "Focus: Input".into(); false }
                KeyCode::Esc => true,
                _ => false,
            },
            Focus::AgentList => match key {
                KeyCode::Up => { if self.selected_agent > 0 { self.selected_agent -= 1; } false }
                KeyCode::Down => { if self.selected_agent + 1 < self.agents.len() { self.selected_agent += 1; } false }
                KeyCode::Tab => { self.focus = Focus::Params; self.status = format!("Focus: {}", self.focus.label()); false }
                KeyCode::Enter => { self.focus = Focus::Input; self.status = "Focus: Input".into(); false }
                KeyCode::Esc => true,
                _ => false,
            },
            Focus::Params => match key {
                KeyCode::Up => { self.temperature = (self.temperature + 0.1).min(2.0); false }
                KeyCode::Down => { self.temperature = (self.temperature - 0.1).max(0.0); false }
                KeyCode::Left => { self.top_p = (self.top_p - 0.05).max(0.0); false }
                KeyCode::Right => { self.top_p = (self.top_p + 0.05).min(1.0); false }
                KeyCode::Tab => { self.focus = Focus::ApiConfig; self.status = format!("Focus: {} — ↑↓ to view APIs", self.focus.label()); false }
                KeyCode::Esc => true,
                _ => false,
            },
            Focus::ApiConfig => match key {
                KeyCode::Up => { if self.selected_api > 0 { self.selected_api -= 1; } false }
                KeyCode::Down => { if self.selected_api + 1 < self.apis.len() { self.selected_api += 1; } false }
                KeyCode::Tab => { self.focus = Focus::Input; self.status = "Focus: Input".into(); false }
                KeyCode::Esc => true,
                _ => false,
            },
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent, areas: &ClickAreas) {
        // Mode bar click
        if areas.mode_bar_contains(event.column, event.row) {
            self.focus = Focus::ModeBar;
            // Determine which mode was clicked based on x position
            let mode_widths: Vec<usize> = Mode::all().iter().map(|m| m.label().len() + 4).collect();
            let mut x_start: u16 = areas.mode_bar.x + 1;
            for (i, w) in mode_widths.iter().enumerate() {
                if event.column >= x_start && event.column < x_start + *w as u16 {
                    self.mode = Mode::all()[i];
                    self.status = format!("Mode: {} — {}", self.mode.label(), self.mode.description());
                    return;
                }
                x_start += *w as u16;
            }
            return;
        }

        // Model list click
        if areas.model_list_contains(event.column, event.row) {
            self.focus = Focus::ModelList;
            if event.row > areas.model_list.y {
                let row = event.row - areas.model_list.y - 1;
                if (row as usize) < self.models.len() {
                    self.selected_model = row as usize;
                    self.status = format!("Model: {}", self.models[self.selected_model].name);
                }
            }
        }

        // Agent list click
        if areas.agent_list_contains(event.column, event.row) {
            self.focus = Focus::AgentList;
            if event.row > areas.agent_list.y {
                let row = event.row - areas.agent_list.y - 1;
                let agent_idx = (row / 2) as usize;
                if agent_idx < self.agents.len() {
                    self.selected_agent = agent_idx;
                    self.status = format!("Agent: {}", self.agents[self.selected_agent].name);
                }
            }
            return;
        }

        // Input click
        if areas.input_contains(event.column, event.row) {
            self.focus = Focus::Input;
            self.status = "Focus: Input — type and Enter".into();
            return;
        }

        // Params click
        if areas.params_contains(event.column, event.row) {
            self.focus = Focus::Params;
            self.status = "Focus: Params — ↑↓ temp, ←→ top_p".into();
            return;
        }

        // API config click
        if areas.api_config_contains(event.column, event.row) {
            self.focus = Focus::ApiConfig;
            if event.row > areas.api_config.y {
                let row = event.row - areas.api_config.y - 1;
                if (row as usize) < self.apis.len() {
                    self.selected_api = row as usize;
                    self.status = format!("API: {} ({})", self.apis[self.selected_api].name,
                        if self.apis[self.selected_api].configured { "✓ configured" } else { "✗ not set" });
                }
            }
        }
    }
}

// ── Click Areas ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct ClickAreas {
    mode_bar: Rect,
    model_list: Rect,
    agent_list: Rect,
    input: Rect,
    params: Rect,
    api_config: Rect,
}

impl ClickAreas {
    fn mode_bar_contains(&self, x: u16, y: u16) -> bool {
        self.mode_bar.x <= x && x < self.mode_bar.x + self.mode_bar.width
            && self.mode_bar.y <= y && y < self.mode_bar.y + self.mode_bar.height
    }

    fn model_list_contains(&self, x: u16, y: u16) -> bool {
        self.model_list.x <= x && x < self.model_list.x + self.model_list.width
            && self.model_list.y <= y && y < self.model_list.y + self.model_list.height
    }

    fn agent_list_contains(&self, x: u16, y: u16) -> bool {
        self.agent_list.x <= x && x < self.agent_list.x + self.agent_list.width
            && self.agent_list.y <= y && y < self.agent_list.y + self.agent_list.height
    }

    fn input_contains(&self, x: u16, y: u16) -> bool {
        self.input.x <= x && x < self.input.x + self.input.width
            && self.input.y <= y && y < self.input.y + self.input.height
    }

    fn params_contains(&self, x: u16, y: u16) -> bool {
        self.params.x <= x && x < self.params.x + self.params.width
            && self.params.y <= y && y < self.params.y + self.params.height
    }

    fn api_config_contains(&self, x: u16, y: u16) -> bool {
        self.api_config.x <= x && x < self.api_config.x + self.api_config.width
            && self.api_config.y <= y && y < self.api_config.y + self.api_config.height
    }
}

// ── Run Dashboard ───────────────────────────────────────────────────────────

pub async fn run_dashboard(ollama_url: &str) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut d = Dashboard::new(ollama_url);
    d.refresh_models().await;
    d.refresh_system().await;
    d.status = "Ready. Tab=switch | ←→=mode | ↑↓=select | Enter=execute | Click=focus | Esc=quit".into();

    let (result_tx, mut result_rx) = mpsc::channel::<DashboardResult>(10);
    let mut last_refresh = Instant::now();
    let mut click_areas = ClickAreas {
        mode_bar: Rect::default(),
        model_list: Rect::default(),
        agent_list: Rect::default(),
        input: Rect::default(),
        params: Rect::default(),
        api_config: Rect::default(),
    };

    loop {
        // Check for async results
        if let Ok(res) = result_rx.try_recv() {
            d.output = res.response.clone();
            d.metrics = Some(res.metrics.clone());
            d.status = format!(
                "✓ {} | {} | {:.1} tok/s | {}ms | {} out | {} in",
                res.mode_label, res.model,
                res.metrics.tokens_per_second, res.metrics.latency_ms,
                res.metrics.output_tokens, res.metrics.input_tokens
            );
            d.history.push(HistoryEntry {
                mode: res.mode, agent: res.agent, model: res.model.clone(),
                prompt: d.input.clone(), response: res.response, metrics: res.metrics,
            });
            d.input.clear();
            d.exec_state = ExecState::Done;
            if d.mode == Mode::Autonomous { d.autonomous_cycles += 1; }
        }

        // Periodic refresh
        if last_refresh.elapsed() > Duration::from_secs(5) {
            d.refresh_system().await;
            last_refresh = Instant::now();
        }

        // Tick counter for spinner animation
        d.tick += 1;

        // Draw
        terminal.draw(|f| {
            click_areas = draw_dashboard(f, &d);
        })?;

        // Poll for input (non-blocking)
        if poll(Duration::from_millis(100))? {
            match read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press { continue; }

                    if key.code == KeyCode::Enter && d.focus == Focus::Input {
                        if !d.input.trim().is_empty() && !d.exec_state.is_running() {
                            execute_action(&mut d, result_tx.clone()).await;
                        }
                        continue;
                    }

                    if d.handle_key(key.code) { break; }
                }
                Event::Mouse(mouse) => {
                    if mouse.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left) {
                        d.handle_mouse(mouse, &click_areas);
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.backend_mut().flush()?;
    Ok(())
}

struct DashboardResult {
    mode: Mode,
    mode_label: String,
    agent: String,
    model: String,
    response: String,
    metrics: ChatMetrics,
}

async fn execute_action(d: &mut Dashboard, tx: mpsc::Sender<DashboardResult>) {
    let prompt = d.input.clone();
    let model = d.current_model();
    let agent_name = d.current_agent_name().to_string();
    let mode = d.mode;
    let mode_label = mode.label().to_string();
    let temp = d.temperature;
    let top_p_val = d.top_p;
    let top_k_val = d.top_k;
    let ctx = d.num_ctx;

    d.exec_state = ExecState::Running { started: Instant::now(), mode_label: mode_label.clone() };
    d.status = format!("⠿ Executing {} via {} ({})...", mode.label(), agent_name, model);

    match mode {
        Mode::Chat => {
            let ollama = d.ollama.clone();
            tokio::spawn(async move {
                let messages = vec![Message::system("You are a helpful coding assistant."), Message::user(&prompt)];
                let options = ChatOptions { temperature: temp, top_p: top_p_val, top_k: top_k_val, num_ctx: ctx };
                match ollama.chat_with_metrics(&model, &messages, options).await {
                    Ok((resp, metrics)) => {
                        let _ = tx.send(DashboardResult { mode, mode_label, agent: agent_name, model, response: resp, metrics }).await;
                    }
                    Err(e) => { tracing::error!("chat error: {e}"); }
                }
            });
        }
        Mode::Code | Mode::Review | Mode::Test | Mode::Refactor | Mode::Autonomous => {
            let orchestrator = d.orchestrator.clone();
            let start = Instant::now();
            if mode == Mode::Autonomous { d.autonomous_running = true; }
            tokio::spawn(async move {
                let req = CodeRequest { task: prompt.clone(), language: None, files: Vec::new(), model_override: Some(model.clone()) };
                let result = match mode {
                    Mode::Code | Mode::Autonomous => orchestrator.code(req).await.map(|r| r.output),
                    Mode::Review => orchestrator.review(vec![prompt.clone()]).await.map(|r| r.output),
                    Mode::Test => orchestrator.test(vec![prompt.clone()], None).await.map(|r| r.output),
                    Mode::Refactor => orchestrator.refactor(vec![prompt.clone()], &prompt).await.map(|r| r.output),
                    _ => unreachable!(),
                };
                let latency_ms = start.elapsed().as_millis() as u64;
                match result {
                    Ok(resp) => {
                        let metrics = ChatMetrics::from_request(&model, &[Message::user(&prompt)], &ChatOptions { temperature: temp, top_p: top_p_val, top_k: top_k_val, num_ctx: ctx }, latency_ms, &resp);
                        let _ = tx.send(DashboardResult { mode, mode_label, agent: agent_name, model, response: resp, metrics }).await;
                    }
                    Err(e) => { tracing::error!("orchestrator error: {e}"); }
                }
            });
        }
    }
    d.input.clear();
}

// ── Draw ────────────────────────────────────────────────────────────────────

fn draw_dashboard(f: &mut ratatui::Frame<'_>, d: &Dashboard) -> ClickAreas {
    let area = f.area();

    let top = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(area);

    // ── Mode bar with branding ──
    let mode_spans: Vec<Span> = Mode::all()
        .iter()
        .map(|m| {
            if *m == d.mode {
                Span::styled(format!(" [{label}] ", label = m.label()), Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD))
            } else {
                Span::styled(format!("  {label}  ", label = m.label()), Style::default().fg(Color::DarkGray))
            }
        })
        .collect();
    let mut mode_line_spans = mode_spans;
    // Add branding on the right
    let brand_span = Span::styled(format!(" {brand} ", brand = EDGESEARCH_BRAND), Style::default().fg(Color::White).bg(Color::DarkGray).add_modifier(Modifier::BOLD));
    mode_line_spans.push(Span::raw("  "));
    mode_line_spans.push(brand_span);

    let mode_bar = Paragraph::new(Text::from(Line::from(mode_line_spans)))
        .block(
            Block::default()
                .title(format!("Mode (←→ or click) | Agent: {} | Model: {}", d.current_agent_name(), d.current_model()))
                .borders(Borders::ALL)
                .border_style(if d.focus == Focus::ModeBar { Style::default().fg(Color::Cyan) } else { Style::default() }),
        );
    f.render_widget(mode_bar, top[0]);

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(42), Constraint::Min(60)].as_ref())
        .split(top[1]);

    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(7), Constraint::Length(10), Constraint::Length(7), Constraint::Length(6), Constraint::Length(10)].as_ref())
        .split(main[0]);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(7), Constraint::Length(8), Constraint::Length(3), Constraint::Length(1)].as_ref())
        .split(main[1]);

    // ── Models ──
    let model_items: Vec<ListItem> = d.models.iter().enumerate().map(|(i, m)| {
        let prefix = if i == d.selected_model { "▶ " } else { "  " };
        let loaded = if d.models_loaded.contains(&m.name) { " ●" } else { "" };
        ListItem::new(format!("{prefix}{name} ({size}, {quant}){loaded}", name = m.name, size = m.size, quant = m.quantization))
    }).collect();
    let model_list = List::new(model_items)
        .block(Block::default().title("Models (↑↓ or click)").borders(Borders::ALL)
            .border_style(if d.focus == Focus::ModelList { Style::default().fg(Color::Cyan) } else { Style::default() }))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(model_list, left[0]);

    // ── Agents ──
    let agent_items: Vec<ListItem> = d.agents.iter().enumerate().map(|(i, a)| {
        let prefix = if i == d.selected_agent { "▶ " } else { "  " };
        let model_tag = if a.model.is_empty() { "auto" } else { a.model };
        ListItem::new(format!("{prefix}{name} [{model}]\n   {desc}", name = a.name, model = model_tag, desc = a.description))
    }).collect();
    let agent_list = List::new(agent_items)
        .block(Block::default().title("Agents (↑↓ or click)").borders(Borders::ALL)
            .border_style(if d.focus == Focus::AgentList { Style::default().fg(Color::Magenta) } else { Style::default() }))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(agent_list, left[1]);

    // ── Params ──
    let params_text = vec![
        Line::from(vec![Span::styled("Temp: ", Style::default().fg(Color::Yellow)), Span::raw(format!("{:.1}", d.temperature)), Span::raw(" (↑↓)")]),
        Line::from(vec![Span::styled("TopP: ", Style::default().fg(Color::Yellow)), Span::raw(format!("{:.2}", d.top_p)), Span::raw(" (←→)")]),
        Line::from(vec![Span::styled("TopK: ", Style::default().fg(Color::Yellow)), Span::raw(format!("{}", d.top_k))]),
        Line::from(vec![Span::styled("Ctx:  ", Style::default().fg(Color::Yellow)), Span::raw(format!("{} tok", d.num_ctx))]),
    ];
    let params = Paragraph::new(Text::from(params_text))
        .block(Block::default().title("Params (click to focus)").borders(Borders::ALL)
            .border_style(if d.focus == Focus::Params { Style::default().fg(Color::Cyan) } else { Style::default() }));
    f.render_widget(params, left[2]);

    // ── System ──
    let vram_pct = if d.vram_total_mb > 0 { d.vram_used_mb as f64 / d.vram_total_mb as f64 * 100.0 } else { 0.0 };
    let vram_filled = (vram_pct / 100.0 * 20.0) as usize;
    let vram_bar: String = "█".repeat(vram_filled) + &"░".repeat(20 - vram_filled);
    let sys_text = vec![
        Line::from(vec![Span::styled("VRAM: ", Style::default().fg(Color::Green)), Span::raw(format!("{used}MB/{total}MB ({pct:.0}%)", used = d.vram_used_mb, total = d.vram_total_mb, pct = vram_pct))]),
        Line::from(format!(" {vram_bar}")),
        Line::from(vec![Span::styled("Loaded: ", Style::default().fg(Color::Green)), Span::raw(if d.models_loaded.is_empty() { "(none)".into() } else { d.models_loaded.join(", ") })]),
    ];
    f.render_widget(Paragraph::new(Text::from(sys_text)).block(Block::default().title("System").borders(Borders::ALL)), left[3]);

    // ── API Config ──
    let api_items: Vec<ListItem> = d.apis.iter().enumerate().map(|(i, a)| {
        let prefix = if i == d.selected_api { "▶ " } else { "  " };
        let status_icon = if a.configured { "✓" } else { "✗" };
        let status_color = if a.configured { Color::Green } else { Color::Red };
        ListItem::new(vec![
            Line::from(vec![
                Span::raw(prefix),
                Span::styled(a.name.to_string(), Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(status_icon, Style::default().fg(status_color)),
                Span::raw(format!("  {desc}", desc = a.description)),
            ]),
            Line::from(vec![
                Span::styled(format!("   env: {env}", env = a.env_var), Style::default().fg(Color::DarkGray)),
            ]),
        ])
    }).collect();
    let api_list = List::new(api_items)
        .block(Block::default().title("External APIs (↑↓ or click)").borders(Borders::ALL)
            .border_style(if d.focus == Focus::ApiConfig { Style::default().fg(Color::Yellow) } else { Style::default() }))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(api_list, left[4]);

    // ── Input with execution indicator ──
    let input_hint = match d.mode {
        Mode::Chat => "chat message",
        Mode::Code => "code task description",
        Mode::Review => "file path to review",
        Mode::Test => "file path to test",
        Mode::Refactor => "file path + refactor goal",
        Mode::Autonomous => "task for autonomous loop",
    };
    let input_style = if d.focus == Focus::Input { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Gray) };

    // Execution indicator prefix
    let spinner = d.exec_state.spinner();
    let exec_prefix = match &d.exec_state {
        ExecState::Running { mode_label: _, .. } => format!("{} ", spinner),
        ExecState::Done => "✓ ".into(),
        ExecState::Idle => "○ ".into(),
    };
    let exec_color = match &d.exec_state {
        ExecState::Running { .. } => Color::Yellow,
        ExecState::Done => Color::Green,
        ExecState::Idle => Color::DarkGray,
    };

    let input_text = if d.exec_state.is_running() {
        format!("{} {} (running {:.1}s)...", spinner, d.input, d.exec_state.elapsed_secs())
    } else {
        format!("{}> {}", exec_prefix, d.input)
    };

    f.render_widget(
        Paragraph::new(input_text)
            .block(Block::default().title(format!("Input ({input_hint}) — Enter=execute | Click to focus")).borders(Borders::ALL)
                .border_style(input_style)),
        right[0],
    );

    // Render spinner color overlay
    if d.exec_state.is_running() {
        let spinner_area = Rect { x: right[0].x + 1, y: right[0].y + 1, width: 1, height: 1 };
        f.render_widget(Paragraph::new(Span::styled(format!("{}", spinner), Style::default().fg(exec_color).add_modifier(Modifier::BOLD))), spinner_area);
    }

    // ── Output ──
    let output_text = if d.output.is_empty() {
        if d.exec_state.is_running() {
            Text::raw(format!("⠿ Processing... {:.1}s elapsed", d.exec_state.elapsed_secs()))
        } else {
            Text::raw("(output will appear here)")
        }
    } else {
        Text::raw(d.output.clone())
    };
    f.render_widget(
        Paragraph::new(output_text).wrap(Wrap { trim: false })
            .block(Block::default().title("Output").borders(Borders::ALL).border_style(Style::default().fg(Color::Green))),
        right[1],
    );

    // ── Stats ──
    let stats_text = if let Some(ref m) = d.metrics {
        let tps_color = if m.tokens_per_second > 20.0 { Color::Green } else if m.tokens_per_second > 5.0 { Color::Yellow } else { Color::Red };
        vec![
            Line::from(vec![Span::styled("Model:     ", Style::default().fg(Color::Cyan)), Span::raw(&m.model)]),
            Line::from(vec![Span::styled("Latency:   ", Style::default().fg(Color::Cyan)), Span::raw(format!("{} ms", m.latency_ms))]),
            Line::from(vec![Span::styled("Tokens/s:  ", Style::default().fg(Color::Cyan)), Span::styled(format!("{:.1}", m.tokens_per_second), Style::default().fg(tps_color).add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::styled("Out tokens:", Style::default().fg(Color::Cyan)), Span::raw(format!("{}", m.output_tokens)), Span::raw("  "), Span::styled("In: ", Style::default().fg(Color::Cyan)), Span::raw(format!("{}", m.input_tokens))]),
            Line::from(vec![Span::styled("Context:   ", Style::default().fg(Color::Cyan)), Span::raw(format!("{} tok", m.num_ctx)), Span::raw("  "), Span::styled("Temp: ", Style::default().fg(Color::Cyan)), Span::raw(format!("{:.1}", m.temperature))]),
        ]
    } else {
        vec![Line::from("(no metrics yet — send a message)")]
    };
    f.render_widget(Paragraph::new(Text::from(stats_text)).block(Block::default().title("Metrics").borders(Borders::ALL)), right[2]);

    // ── History ──
    let history_text: Vec<Line> = if d.history.is_empty() {
        vec![Line::from("(no history)")]
    } else {
        d.history.iter().rev().take(3).map(|h| {
            Line::from(vec![
                Span::styled(format!("[{}] ", h.mode.label()), Style::default().fg(Color::Magenta)),
                Span::styled(format!("{} ", h.agent), Style::default().fg(Color::Cyan)),
                Span::raw(h.prompt.chars().take(40).collect::<String>()),
                Span::styled(format!(" → {:.1} tok/s", h.metrics.tokens_per_second), Style::default().fg(Color::DarkGray)),
            ])
        }).collect()
    };
    f.render_widget(Paragraph::new(Text::from(history_text)).block(Block::default().title("History").borders(Borders::ALL)), right[3]);

    // ── Status bar with execution indicator ──
    let auto_indicator = if d.autonomous_running { format!(" AUTO[{}c] | ", d.autonomous_cycles) } else { " ".into() };
    let exec_indicator = match &d.exec_state {
        ExecState::Running { mode_label: _, .. } => format!("{} {:.1}s | ", d.exec_state.spinner(), d.exec_state.elapsed_secs()),
        ExecState::Done => "✓ | ".into(),
        ExecState::Idle => "".into(),
    };
    let status_style = match &d.exec_state {
        ExecState::Running { .. } => Style::default().fg(Color::Black).bg(Color::Yellow),
        ExecState::Done => Style::default().fg(Color::Black).bg(Color::Green),
        ExecState::Idle => Style::default().fg(Color::Black).bg(Color::Yellow),
    };
    f.render_widget(
        Paragraph::new(format!("{}{}{} ", auto_indicator, exec_indicator, d.status))
            .style(status_style),
        right[4],
    );

    ClickAreas {
        mode_bar: top[0],
        model_list: left[0],
        agent_list: left[1],
        input: right[0],
        params: left[2],
        api_config: left[4],
    }
}