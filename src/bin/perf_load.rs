use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use hdrhistogram::Histogram;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SCORE_TOKEN: &str = "1145141919810";
const PASSWORD: &str = "Perfpass123";
const MAX_ERROR_EXAMPLES_PER_OP: usize = 3;
const REGISTER_ATTEMPTS: usize = 8;

#[derive(Parser, Debug, Clone)]
#[command(about = "Async HTTP load tester for the local Arcaea server")]
struct Args {
    #[arg(long, default_value = "http://127.0.0.1:8090", env = "PERF_BASE_URL")]
    base_url: String,

    #[arg(long, default_value = "/yinmo/30", env = "PERF_API_PREFIX")]
    prefix: String,

    #[arg(long, value_enum, default_value_t = ServerKind::Rust)]
    server_kind: ServerKind,

    #[arg(long, default_value_t = 200)]
    users: usize,

    #[arg(long, default_value_t = 100)]
    concurrency: usize,

    #[arg(long, default_value_t = 30)]
    duration_secs: u64,

    #[arg(long, default_value_t = 20)]
    prepare_concurrency: usize,

    #[arg(long, default_value_t = 15)]
    request_timeout_secs: u64,

    #[arg(long, default_value = "perf")]
    name_prefix: String,

    #[arg(long, value_delimiter = ',')]
    songs: Vec<String>,

    #[arg(long, default_value_t = false)]
    download_url: bool,

    #[arg(long, default_value_t = false)]
    prepare_only: bool,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, Eq, PartialEq)]
enum ServerKind {
    Rust,
    Python,
}

#[derive(Clone, Debug)]
struct ApiTarget {
    base_url: String,
    prefix: String,
}

impl ApiTarget {
    fn new(base_url: &str, prefix: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            prefix: normalize_prefix(prefix),
        }
    }

    fn url(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        if self.prefix.is_empty() {
            format!("{}/{}", self.base_url, path)
        } else {
            format!("{}{}/{}", self.base_url, self.prefix, path)
        }
    }
}

#[derive(Clone, Debug)]
struct LoadUser {
    user_id: i32,
    token: String,
}

#[derive(Clone, Debug)]
struct ChartTarget {
    song_id: String,
    difficulty: i32,
    song_hash: String,
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope<T = Value> {
    success: bool,
    value: Option<T>,
    error_code: Option<i64>,
    extra: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct RegisterValue {
    user_id: i32,
    access_token: String,
}

#[derive(Clone, Copy, Debug)]
enum Operation {
    UserMe,
    ScoreTop,
    ScoreSubmit,
    ScoreMe,
    ScoreFriend,
    PurchasePack,
    PurchaseSingle,
    DownloadSong,
    WorldMap,
    FriendMe,
    UpdateUser,
    ScoreToken,
}

impl Operation {
    fn name(self) -> &'static str {
        match self {
            Operation::UserMe => "user_me",
            Operation::ScoreTop => "score_top",
            Operation::ScoreSubmit => "score_submit",
            Operation::ScoreMe => "score_me",
            Operation::ScoreFriend => "score_friend",
            Operation::PurchasePack => "purchase_pack",
            Operation::PurchaseSingle => "purchase_single",
            Operation::DownloadSong => "download_song",
            Operation::WorldMap => "world_map",
            Operation::FriendMe => "friend_me",
            Operation::UpdateUser => "update_user",
            Operation::ScoreToken => "score_token",
        }
    }
}

#[derive(Debug)]
enum RequestOutcome {
    Ok,
    Http { status: u16, body: String },
    Api { code: Option<i64>, body: String },
    Decode(String),
    Transport(String),
}

impl RequestOutcome {
    fn is_ok(&self) -> bool {
        matches!(self, RequestOutcome::Ok)
    }

    fn label(&self) -> &'static str {
        match self {
            RequestOutcome::Ok => "ok",
            RequestOutcome::Http { .. } => "http",
            RequestOutcome::Api { .. } => "api",
            RequestOutcome::Decode(_) => "decode",
            RequestOutcome::Transport(_) => "transport",
        }
    }

    fn short(&self) -> String {
        match self {
            RequestOutcome::Ok => "ok".to_string(),
            RequestOutcome::Http { status, body } => {
                format!("http status={status} body={}", snippet(body))
            }
            RequestOutcome::Api { code, body } => {
                format!("api code={code:?} body={}", snippet(body))
            }
            RequestOutcome::Decode(message) => format!("decode {message}"),
            RequestOutcome::Transport(message) => format!("transport {message}"),
        }
    }
}

struct OpMetrics {
    total: u64,
    ok: u64,
    http_errors: u64,
    api_errors: u64,
    decode_errors: u64,
    transport_errors: u64,
    latency_us: Histogram<u64>,
}

impl OpMetrics {
    fn new() -> Self {
        Self {
            total: 0,
            ok: 0,
            http_errors: 0,
            api_errors: 0,
            decode_errors: 0,
            transport_errors: 0,
            latency_us: new_histogram(),
        }
    }

    fn record(&mut self, latency: Duration, outcome: &RequestOutcome) {
        self.total += 1;
        match outcome {
            RequestOutcome::Ok => self.ok += 1,
            RequestOutcome::Http { .. } => self.http_errors += 1,
            RequestOutcome::Api { .. } => self.api_errors += 1,
            RequestOutcome::Decode(_) => self.decode_errors += 1,
            RequestOutcome::Transport(_) => self.transport_errors += 1,
        }
        let value = latency.as_micros().min(u64::MAX as u128) as u64;
        let _ = self.latency_us.record(value.max(1));
    }

    fn merge(&mut self, other: &OpMetrics) {
        self.total += other.total;
        self.ok += other.ok;
        self.http_errors += other.http_errors;
        self.api_errors += other.api_errors;
        self.decode_errors += other.decode_errors;
        self.transport_errors += other.transport_errors;
        let _ = self.latency_us.add(&other.latency_us);
    }
}

struct Metrics {
    total: u64,
    ok: u64,
    http_errors: u64,
    api_errors: u64,
    decode_errors: u64,
    transport_errors: u64,
    latency_us: Histogram<u64>,
    by_op: BTreeMap<String, OpMetrics>,
    examples: BTreeMap<String, Vec<String>>,
}

impl Metrics {
    fn new() -> Self {
        Self {
            total: 0,
            ok: 0,
            http_errors: 0,
            api_errors: 0,
            decode_errors: 0,
            transport_errors: 0,
            latency_us: new_histogram(),
            by_op: BTreeMap::new(),
            examples: BTreeMap::new(),
        }
    }

    fn record(&mut self, op: Operation, latency: Duration, outcome: RequestOutcome) {
        self.total += 1;
        match &outcome {
            RequestOutcome::Ok => self.ok += 1,
            RequestOutcome::Http { .. } => self.http_errors += 1,
            RequestOutcome::Api { .. } => self.api_errors += 1,
            RequestOutcome::Decode(_) => self.decode_errors += 1,
            RequestOutcome::Transport(_) => self.transport_errors += 1,
        }

        let value = latency.as_micros().min(u64::MAX as u128) as u64;
        let _ = self.latency_us.record(value.max(1));

        let op_name = op.name().to_string();
        self.by_op
            .entry(op_name.clone())
            .or_insert_with(OpMetrics::new)
            .record(latency, &outcome);

        if !outcome.is_ok() {
            let examples = self.examples.entry(op_name).or_default();
            if examples.len() < MAX_ERROR_EXAMPLES_PER_OP {
                examples.push(format!("{}: {}", outcome.label(), outcome.short()));
            }
        }
    }

    fn merge(&mut self, other: &Metrics) {
        self.total += other.total;
        self.ok += other.ok;
        self.http_errors += other.http_errors;
        self.api_errors += other.api_errors;
        self.decode_errors += other.decode_errors;
        self.transport_errors += other.transport_errors;
        let _ = self.latency_us.add(&other.latency_us);

        for (op, metrics) in &other.by_op {
            self.by_op
                .entry(op.clone())
                .or_insert_with(OpMetrics::new)
                .merge(metrics);
        }

        for (op, examples) in &other.examples {
            let target = self.examples.entry(op.clone()).or_default();
            for example in examples {
                if target.len() >= MAX_ERROR_EXAMPLES_PER_OP {
                    break;
                }
                target.push(example.clone());
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    validate_args(&args)?;

    let target = ApiTarget::new(&args.base_url, &args.prefix);
    let client = Client::builder()
        .pool_max_idle_per_host(args.concurrency.max(args.prepare_concurrency).max(1) * 2)
        .timeout(Duration::from_secs(args.request_timeout_secs))
        .build()
        .context("build reqwest client")?;

    let charts = load_charts(&args)?;
    println!(
        "target={} prefix={} users={} concurrency={} duration={}s charts={} download_url={}",
        target.base_url,
        if target.prefix.is_empty() {
            "/"
        } else {
            &target.prefix
        },
        args.users,
        args.concurrency,
        args.duration_secs,
        charts.len(),
        args.download_url
    );

    let prepare_started = Instant::now();
    let users = prepare_users(&args, &client, &target).await?;
    let prepare_elapsed = prepare_started.elapsed();
    println!(
        "prepared users={} elapsed={:.2}s rate={:.1}/s",
        users.len(),
        prepare_elapsed.as_secs_f64(),
        users.len() as f64 / prepare_elapsed.as_secs_f64().max(0.001)
    );

    if args.prepare_only {
        return Ok(());
    }

    let started = Instant::now();
    let metrics = run_load(args.clone(), client, target, users, charts).await?;
    let elapsed = started.elapsed();
    print_report(&metrics, elapsed);

    Ok(())
}

fn validate_args(args: &Args) -> Result<()> {
    if args.users == 0 {
        bail!("--users must be greater than 0");
    }
    if args.concurrency == 0 {
        bail!("--concurrency must be greater than 0");
    }
    if args.prepare_concurrency == 0 {
        bail!("--prepare-concurrency must be greater than 0");
    }
    if args.duration_secs == 0 && !args.prepare_only {
        bail!("--duration-secs must be greater than 0 unless --prepare-only is used");
    }
    Ok(())
}

async fn prepare_users(args: &Args, client: &Client, target: &ApiTarget) -> Result<Vec<LoadUser>> {
    let next = Arc::new(AtomicUsize::new(0));
    let done = Arc::new(AtomicUsize::new(0));
    let run_id = run_id();
    let prefix = safe_name_prefix(&args.name_prefix);

    let mut handles = Vec::new();
    let worker_count = args.prepare_concurrency.min(args.users);
    for _ in 0..worker_count {
        let next = next.clone();
        let done = done.clone();
        let client = client.clone();
        let target = target.clone();
        let run_id = run_id.clone();
        let prefix = prefix.clone();
        let users = args.users;

        handles.push(tokio::spawn(async move {
            let mut local = Vec::new();
            loop {
                let idx = next.fetch_add(1, Ordering::Relaxed);
                if idx >= users {
                    break;
                }

                let result = register_one_with_retry(&client, &target, &prefix, &run_id, idx).await;
                match result {
                    Ok(user) => local.push(Ok(user)),
                    Err(err) => local.push(Err(format!("{err:#}"))),
                }

                let finished = done.fetch_add(1, Ordering::Relaxed) + 1;
                if finished == users || finished % 50 == 0 {
                    eprintln!("prepare progress: {finished}/{users}");
                }
            }
            local
        }));
    }

    let mut users = Vec::with_capacity(args.users);
    let mut errors = Vec::new();
    for handle in handles {
        for result in handle.await.context("prepare worker panicked")? {
            match result {
                Ok(user) => users.push(user),
                Err(err) => {
                    if errors.len() < 5 {
                        errors.push(err);
                    }
                }
            }
        }
    }

    if !errors.is_empty() {
        bail!(
            "failed to prepare {} users; examples: {}",
            errors.len(),
            errors.join(" | ")
        );
    }

    if users.len() != args.users {
        bail!("prepared {} users, expected {}", users.len(), args.users);
    }

    Ok(users)
}

async fn register_one_with_retry(
    client: &Client,
    target: &ApiTarget,
    prefix: &str,
    run_id: &str,
    idx: usize,
) -> Result<LoadUser> {
    let mut last_error = None;

    for attempt in 0..REGISTER_ATTEMPTS {
        match register_one(client, target, prefix, run_id, idx, attempt).await {
            Ok(user) => return Ok(user),
            Err(err) => {
                last_error = Some(err);
                tokio::time::sleep(Duration::from_millis(20 * (attempt as u64 + 1))).await;
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("register failed without error")))
}

async fn register_one(
    client: &Client,
    target: &ApiTarget,
    prefix: &str,
    run_id: &str,
    idx: usize,
    attempt: usize,
) -> Result<LoadUser> {
    let name = make_username(prefix, run_id, idx, attempt);
    let email = format!("{name}@example.com");
    let device_id = format!("load-{run_id}-{idx}");

    let fields = [
        ("name", name.as_str()),
        ("password", PASSWORD),
        ("email", email.as_str()),
        ("device_id", device_id.as_str()),
    ];

    let response = client
        .post(target.url("user"))
        .form(&fields)
        .send()
        .await
        .context("send register request")?;

    let status = response.status();
    let body = response.text().await.context("read register response")?;
    if !status.is_success() {
        bail!(
            "register http status={} body={}",
            status.as_u16(),
            snippet(&body)
        );
    }

    let envelope: ApiEnvelope<RegisterValue> =
        serde_json::from_str(&body).context("decode register response")?;
    if !envelope.success {
        bail!(
            "register api error code={:?} extra={:?} body={}",
            envelope.error_code,
            envelope.extra,
            snippet(&body)
        );
    }

    let value = envelope
        .value
        .ok_or_else(|| anyhow!("register response missing value"))?;
    Ok(LoadUser {
        user_id: value.user_id,
        token: value.access_token,
    })
}

async fn run_load(
    args: Args,
    client: Client,
    target: ApiTarget,
    users: Vec<LoadUser>,
    charts: Vec<ChartTarget>,
) -> Result<Metrics> {
    let users = Arc::new(users);
    let charts = Arc::new(charts);
    let next_user = Arc::new(AtomicUsize::new(0));
    let deadline = Instant::now() + Duration::from_secs(args.duration_secs);
    let mut handles = Vec::new();

    for worker_id in 0..args.concurrency {
        let client = client.clone();
        let target = target.clone();
        let users = users.clone();
        let charts = charts.clone();
        let next_user = next_user.clone();

        handles.push(tokio::spawn(async move {
            let mut rng = StdRng::from_entropy();
            let mut metrics = Metrics::new();

            while Instant::now() < deadline {
                let op = choose_operation(&mut rng);
                let user_idx = next_user.fetch_add(1, Ordering::Relaxed) % users.len();
                let chart_idx = rng.gen_range(0..charts.len());
                let user = &users[user_idx];
                let chart = &charts[chart_idx];

                let started = Instant::now();
                let outcome = execute_operation(
                    op,
                    &client,
                    &target,
                    args.server_kind,
                    user,
                    chart,
                    args.download_url,
                    &mut rng,
                )
                .await;
                metrics.record(op, started.elapsed(), outcome);
            }

            eprintln!("worker {worker_id} done");
            metrics
        }));
    }

    let mut metrics = Metrics::new();
    for handle in handles {
        let worker_metrics = handle.await.context("load worker panicked")?;
        metrics.merge(&worker_metrics);
    }

    Ok(metrics)
}

async fn execute_operation(
    op: Operation,
    client: &Client,
    target: &ApiTarget,
    server_kind: ServerKind,
    user: &LoadUser,
    chart: &ChartTarget,
    download_url: bool,
    rng: &mut StdRng,
) -> RequestOutcome {
    match op {
        Operation::UserMe => {
            execute_json(client.get(target.url("user/me")).bearer_auth(&user.token)).await
        }
        Operation::ScoreTop => {
            let difficulty = chart.difficulty.to_string();
            execute_json(
                client
                    .get(target.url("score/song"))
                    .bearer_auth(&user.token)
                    .query(&[
                        ("song_id", chart.song_id.as_str()),
                        ("difficulty", difficulty.as_str()),
                    ]),
            )
            .await
        }
        Operation::ScoreSubmit => {
            let form = score_submission_form(user, chart, rng);
            execute_json(
                client
                    .post(target.url("score/song"))
                    .bearer_auth(&user.token)
                    .form(&form),
            )
            .await
        }
        Operation::ScoreMe => {
            let difficulty = chart.difficulty.to_string();
            execute_json(
                client
                    .get(target.url("score/song/me"))
                    .bearer_auth(&user.token)
                    .query(&[
                        ("song_id", chart.song_id.as_str()),
                        ("difficulty", difficulty.as_str()),
                    ]),
            )
            .await
        }
        Operation::ScoreFriend => {
            let difficulty = chart.difficulty.to_string();
            execute_json(
                client
                    .get(target.url("score/song/friend"))
                    .bearer_auth(&user.token)
                    .query(&[
                        ("song_id", chart.song_id.as_str()),
                        ("difficulty", difficulty.as_str()),
                    ]),
            )
            .await
        }
        Operation::PurchasePack => {
            execute_json(
                client
                    .get(target.url("purchase/bundle/pack"))
                    .bearer_auth(&user.token),
            )
            .await
        }
        Operation::PurchaseSingle => {
            execute_json(
                client
                    .get(target.url("purchase/bundle/single"))
                    .bearer_auth(&user.token),
            )
            .await
        }
        Operation::DownloadSong => {
            let include_urls = if download_url { "true" } else { "false" };
            execute_json(
                client
                    .get(target.url("serve/download/me/song"))
                    .bearer_auth(&user.token)
                    .query(&[("sid", chart.song_id.as_str()), ("url", include_urls)]),
            )
            .await
        }
        Operation::WorldMap => {
            execute_json(
                client
                    .get(target.url("world/map/me"))
                    .bearer_auth(&user.token),
            )
            .await
        }
        Operation::FriendMe => {
            execute_json(client.get(target.url("friend/me")).bearer_auth(&user.token)).await
        }
        Operation::UpdateUser => {
            if server_kind == ServerKind::Python {
                let value = if rng.gen_bool(0.5) { "true" } else { "false" };
                execute_json(
                    client
                        .post(target.url("user/me/setting/is_hide_rating"))
                        .bearer_auth(&user.token)
                        .form(&[("value", value)]),
                )
                .await
            } else {
                let payload = serde_json::json!({
                    "is_hide_rating": rng.gen_bool(0.5),
                    "favorite_character": if rng.gen_bool(0.5) { 0 } else { 1 },
                });
                execute_json(
                    client
                        .post(target.url("user/update"))
                        .bearer_auth(&user.token)
                        .json(&payload),
                )
                .await
            }
        }
        Operation::ScoreToken => execute_json(client.get(target.url("score/token"))).await,
    }
}

async fn execute_json(builder: reqwest::RequestBuilder) -> RequestOutcome {
    let response = match builder.send().await {
        Ok(response) => response,
        Err(err) => return RequestOutcome::Transport(err.to_string()),
    };

    let status = response.status();
    let body = match response.text().await {
        Ok(body) => body,
        Err(err) => return RequestOutcome::Transport(err.to_string()),
    };

    if !status.is_success() {
        return RequestOutcome::Http {
            status: status.as_u16(),
            body,
        };
    }

    let envelope: ApiEnvelope<Value> = match serde_json::from_str(&body) {
        Ok(envelope) => envelope,
        Err(err) => return RequestOutcome::Decode(err.to_string()),
    };

    if envelope.success {
        RequestOutcome::Ok
    } else {
        RequestOutcome::Api {
            code: envelope.error_code,
            body,
        }
    }
}

fn choose_operation(rng: &mut StdRng) -> Operation {
    match rng.gen_range(0..100) {
        0..=14 => Operation::UserMe,
        15..=30 => Operation::ScoreTop,
        31..=48 => Operation::ScoreSubmit,
        49..=58 => Operation::ScoreMe,
        59..=65 => Operation::ScoreFriend,
        66..=72 => Operation::PurchasePack,
        73..=77 => Operation::PurchaseSingle,
        78..=82 => Operation::DownloadSong,
        83..=87 => Operation::WorldMap,
        88..=92 => Operation::FriendMe,
        93..=97 => Operation::UpdateUser,
        _ => Operation::ScoreToken,
    }
}

fn score_submission_form(
    user: &LoadUser,
    chart: &ChartTarget,
    rng: &mut StdRng,
) -> Vec<(&'static str, String)> {
    let total_notes = 1000;
    let miss_count = rng.gen_range(0..=20);
    let near_count = rng.gen_range(0..=30);
    let perfect_count = total_notes - near_count - miss_count;
    let shiny_perfect_count = 0;
    let score = perfect_count * 10_000 + near_count * 5_000 + shiny_perfect_count;
    let health = if miss_count == 0 {
        100
    } else {
        rng.gen_range(40..=100)
    };
    let modifier = 0;
    let clear_type = if miss_count == 0 && near_count == 0 {
        3
    } else if miss_count == 0 {
        2
    } else {
        1
    };
    let beyond_gauge = 0;

    let submission_hash = score_submission_hash(
        user.user_id,
        &chart.song_hash,
        &chart.song_id,
        chart.difficulty,
        score,
        shiny_perfect_count,
        perfect_count,
        near_count,
        miss_count,
        health,
        modifier,
        clear_type,
    );

    vec![
        ("song_token", SCORE_TOKEN.to_string()),
        ("song_hash", chart.song_hash.clone()),
        ("song_id", chart.song_id.clone()),
        ("difficulty", chart.difficulty.to_string()),
        ("score", score.to_string()),
        ("shiny_perfect_count", shiny_perfect_count.to_string()),
        ("perfect_count", perfect_count.to_string()),
        ("near_count", near_count.to_string()),
        ("miss_count", miss_count.to_string()),
        ("health", health.to_string()),
        ("modifier", modifier.to_string()),
        ("clear_type", clear_type.to_string()),
        ("beyond_gauge", beyond_gauge.to_string()),
        ("submission_hash", submission_hash),
    ]
}

#[allow(clippy::too_many_arguments)]
fn score_submission_hash(
    user_id: i32,
    song_hash: &str,
    song_id: &str,
    difficulty: i32,
    score: i32,
    shiny_perfect_count: i32,
    perfect_count: i32,
    near_count: i32,
    miss_count: i32,
    health: i32,
    modifier: i32,
    clear_type: i32,
) -> String {
    let hash_input = format!(
        "{SCORE_TOKEN}{song_hash}{song_id}{difficulty}{score}{shiny_perfect_count}{perfect_count}{near_count}{miss_count}{health}{modifier}{clear_type}"
    );
    let user_hash = md5_string(&format!("{user_id}{song_hash}"));
    md5_string(&format!("{hash_input}{user_hash}"))
}

fn load_charts(args: &Args) -> Result<Vec<ChartTarget>> {
    let mut charts = if args.songs.is_empty() {
        scan_local_charts("songs")?
    } else {
        parse_chart_specs(&args.songs)?
    };

    charts.sort_by(|a, b| {
        a.song_id
            .cmp(&b.song_id)
            .then_with(|| a.difficulty.cmp(&b.difficulty))
    });
    charts.dedup_by(|a, b| a.song_id == b.song_id && a.difficulty == b.difficulty);

    if charts.is_empty() {
        bail!("no charts available; pass --songs song_id:difficulty or run from the repo root");
    }

    Ok(charts)
}

fn scan_local_charts(root: &str) -> Result<Vec<ChartTarget>> {
    let root = Path::new(root);
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut charts = Vec::new();
    for song_entry in fs::read_dir(root).context("read songs directory")? {
        let song_entry = song_entry.context("read song entry")?;
        if !song_entry
            .file_type()
            .context("read song file type")?
            .is_dir()
        {
            continue;
        }

        let song_id = song_entry.file_name().to_string_lossy().to_string();
        for chart_entry in fs::read_dir(song_entry.path())
            .with_context(|| format!("read chart directory for {}", song_entry.path().display()))?
        {
            let chart_entry = chart_entry.context("read chart entry")?;
            if !chart_entry
                .file_type()
                .context("read chart file type")?
                .is_file()
            {
                continue;
            }

            let file_name = chart_entry.file_name().to_string_lossy().to_string();
            let Some(difficulty_text) = file_name.strip_suffix(".aff") else {
                continue;
            };
            let Ok(difficulty) = difficulty_text.parse::<i32>() else {
                continue;
            };
            if !(0..=4).contains(&difficulty) {
                continue;
            }

            let bytes = fs::read(chart_entry.path())
                .with_context(|| format!("read chart {}", chart_entry.path().display()))?;
            charts.push(ChartTarget {
                song_id: song_id.clone(),
                difficulty,
                song_hash: format!("{:x}", md5::compute(bytes)),
            });
        }
    }

    Ok(charts)
}

fn parse_chart_specs(specs: &[String]) -> Result<Vec<ChartTarget>> {
    let mut charts = Vec::new();
    for spec in specs {
        let (song_id, difficulty_text) = spec.rsplit_once(':').ok_or_else(|| {
            anyhow!("invalid --songs entry `{spec}`, expected song_id:difficulty")
        })?;
        let difficulty = difficulty_text
            .parse::<i32>()
            .with_context(|| format!("invalid difficulty in --songs entry `{spec}`"))?;
        if !(0..=4).contains(&difficulty) {
            bail!("difficulty out of range in --songs entry `{spec}`");
        }

        let path = Path::new("songs")
            .join(song_id)
            .join(format!("{difficulty}.aff"));
        let song_hash = if path.is_file() {
            format!(
                "{:x}",
                md5::compute(
                    fs::read(&path)
                        .with_context(|| { format!("read chart {}", path.display()) })?
                )
            )
        } else {
            "test_hash".to_string()
        };

        charts.push(ChartTarget {
            song_id: song_id.to_string(),
            difficulty,
            song_hash,
        });
    }

    Ok(charts)
}

fn print_report(metrics: &Metrics, elapsed: Duration) {
    let seconds = elapsed.as_secs_f64().max(0.001);
    println!();
    println!("load result");
    println!("elapsed_s        {:.2}", seconds);
    println!("total_requests   {}", metrics.total);
    println!("ok_requests      {}", metrics.ok);
    println!("qps_total        {:.2}", metrics.total as f64 / seconds);
    println!("qps_ok           {:.2}", metrics.ok as f64 / seconds);
    println!(
        "errors           http={} api={} decode={} transport={}",
        metrics.http_errors, metrics.api_errors, metrics.decode_errors, metrics.transport_errors
    );
    println!(
        "latency_ms       p50={} p90={} p95={} p99={} max={}",
        percentile_ms(&metrics.latency_us, 0.50),
        percentile_ms(&metrics.latency_us, 0.90),
        percentile_ms(&metrics.latency_us, 0.95),
        percentile_ms(&metrics.latency_us, 0.99),
        max_ms(&metrics.latency_us)
    );
    println!();
    println!(
        "{:<16} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9}",
        "operation", "total", "ok", "err", "qps", "p95_ms", "p99_ms"
    );
    for (op, stat) in &metrics.by_op {
        let err = stat.total.saturating_sub(stat.ok);
        println!(
            "{:<16} {:>9} {:>9} {:>9} {:>9.2} {:>9} {:>9}",
            op,
            stat.total,
            stat.ok,
            err,
            stat.total as f64 / seconds,
            percentile_ms(&stat.latency_us, 0.95),
            percentile_ms(&stat.latency_us, 0.99)
        );
    }

    if !metrics.examples.is_empty() {
        println!();
        println!("error examples");
        for (op, examples) in &metrics.examples {
            for example in examples {
                println!("{op}: {example}");
            }
        }
    }
}

fn new_histogram() -> Histogram<u64> {
    Histogram::new_with_bounds(1, 60_000_000, 3).expect("valid histogram bounds")
}

fn percentile_ms(histogram: &Histogram<u64>, quantile: f64) -> String {
    if histogram.is_empty() {
        "-".to_string()
    } else {
        format!(
            "{:.2}",
            histogram.value_at_quantile(quantile) as f64 / 1000.0
        )
    }
}

fn max_ms(histogram: &Histogram<u64>) -> String {
    if histogram.is_empty() {
        "-".to_string()
    } else {
        format!("{:.2}", histogram.max() as f64 / 1000.0)
    }
}

fn normalize_prefix(prefix: &str) -> String {
    let trimmed = prefix.trim().trim_matches('/');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("/{trimmed}")
    }
}

fn safe_name_prefix(input: &str) -> String {
    let mut output: String = input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(4)
        .collect();
    if output.is_empty() {
        output.push('p');
    }
    output.to_ascii_lowercase()
}

fn make_username(prefix: &str, run_id: &str, idx: usize, attempt: usize) -> String {
    let idx = base36(idx as u128);
    let attempt = base36(attempt as u128);
    let mut name = format!("{prefix}{run_id}{idx}{attempt}");
    if name.len() > 16 {
        name.truncate(16);
    }
    if name.len() < 3 {
        name.push_str("usr");
        name.truncate(3);
    }
    name
}

fn run_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_millis();
    let mut id = base36(millis);
    if id.len() > 7 {
        id = id[id.len() - 7..].to_string();
    }
    id
}

fn base36(mut value: u128) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if value == 0 {
        return "0".to_string();
    }

    let mut chars = Vec::new();
    while value > 0 {
        let idx = (value % 36) as usize;
        chars.push(DIGITS[idx] as char);
        value /= 36;
    }
    chars.iter().rev().collect()
}

fn md5_string(input: &str) -> String {
    format!("{:x}", md5::compute(input.as_bytes()))
}

fn snippet(body: &str) -> String {
    let single_line = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.len() <= 180 {
        single_line
    } else {
        format!("{}...", &single_line[..180])
    }
}
