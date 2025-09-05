use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::io::AsyncWriteExt;
use tokio::fs::{self, OpenOptions, read};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::time::{Instant, Interval};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

#[derive(Clone)]
struct Config {
    port: u16,
    ttl: Duration,
    cipher_blob_bytes: usize,
    rl_max_per_window: usize,
    rl_window: Duration,
    data_dir: PathBuf,
    pow_prefix: String,
}
impl Config {
    fn from_env() -> Self {
        let port = env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(3000);
        let ttl_hours = env::var("TTL_HOURS").ok().and_then(|s| s.parse().ok()).unwrap_or(72);
        let cipher_blob_bytes = env::var("CIPHER_BLOB_BYTES").ok().and_then(|s| s.parse().ok()).unwrap_or(4096);
        let rl_max_per_window = env::var("RL_MAX_PER_WINDOW").ok().and_then(|s| s.parse().ok()).unwrap_or(20);
        let rl_window_secs = env::var("RL_WINDOW_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(60);
        let data_dir = env::var("DATA_DIR").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("./data"));
        let pow_prefix = env::var("POW_PREFIX").unwrap_or_else(|_| "0000".to_string());
        Self {
            port,
            ttl: Duration::from_secs(ttl_hours as u64 * 3600),
            cipher_blob_bytes,
            rl_max_per_window,
            rl_window: Duration::from_secs(rl_window_secs),
            data_dir,
            pow_prefix,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "t")]
enum LogEvent {
    Put { to_id: String, cipher_blob: String, expires_ms: u128 },
    Drain { to_id: String },
}

struct Storage {
    cfg: Config,
    log_path: PathBuf,
}
impl Storage {
    async fn new(cfg: Config) -> anyhow::Result<Self> {
        println!("[init] Storage in {:?}", cfg.data_dir);
        if !Path::new(&cfg.data_dir).exists() {
            fs::create_dir_all(&cfg.data_dir).await?;
        }
        let log_path = cfg.data_dir.join("mailbox.log");
        println!("[init] Log file {:?}", log_path);
        // nur anlegen, sofort schließen
        OpenOptions::new().create(true).append(true).open(&log_path).await?;
        Ok(Self { cfg, log_path })
    }
    async fn append(&self, ev: &LogEvent) -> anyhow::Result<()> {
        let mut file = OpenOptions::new().create(true).append(true).open(&self.log_path).await?;
        let line = serde_json::to_string(ev)? + "\n";
        file.write_all(line.as_bytes()).await?;
        file.flush().await?;
        Ok(())
    }
        async fn load(&self) -> anyhow::Result<HashMap<String, Vec<Envelope>>> {
        println!("[load] replay from {:?}", self.log_path);

        // Datei anlegen, falls sie noch nicht existiert
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await?;

        // Inhalt komplett lesen (async) und in Zeilen splitten
        let bytes = read(&self.log_path).await?;
        if bytes.is_empty() {
            println!("[load] empty log");
            return Ok(HashMap::new());
        }

        let text = String::from_utf8_lossy(&bytes);
        let mut map: HashMap<String, Vec<Envelope>> = HashMap::new();

        let now_instant = Instant::now();
        let now_ms = now_epoch_ms();
        let mut n = 0usize;

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }
            match serde_json::from_str::<LogEvent>(line) {
                Ok(LogEvent::Put { to_id, cipher_blob, expires_ms }) => {
                    if expires_ms > now_ms {
                        let remaining = Duration::from_millis((expires_ms - now_ms) as u64);
                        map.entry(to_id)
                            .or_default()
                            .push(Envelope {
                                cipher_blob,
                                expires_at: now_instant + remaining,
                            });
                    }
                    n += 1;
                }
                Ok(LogEvent::Drain { to_id }) => {
                    map.remove(&to_id);
                    n += 1;
                }
                Err(_) => {
                    // Ignoriere defekte Zeilen – robust bleiben
                }
            }
        }

        println!("[load] applied {} events", n);
        Ok(map)
    }

    async fn compact(&self, state: &HashMap<String, Vec<Envelope>>) -> anyhow::Result<()> {
        let tmp = self.cfg.data_dir.join("mailbox.tmp");
        let mut tf = OpenOptions::new().create(true).write(true).truncate(true).open(&tmp).await?;
        let now_ms = now_epoch_ms();
        let mut c = 0usize;
        for (to_id, queue) in state.iter() {
            for envl in queue {
                let rest = envl.expires_at.checked_duration_since(Instant::now()).unwrap_or(Duration::from_secs(0));
                let expires_ms = now_ms + rest.as_millis() as u128;
                let ev = LogEvent::Put { to_id: to_id.clone(), cipher_blob: envl.cipher_blob.clone(), expires_ms };
                let line = serde_json::to_string(&ev)? + "\n";
                tf.write_all(line.as_bytes()).await?;
                c += 1;
            }
        }
        tf.flush().await?;
        println!("[compact] wrote {} lines -> rename", c);
        fs::rename(&tmp, &self.log_path).await?;
        Ok(())
    }
}

fn now_epoch_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
}

#[derive(Clone)]
struct Envelope { cipher_blob: String, expires_at: Instant }

#[derive(Clone)]
struct AppState {
    cfg: Config,
    mailboxes: Arc<Mutex<HashMap<String, Vec<Envelope>>>>,
    ratelimit: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    storage: Arc<Storage>,
    started: Instant,
}

#[derive(Deserialize)] struct PutEnvelope { to_id: String, cipher_blob: String }
#[derive(Deserialize)] struct MailboxQuery { r#for: String }

async fn start_gc(state: AppState, mut tick: Interval) {
    loop {
        tick.tick().await;
        let now = Instant::now();
        {
            let mut boxes = state.mailboxes.lock().await;
            for (_id, q) in boxes.iter_mut() { q.retain(|e| e.expires_at > now); }
        }
        {
            let mut rl = state.ratelimit.lock().await;
            for (_id, times) in rl.iter_mut() {
                times.retain(|t| now.saturating_duration_since(*t) < state.cfg.rl_window);
            }
        }
    }
}
async fn start_compactor(state: AppState, mut tick: Interval) {
    loop {
        tick.tick().await;
        let snapshot = {
            let boxes = state.mailboxes.lock().await;
            boxes.clone()
        };
        if let Err(e) = state.storage.compact(&snapshot).await {
            eprintln!("[compact] error: {e:?}");
        }
    }
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let boxes = state.mailboxes.lock().await;
    let queues = boxes.len();
    let total: usize = boxes.values().map(|v| v.len()).sum();
    let up = Instant::now().saturating_duration_since(state.started).as_secs();
    Json(serde_json::json!({ "ok": true, "queues": queues, "total_envelopes": total, "uptime_seconds": up }))
}

fn valid_pow(prefix: &str, nonce: &str, to_id: &str) -> bool {
    if prefix.is_empty() { return true; }
    let mut h = Sha256::new();
    h.update(nonce.as_bytes());
    h.update(to_id.as_bytes());
    let hex = hex_lower(&h.finalize());
    hex.starts_with(prefix)
}
fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len()*2);
    for &b in bytes { s.push(HEX[(b>>4) as usize] as char); s.push(HEX[(b&0x0f) as usize] as char); }
    s
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("[boot] load config");
    let cfg = Config::from_env();
    println!("[boot] cfg: port={}, ttl_h={}, blob={}B, rl={}/{}s, data={:?}, pow_prefix='{}'",
        cfg.port, cfg.ttl.as_secs()/3600, cfg.cipher_blob_bytes, cfg.rl_max_per_window, cfg.rl_window.as_secs(), cfg.data_dir, cfg.pow_prefix);

    println!("[boot] init storage");
    let storage = Arc::new(Storage::new(cfg.clone()).await?);

    println!("[boot] load log");
    let loaded = storage.load().await?;

    println!("[boot] build state/router");
    let state = AppState {
        cfg: cfg.clone(),
        mailboxes: Arc::new(Mutex::new(loaded)),
        ratelimit: Arc::new(Mutex::new(HashMap::new())),
        storage,
        started: Instant::now(),
    };
    let app = Router::new()
        .route("/ping", get(|| async { "pong" }))
        .route("/health", get(health))
        .route("/v1/envelopes", put(put_envelope))
        .route("/v1/mailbox", get(get_mailbox))
        .with_state(state.clone());

    println!("[boot] spawn GC + compactor");
    tokio::spawn(start_gc(state.clone(), tokio::time::interval(Duration::from_secs(60))));
    tokio::spawn(start_compactor(state.clone(), tokio::time::interval(Duration::from_secs(300))));

    let bind = format!("127.0.0.1:{}", cfg.port);
    println!("[boot] bind {}", bind);
    let listener = TcpListener::bind(&bind).await?;
    println!("🚀 Aegis Relay läuft auf http://{}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn put_envelope(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<PutEnvelope>,
) -> (StatusCode, Json<serde_json::Value>) {
    if payload.to_id.trim().is_empty() || payload.cipher_blob.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"ok":false,"error":"invalid_input"})));
    }
    match BASE64.decode(&payload.cipher_blob) {

        Ok(bytes) if bytes.len() == state.cfg.cipher_blob_bytes => {}
        Ok(bytes) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"ok":false,"error":"invalid_size","expected_bytes":state.cfg.cipher_blob_bytes,"got_bytes":bytes.len()})));
        }
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"ok":false,"error":"invalid_base64"})));
        }
    }
    let nonce = headers.get("X-POW-Nonce").and_then(|v| v.to_str().ok()).unwrap_or("");
    if !valid_pow(&state.cfg.pow_prefix, nonce, &payload.to_id) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"ok":false,"error":"pow_required","prefix":state.cfg.pow_prefix})));
    }
    {
        let mut rl = state.ratelimit.lock().await;
        let times = rl.entry(payload.to_id.clone()).or_default();
        let now = Instant::now();
        times.retain(|t| now.saturating_duration_since(*t) < state.cfg.rl_window);
        if times.len() >= state.cfg.rl_max_per_window {
            return (StatusCode::TOO_MANY_REQUESTS, Json(serde_json::json!({"ok":false,"error":"rate_limited","limit":state.cfg.rl_max_per_window,"window_seconds":state.cfg.rl_window.as_secs()})));
        }
        times.push(now);
    }
    {
        let mut boxes = state.mailboxes.lock().await;
        let queue = boxes.entry(payload.to_id.clone()).or_default();
        let expires_at = Instant::now() + state.cfg.ttl;
        queue.push(Envelope { cipher_blob: payload.cipher_blob.clone(), expires_at });
        let expires_ms = now_epoch_ms() + state.cfg.ttl.as_millis() as u128;
        if let Err(e) = state.storage.append(&LogEvent::Put { to_id: payload.to_id, cipher_blob: payload.cipher_blob, expires_ms }).await {
            eprintln!("[persist] append error: {e:?}");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok":false,"error":"persist_failed"})));
        }
    }
    (StatusCode::OK, Json(serde_json::json!({"ok":true})))
}

async fn get_mailbox(
    State(state): State<AppState>,
    Query(q): Query<MailboxQuery>,
) -> Json<serde_json::Value> {
    let mut out = Vec::new();
    {
        let mut boxes = state.mailboxes.lock().await;
        if let Some(queue) = boxes.get_mut(&q.r#for) {
            let now = Instant::now();
            queue.retain(|e| e.expires_at > now);
            let drained: Vec<Envelope> = queue.drain(..).collect();
            for e in drained {
                out.push(serde_json::json!({ "cipher_blob": e.cipher_blob }));
            }
        }
    }
    if let Err(e) = state.storage.append(&LogEvent::Drain { to_id: q.r#for }).await {
        eprintln!("[persist] drain error: {e:?}");
    }
    Json(serde_json::json!({ "ok": true, "envelopes": out }))
}
