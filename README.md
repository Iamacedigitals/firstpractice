<div align="center">

# 🛰️ firstpractice

### A real-time Wikipedia edit watcher & anomaly detector, written in Rust

Streams live edits from Wikimedia's `recentchange` feed, parses them into typed Rust structs, flags suspicious behaviour (mass deletions, anonymous edit bursts, revert wars, statistical outliers), stores everything in PostgreSQL, and shows it in a terminal dashboard.

![Rust](https://img.shields.io/badge/Rust-2024_edition-000000?logo=rust&logoColor=white)
![Tokio](https://img.shields.io/badge/async-tokio-1f6feb)
![PostgreSQL](https://img.shields.io/badge/database-PostgreSQL-336791?logo=postgresql&logoColor=white)
![sqlx](https://img.shields.io/badge/sqlx-0.8-orange)
![TUI](https://img.shields.io/badge/TUI-ratatui-8A2BE2)
![Status](https://img.shields.io/badge/status-learning_project-yellow)

</div>

---

## 📖 Table of Contents

- [Why this project exists](#-why-this-project-exists)
- [Features](#-features)
- [How it works](#-how-it-works)
- [Tech stack](#-tech-stack)
- [Getting started](#-getting-started)
- [Configuration](#-configuration)
- [Usage](#-usage)
- [Project structure](#-project-structure)
- [The build journey](#-the-build-journey)
- [Anomaly detection design](#-anomaly-detection-design)
- [Database](#-database)
- [Roadmap](#-roadmap)
- [Lessons learned](#-lessons-learned)
- [Contributing](#-contributing)
- [Acknowledgements](#-acknowledgements)

---

## 🎯 Why this project exists

`firstpractice` started as a Rust practice exercise: *"can I read a live firehose of events and make sense of it?"* Wikipedia is a perfect playground. Thousands of edits arrive every minute, the data is public and messy, and "what counts as suspicious?" is a genuinely interesting question.

The project grew in deliberate steps, from a bare stream reader into a small anomaly-detection pipeline with persistence and a terminal UI. Each stage exists to practise a specific part of Rust and systems design.

| 🎓 Skill practised | 🧩 Where it shows up |
| --- | --- |
| Async Rust | `tokio` runtime, long-lived SSE connection |
| Strong typing & parsing | `serde` → `WikiEdit` struct |
| Error handling | Reconnect logic, malformed events |
| Data structures | `HashSet`, per-article state maps |
| Statistics in code | Z-score outlier filter |
| Databases | `sqlx` + Postgres migrations |
| CLI design | `clap` derive subcommands |
| Terminal UI | `ratatui` + `crossterm` |

---

## ✨ Features

- 📡 **Live ingestion** from the Wikimedia `recentchange` Server-Sent Events stream
- 🧱 **Typed parsing** of raw JSON events into a `WikiEdit` struct
- 🚨 **Anomaly filters** for large deletions, anonymous (IP) edits, edit bursts, and statistical outliers
- 📊 **Per-article tracking** via an `ArticleVulnerabilityMetrics` struct (suspicious edit count, unique IP editors, consecutive reverts)
- 🗄️ **PostgreSQL persistence** with versioned `sqlx` migrations
- 🖥️ **Terminal dashboard** built with `ratatui`
- ⚙️ **CLI and `.env` configuration** via `clap` and `dotenvy`

---

## 🔬 How it works

```mermaid
flowchart LR
    A[("🌐 Wikimedia<br/>recentchange SSE")] -->|text/event-stream| B["📡 Stream reader<br/>reqwest-eventsource"]
    B -->|raw JSON| C["🧱 Parser<br/>serde → WikiEdit"]
    C --> D{"🚨 Anomaly filters"}
    D -->|large deletion| E["⚠️ Flag"]
    D -->|anonymous edit| E
    D -->|edit burst| E
    D -->|z-score &gt; 3σ| E
    D -->|normal| F["✅ Pass-through"]
    E --> G["📊 ArticleVulnerabilityMetrics"]
    F --> G
    G --> H[("🗄️ PostgreSQL<br/>sqlx")]
    G --> I["🖥️ ratatui dashboard"]
```

### Event lifecycle

```mermaid
sequenceDiagram
    participant W as Wikimedia stream
    participant R as Reader (tokio task)
    participant P as Parser
    participant F as Filters
    participant DB as PostgreSQL
    participant UI as TUI

    W->>R: data: {"type":"edit", ...}
    R->>P: raw event string
    P->>F: WikiEdit
    F->>F: update per-article metrics
    F->>DB: persist edit + flags
    F->>UI: push update
    Note over R,W: On disconnect, reconnect and keep reading
```

---

## 🧰 Tech stack

| Crate | Version | Role |
| --- | --- | --- |
| [`tokio`](https://crates.io/crates/tokio) | 1 (full) | Async runtime |
| [`reqwest`](https://crates.io/crates/reqwest) | 0.12 | HTTP client (`rustls-tls`, no OpenSSL needed) |
| [`reqwest-eventsource`](https://crates.io/crates/reqwest-eventsource) | 0.6 | SSE client on top of reqwest |
| [`serde`](https://crates.io/crates/serde) / [`serde_json`](https://crates.io/crates/serde_json) | 1 | Deserialize events into typed structs |
| [`sqlx`](https://crates.io/crates/sqlx) | 0.8 | Async Postgres driver and migrations |
| [`chrono`](https://crates.io/crates/chrono) | 0.4 | Timestamps |
| [`clap`](https://crates.io/crates/clap) | 4 (derive) | Command-line interface |
| [`dotenvy`](https://crates.io/crates/dotenvy) | 0.15 | Load `.env` configuration |
| [`ratatui`](https://crates.io/crates/ratatui) | 0.29 | Terminal UI widgets |
| [`crossterm`](https://crates.io/crates/crossterm) | 0.28 | Terminal backend and input events |
| [`tokio-tungstenite`](https://crates.io/crates/tokio-tungstenite) | 0.24 | WebSocket client (available for experimenting with other feeds) |
| [`futures-util`](https://crates.io/crates/futures-util) | 0.3 | Stream combinators (`StreamExt`) |

---

## 🚀 Getting started

### Prerequisites

- 🦀 **Rust 1.85 or newer** (the crate uses `edition = "2024"`)
- 🐘 **PostgreSQL** 14+ (local install or Docker)
- 🧰 **sqlx-cli** for running migrations

```bash
rustup update stable
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

### 1. Clone

```bash
git clone https://github.com/Iamacedigitals/firstpractice.git
cd firstpractice
```

### 2. Start Postgres (Docker option)

```bash
docker run --name wiki-pg \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=firstpractice \
  -p 5432:5432 -d postgres:16
```

### 3. Configure

```bash
cp .env.example .env   # or create .env by hand, see Configuration below
```

### 4. Run migrations

```bash
sqlx migrate run
```

### 5. Run it

```bash
cargo run --release
```

> 💡 Use `--release` for the live stream. The stream is busy, and optimized builds keep the parser and filters comfortably ahead of it.

---

## ⚙️ Configuration

Settings are read from environment variables (loaded from `.env` by `dotenvy`).

| Variable | Required | Example | Purpose |
| --- | :---: | --- | --- |
| `DATABASE_URL` | ✅ | `postgres://postgres:postgres@localhost:5432/firstpractice` | Postgres connection string used by `sqlx` |

> ⚠️ Never commit `.env`. It is listed in `.gitignore`; keep it that way.

**A note on the Wikimedia stream:** Wikimedia asks API clients to send a descriptive `User-Agent` header (tool name plus contact info). Set one on your `reqwest` client so your traffic is identifiable and polite.

---

## 🕹️ Usage

The CLI is built with `clap`. Run `--help` to see the exact commands available in your build:

```bash
cargo run -- --help
```

Typical flow:

```bash
# Stream, filter and store edits
cargo run --release

# Open the terminal dashboard
cargo run --release -- --help   # check the subcommand for the TUI in your build
```

### Terminal dashboard keys

| Key | Action |
| --- | --- |
| `q` | Quit |
| `↑` / `↓` | Move through the list |

*(Update this table to match the key bindings you implement.)*

---

## 🗂️ Project structure

```text
firstpractice/
├── migrations/        # sqlx SQL migrations (schema versioning)
├── src/               # application source
├── .gitignore
├── Cargo.toml         # dependencies and crate metadata
└── Cargo.lock         # pinned dependency versions
```

<details>
<summary><b>🔎 Suggested module layout as the project grows</b></summary>

<br>

| Module | Responsibility |
| --- | --- |
| `main.rs` | Entry point, CLI parsing, task wiring |
| `stream.rs` | SSE connection, reconnect handling |
| `model.rs` | `WikiEdit` and related serde types |
| `filters.rs` | Anomaly rules and z-score logic |
| `metrics.rs` | `ArticleVulnerabilityMetrics` and per-article state |
| `db.rs` | `sqlx` pool, inserts and queries |
| `tui.rs` | `ratatui` layout, rendering, input loop |

</details>

---

## 🧭 The build journey

This is the story of how the project was built, step by step. Expand each stage to see the thinking behind it.

<details>
<summary><b>🟢 Stage 1: Connect to the firehose</b></summary>

<br>

**Goal:** prove the stream can be read at all.

- Wikimedia publishes every change on `https://stream.wikimedia.org/v2/stream/recentchange` as Server-Sent Events.
- SSE is just a long-lived HTTP response where each message starts with `data:`. `reqwest-eventsource` wraps that so each message arrives as an event.
- First milestone: print raw events to the terminal and watch them scroll.

**What it taught:** how async streams work in Rust (`StreamExt::next`), and that a "connection" can silently die, so reconnect logic matters.

</details>

<details>
<summary><b>🟡 Stage 2: Turn raw JSON into a typed <code>WikiEdit</code></b></summary>

<br>

**Goal:** stop passing strings around.

- Defined a `WikiEdit` struct with `serde::Deserialize` and parsed each event into it.
- Fields of interest include the page title, the editing user, whether it is a bot edit, the old and new byte lengths, the wiki, and a timestamp.
- Optional or missing fields are modelled with `Option<T>` rather than assumed present.

**What it taught:** real-world JSON is messy. Typed structs make the messiness explicit and let the compiler guard every downstream step.

</details>

<details>
<summary><b>🟠 Stage 3: Decide what "suspicious" means</b></summary>

<br>

**Goal:** turn a stream into signals.

Candidate signals considered:

| Signal | Why it matters |
| --- | --- |
| Large byte deletion | Page blanking is a classic vandalism pattern |
| Anonymous (IP) editor | Higher base rate of abuse, not proof of it |
| Edit bursts | Many edits to one page in a short window |
| Consecutive reverts | Possible edit war |
| Statistical outliers | Edits unusually large or small compared to the norm |

The open question was how to choose thresholds from real data instead of guessing, which led to collecting data first and measuring before deciding.

</details>

<details>
<summary><b>🔴 Stage 4: Add a statistical outlier filter</b></summary>

<br>

**Goal:** a first, simple, explainable model.

- Chose a plain **z-score**: flag an edit when its size change sits more than **3 standard deviations** from the mean.
- This assumes roughly normal data, which is a known simplification (see [Anomaly detection design](#-anomaly-detection-design)).
- Starting simple keeps the logic easy to test and easy to explain before reaching for heavier models.

</details>

<details>
<summary><b>🟣 Stage 5: Track state per article</b></summary>

<br>

**Goal:** judge pages, not just single edits.

A per-article struct accumulates evidence over time:

```rust
struct ArticleVulnerabilityMetrics {
    page_id: u64,
    title: String,
    suspicious_edit_count: u32,
    unique_ip_editors: HashSet<String>,
    consecutive_reverts: u32,
}
```

A simple threshold over these fields decides when an article deserves attention. Using a `HashSet` for IP editors makes "how many *different* anonymous users touched this page?" a cheap question.

</details>

<details>
<summary><b>🔵 Stage 6: Persist with PostgreSQL</b></summary>

<br>

**Goal:** keep history so thresholds can be tuned from real data.

- `sqlx` talks to Postgres asynchronously.
- Schema changes live in `migrations/` as ordered SQL files so the database can be rebuilt from scratch with `sqlx migrate run`.
- Stored history is what makes it possible to answer "what is *normal* for this wiki?" with evidence.

</details>

<details>
<summary><b>⚫ Stage 7: Make it visible with a TUI</b></summary>

<br>

**Goal:** a live dashboard instead of scrolling logs.

- `ratatui` renders widgets (tables, lists, gauges) and `crossterm` handles the terminal and key input.
- The stream reader and the UI run as separate async tasks, communicating over a channel so a slow redraw never blocks ingestion.

</details>

---

## 🧪 Anomaly detection design

### Signals at a glance

| Rule | Type | Status |
| --- | --- | --- |
| Large deletion | Heuristic threshold | 🟢 Core idea |
| Anonymous editor | Heuristic flag | 🟢 Core idea |
| Edit burst | Windowed count | 🟡 In progress |
| Z-score outlier (3σ) | Statistical | 🟢 First model |
| Consecutive reverts | Per-article counter | 🟡 In progress |

### The z-score filter

For a value `x` (for example, the byte change of an edit), with mean `μ` and standard deviation `σ` estimated from observed data:

```text
z = (x − μ) / σ        flag if |z| > 3
```

**Why start here:** it is cheap, explainable, and a good baseline to beat.

**Known limitations (worth being upfront about):**

- Edit-size distributions are typically **heavy-tailed and skewed**, not normal, so a 3σ rule can over- or under-flag.
- Statistics computed over *all* wikis mix very different communities. Per-wiki baselines are fairer.
- Large legitimate edits (reverting vandalism, bot clean-ups) can look like outliers.

**Natural upgrades to explore:** median and MAD (robust z-score), log-scaling edit sizes before scoring, per-wiki baselines, and rolling windows so "normal" adapts over time.

### Choosing thresholds from data

1. Collect a few days of edits into Postgres.
2. Plot or tabulate the distribution of byte changes per wiki.
3. Pick thresholds at chosen percentiles (for example 99th or 99.9th).
4. Spot-check flagged edits by hand to estimate precision.
5. Repeat.

---

## 🗄️ Database

Migrations live in [`migrations/`](./migrations) and are applied with `sqlx migrate run`.

```bash
# create a new migration
sqlx migrate add create_edits_table

# apply pending migrations
sqlx migrate run

# roll back the latest (for reversible migrations)
sqlx migrate revert
```

<details>
<summary><b>📐 Document your schema here</b></summary>

<br>

Add a table like this once the schema settles:

| Table | Column | Type | Notes |
| --- | --- | --- | --- |
| `edits` | `id` | `BIGSERIAL` | Primary key |
| `edits` | `title` | `TEXT` | Page title |
| `edits` | `user_name` | `TEXT` | Editor name or IP |
| `edits` | `byte_delta` | `INTEGER` | New length minus old length |
| `edits` | `created_at` | `TIMESTAMPTZ` | Event time |

*(Replace with the real columns from your migration files.)*

</details>

---

## 🗺️ Roadmap

- [x] Read the `recentchange` SSE stream
- [x] Parse events into a typed `WikiEdit`
- [x] Z-score outlier filter
- [x] Postgres integration with migrations
- [ ] Finish edit-burst and revert-streak detection
- [ ] `ArticleVulnerabilityMetrics` threshold alerts
- [ ] Complete the `ratatui` dashboard
- [ ] Robust z-score (median/MAD) and per-wiki baselines
- [ ] Unit tests for each filter using recorded sample events
- [ ] Graceful shutdown and exponential-backoff reconnects
- [ ] GitHub Actions CI (`cargo fmt`, `clippy`, `test`)

---

## 💡 Lessons learned

| 🧠 Lesson | 📝 Takeaway |
| --- | --- |
| Types beat strings | Parsing once into `WikiEdit` removed a whole class of bugs |
| Measure before thresholding | Guessing a cutoff is worse than reading the distribution |
| Simple baselines first | A z-score is easy to reason about and gives something to improve on |
| Separate ingestion from display | Channels keep the UI from stalling the stream |
| Public data is messy | `Option` fields and defensive parsing are essential |

---

## 🤝 Contributing

This is a personal learning project, but suggestions are welcome.

1. Fork the repo
2. Create a branch: `git checkout -b feature/my-idea`
3. Run `cargo fmt && cargo clippy && cargo test`
4. Open a pull request describing the change

---

## 🙏 Acknowledgements

- [Wikimedia EventStreams](https://wikitech.wikimedia.org/wiki/Event_Platform/EventStreams) for the free public feed
- The Rust community and the authors of the crates listed above

---

<div align="center">

Built with 🦀 by [**Ace**](https://github.com/Iamacedigitals)

*Learning in public, one commit at a time.*

</div>
