// Standalone TUI dashboard — reads live from Postgres (scoped_edits, flagged_edits,
// article_metrics). Runs as its own binary alongside the ingestion program
// (`cargo run` runs main.rs; `cargo run --bin tui` runs this). Polls on a timer rather
// than sharing in-process state, so it can never slow down or crash the ingestion loop.

use std::io;
use std::str::FromStr;
use std::time::Duration;

use crossterm::event::{self, Event as CEvent, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Terminal;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::Row;

struct FeedRow {
    title: String,
    delta: i64,
    timestamp: i64,
    flagged: bool,
}

struct WatchRow {
    title: String,
    suspicious_edit_count: i32,
    unique_ip_count: i32,
    consecutive_reverts: i32,
}

struct FlagRow {
    title: String,
    reason: String,
    timestamp: i64,
}

impl WatchRow {
    // Same weighting idea discussed for SuspicionScore: log-dampened edit volume,
    // IP spread, reverts weighted highest. Purely a display ranking, not written back.
    fn severity(&self) -> f64 {
        let edit_component = ((self.suspicious_edit_count as f64) + 1.0).ln() * 2.0;
        let ip_component = self.unique_ip_count as f64 * 3.0;
        let revert_component = self.consecutive_reverts as f64 * 4.0;
        edit_component + ip_component + revert_component
    }

    fn breakdown(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("suspicious_edit_volume", ((self.suspicious_edit_count as f64) + 1.0).ln() * 2.0),
            ("unique_ip_spread", self.unique_ip_count as f64 * 3.0),
            ("consecutive_reverts", self.consecutive_reverts as f64 * 4.0),
        ]
    }
}

async fn connect_db() -> Result<sqlx::PgPool, Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL")?;
    let connect_options = PgConnectOptions::from_str(&db_url)?.statement_cache_capacity(0);
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect_with(connect_options)
        .await?;
    Ok(pool)
}

async fn fetch_feed(pool: &sqlx::PgPool) -> Vec<FeedRow> {
    let rows = sqlx::query(
        "SELECT title, delta, timestamp,
                EXISTS(SELECT 1 FROM flagged_edits f WHERE f.title = scoped_edits.title
                       AND f.timestamp = scoped_edits.timestamp) AS flagged
         FROM scoped_edits
         ORDER BY timestamp DESC
         LIMIT 15",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| FeedRow {
            title: r.get("title"),
            delta: r.get("delta"),
            timestamp: r.get("timestamp"),
            flagged: r.get("flagged"),
        })
        .collect()
}

async fn fetch_watchlist(pool: &sqlx::PgPool) -> Vec<WatchRow> {
    let rows = sqlx::query(
        "SELECT title, suspicious_edit_count, unique_ip_count, consecutive_reverts
         FROM article_metrics
         ORDER BY (suspicious_edit_count * 2 + unique_ip_count * 3 + consecutive_reverts * 4) DESC
         LIMIT 10",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| WatchRow {
            title: r.get("title"),
            suspicious_edit_count: r.get("suspicious_edit_count"),
            unique_ip_count: r.get("unique_ip_count"),
            consecutive_reverts: r.get("consecutive_reverts"),
        })
        .collect()
}

async fn fetch_flagged(pool: &sqlx::PgPool) -> Vec<FlagRow> {
    let rows = sqlx::query("SELECT title, reason, timestamp FROM flagged_edits ORDER BY timestamp DESC LIMIT 8")
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    rows.into_iter()
        .map(|r| FlagRow {
            title: r.get("title"),
            reason: r.get("reason"),
            timestamp: r.get("timestamp"),
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let pool = connect_db().await?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut feed = fetch_feed(&pool).await;
    let mut watchlist = fetch_watchlist(&pool).await;
    let mut flagged = fetch_flagged(&pool).await;
    let mut selected: usize = 0;
    let mut list_state = ListState::default();
    list_state.select(Some(0));

    let mut last_refresh = std::time::Instant::now();
    let refresh_every = Duration::from_secs(5);

    loop {
        terminal.draw(|f| {
            let size = f.area();
            let outer = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(10), Constraint::Length(10)])
                .split(size);

            let top = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(outer[0]);

            // Live feed
            let feed_items: Vec<ListItem> = feed
                .iter()
                .map(|r| {
                    let dcolor = if r.delta < 0 { Color::Red } else { Color::Green };
                    let badge = if r.flagged {
                        Span::styled(" FLAGGED ", Style::default().bg(Color::Red).fg(Color::Black))
                    } else {
                        Span::styled(" clear ", Style::default().fg(Color::DarkGray))
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("{:>7} ", r.delta), Style::default().fg(dcolor)),
                        badge,
                        Span::raw(format!(" {}", r.title)),
                    ]))
                })
                .collect();
            f.render_widget(
                List::new(feed_items).block(Block::default().borders(Borders::ALL).title("live feed")),
                top[0],
            );

            // Right column: watchlist + detail
            let right = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(top[1]);

            let watch_items: Vec<ListItem> = watchlist
                .iter()
                .map(|w| {
                    let sev = w.severity();
                    let color = if sev >= 10.0 { Color::Red } else if sev >= 5.0 { Color::Yellow } else { Color::Gray };
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("{:<28}", w.title), Style::default().fg(Color::White)),
                        Span::styled(format!(" sev {:.1}", sev), Style::default().fg(color)),
                    ]))
                })
                .collect();
            f.render_stateful_widget(
                List::new(watch_items)
                    .block(Block::default().borders(Borders::ALL).title("top watched pages (↑↓ select)"))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED)),
                right[0],
                &mut list_state,
            );

            // Score breakdown for selected row
            let detail_lines: Vec<Line> = if let Some(w) = watchlist.get(selected) {
                let mut lines = vec![Line::from(Span::styled(
                    format!("{} — score {:.1}", w.title, w.severity()),
                    Style::default().add_modifier(Modifier::BOLD),
                ))];
                for (label, val) in w.breakdown() {
                    lines.push(Line::from(format!("{label:<24} {val:.1}")));
                }
                lines
            } else {
                vec![Line::from("No watched pages yet.")]
            };
            f.render_widget(
                Paragraph::new(detail_lines).block(Block::default().borders(Borders::ALL).title("score breakdown")),
                right[1],
            );

            // Flagged edits log
            let flag_lines: Vec<Line> = flagged
                .iter()
                .map(|fl| {
                    Line::from(vec![
                        Span::styled("🚩 ", Style::default().fg(Color::Red)),
                        Span::raw(format!("{} | {} | t={}", fl.title, fl.reason, fl.timestamp)),
                    ])
                })
                .collect();
            f.render_widget(
                Paragraph::new(flag_lines).block(Block::default().borders(Borders::ALL).title("flagged edits log")),
                outer[1],
            );

            let _ = Rect::default(); // keep import used if layout trimmed later
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let CEvent::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Down => {
                        if !watchlist.is_empty() {
                            selected = (selected + 1).min(watchlist.len() - 1);
                            list_state.select(Some(selected));
                        }
                    }
                    KeyCode::Up => {
                        selected = selected.saturating_sub(1);
                        list_state.select(Some(selected));
                    }
                    _ => {}
                }
            }
        }

        if last_refresh.elapsed() >= refresh_every {
            feed = fetch_feed(&pool).await;
            watchlist = fetch_watchlist(&pool).await;
            flagged = fetch_flagged(&pool).await;
            if selected >= watchlist.len() && !watchlist.is_empty() {
                selected = watchlist.len() - 1;
                list_state.select(Some(selected));
            }
            last_refresh = std::time::Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
