use std::{
    io,
    thread,
    time::{Duration, Instant},
    process::{Command, Stdio},
    io::{BufRead, BufReader},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use crossbeam_channel::Sender;

use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    text::{Line, Span},
};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{
        enable_raw_mode, disable_raw_mode,
        EnterAlternateScreen, LeaveAlternateScreen,
    },
};

use sysinfo::{System, RefreshKind, CpuRefreshKind};

// ======================================================
// UI → Engine Commands
// ======================================================

#[derive(Debug, Clone)]
pub enum UiCommand {
    Quit,
}

// ======================================================
// CAVA STATE
// ======================================================

#[derive(Clone)]
struct CavaState {
    bars: Arc<Mutex<Vec<u8>>>,
    running: Arc<AtomicBool>,
}

fn start_cava(state: CavaState) {
    thread::spawn(move || {
        let config = format!(
            "{}/.config/cava/config_rust",
            std::env::var("HOME").unwrap_or_default()
        );

        let mut child = match Command::new("cava")
            .arg("-p")
            .arg(config)
            .stdout(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => {
                eprintln!("[ui] cava not found — soundbar disabled");
                return;
            }
        };

        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => return,
        };

        let reader = BufReader::new(stdout);

        for line in reader.lines() {
            if !state.running.load(Ordering::Relaxed) {
                break;
            }

            if let Ok(line) = line {
                let values: Vec<u8> = line
                    .split(';')
                    .filter_map(|v| v.parse::<u8>().ok())
                    .collect();

                if let Ok(mut bars) = state.bars.lock() {
                    *bars = values;
                }
            }
        }
    });
}

// ======================================================
// UI STATE
// ======================================================

struct UiState {
    sys: System,
    cpu_name: String,
    total_ram_gb: u64,
    os: String,
    kernel: String,
}

fn create_ui_state() -> UiState {
    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
    );

    sys.refresh_all();

    UiState {
        cpu_name: sys
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".into()),
        total_ram_gb: sys.total_memory() / 1024 / 1024,
        os: System::name().unwrap_or_else(|| "Unknown OS".into()),
        kernel: System::kernel_version().unwrap_or_default(),
        sys,
    }
}

// ======================================================
// THREAD ENTRY
// ======================================================

pub fn start_ui_thread(tx: Sender<UiCommand>) {
    thread::spawn(move || {
        if let Err(e) = run_ui(tx) {
            eprintln!("[ui] error: {e}");
        }
    });
}

// ======================================================
// MAIN LOOP
// ======================================================

fn run_ui(tx: Sender<UiCommand>) -> anyhow::Result<()> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = create_ui_state();

    let cava = CavaState {
        bars: Arc::new(Mutex::new(vec![0; 48])),
        running: Arc::new(AtomicBool::new(true)),
    };

    start_cava(cava.clone());

    let tick_rate = Duration::from_millis(60);
    let mut last_tick = Instant::now();

    loop {
        state.sys.refresh_cpu();

        terminal.draw(|f| draw(f, &state, &cava))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if let KeyCode::Char('q') = key.code {
                    let _ = tx.send(UiCommand::Quit);
                    break;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    cava.running.store(false, Ordering::Relaxed);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}

// ======================================================
// DRAW ROOT
// ======================================================

fn draw(
    f: &mut ratatui::Frame,
    state: &UiState,
    cava: &CavaState,
) {
    let size = f.size();

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Percentage(65),
            Constraint::Percentage(35),
        ])
        .split(size);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(rows[0]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(60),
        ])
        .split(rows[1]);

    color_panel(f, top[0]);
    system_panel(f, top[1], state);
    fetch_panel(f, bottom[0], cava);
    fortune_panel(f, bottom[1]);
}

// ======================================================
// STYLE
// ======================================================

fn window(title: &str) -> Block<'static> {
    Block::default()
        .title(Span::styled(
            format!(" {} ", title),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(180, 140, 255)))
}

// ======================================================
// PANELS
// ======================================================

fn color_panel(f: &mut ratatui::Frame, area: Rect) {
    let text = (0..8)
        .map(|i| {
            Line::from(vec![Span::styled(
                "████████████████",
                Style::default().fg(Color::Rgb(120 + i * 10, 100, 200)),
            )])
        })
        .collect::<Vec<_>>();

    f.render_widget(
        Paragraph::new(text).block(window("Color Test")),
        area,
    );
}

fn system_panel(
    f: &mut ratatui::Frame,
    area: Rect,
    state: &UiState,
) {
    let mut lines = vec![
        Line::from(format!("CPU: {}", state.cpu_name)),
        Line::from(format!("RAM: {} GB", state.total_ram_gb)),
        Line::from(format!("OS: {}", state.os)),
        Line::from(format!("Kernel: {}", state.kernel)),
    ];

    for (i, cpu) in state.sys.cpus().iter().enumerate() {
        let usage = cpu.cpu_usage();
        let filled = ((usage / 100.0) * 20.0) as usize;

        let bar = format!(
            "{}{}",
            "█".repeat(filled),
            "░".repeat(20 - filled)
        );

        lines.push(Line::from(format!(
            "C{:02} [{}] {:>5.1}%",
            i, bar, usage
        )));
    }

    f.render_widget(
        Paragraph::new(lines).block(window("System Monitor")),
        area,
    );
}

// ======================================================
// Cava
// ======================================================

fn fetch_panel(
    f: &mut ratatui::Frame,
    area: Rect,
    cava: &CavaState,
) {
    let bars = cava.bars.lock().unwrap();

    let w = area.width.saturating_sub(2) as usize;
    let h = area.height.saturating_sub(2) as usize;

    if w == 0 || h == 0 {
        return;
    }

    let spacing = w as f32 / bars.len().max(1) as f32;
    let mut lines = vec![Line::from(""); h];

    for (i, val) in bars.iter().enumerate() {
        let x = (i as f32 * spacing) as usize;
        let bh = (*val as usize * h) / 100;

        for y in 0..bh.min(h) {
            let row = h - 1 - y;

            while lines[row].spans.len() < x {
                lines[row].spans.push(Span::raw(" "));
            }

            lines[row].spans.push(
                Span::styled(
                    "▏",
                    Style::default().fg(Color::Rgb(210, 180, 255)), // pastel purple
                ),
            );
        }
    }

    f.render_widget(
        Paragraph::new(lines).block(window("CAVA")),
        area,
    );
}

fn fortune_panel(f: &mut ratatui::Frame, area: Rect) {
    let text = vec![
        Line::from("Ruischii"),
        Line::from("UwU"),
        Line::from("IDK, REZE PLS HELP."),
    ];

    f.render_widget(
        Paragraph::new(text).block(window("Author")),
        area,
    );
}
