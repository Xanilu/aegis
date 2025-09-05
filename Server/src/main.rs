use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::time::{Instant, Interval};

const CIPHER_BLOB_BYTES: usize = 4096;   // feste Größe (nach Base64)
const RL_MAX_PER_WINDOW: usize = 20;     // max. Nachrichten pro Fenster
const RL_WINDOW: Duration = Duration::from_secs(60); // Fensterlänge

/// Umschlag in der Mailbox
struct Envelope {
    cipher_blob: String,   // Base64 (geprüft)
    expires_at: Instant,
}

/// Gemeinsamer Serverzustand
#[derive(Clone)]
struct AppState {
    mailboxes: Arc<Mutex<HashMap<String, Vec<Envelope>>>>, // to_id -> queue
    ratelimit: Arc<Mutex<HashMap<String, Vec<Instant>>>>,  // to_id -> Sendezeiten
    ttl: Duration,
}

#[derive(Deserialize)]
struct PutEnvelope {
    to_id: String,
    cipher_blob: String, // Base64
}

#[derive(Deserialize)]
struct MailboxQuery {
    r#for: String, // raw identifier (keyword "for")
}

/// Hintergrund-Task: abgelaufene Nachrichten & alte Rate-Timestamps aufräumen
async fn start_gc(state: AppState, mut tick: Interval) {
    loop {
        tick.tick().await;
        let now = Instant::now();

        // Mailbox-TTL säubern
        {
            let mut boxes = state.mailboxes.lock().await;
            for (_id, queue) in boxes.iter_mut() {
                queue.retain(|e| e.expires_at > now);
            }
        }

        // Rate-Limit-Fenster säubern
        {
            let mut rl = state.ratelimit.lock().await;
            for (_id, times) in rl.iter_mut() {
                times.retain(|t| now.saturating_duration_since(*t) < RL_WINDOW);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // Zustand
    let state = AppState {
        mailboxes: Arc::new(Mutex::new(HashMap::new())),
        ratelimit: Arc::new(Mutex::new(HashMap::new())),
        ttl: Duration::from_secs(72 * 60 * 60), // 72h
    };

    // Router
    let app = Router::new()
        .route("/ping", get(|| async { "pong" }))
        .route("/v1/envelopes", put(put_envelope))
        .route("/v1/mailbox", get(get_mailbox))
        .with_state(state.clone());

    // GC-Task alle 60s
    let interval = tokio::time::interval(Duration::from_secs(60));
    tokio::spawn(start_gc(state.clone(), interval));

    // Listener
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("🚀 Aegis Relay läuft auf http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

/// PUT /v1/envelopes
/// Body: { "to_id": "...", "cipher_blob": "BASE64..." }
async fn put_envelope(
    State(state): State<AppState>,
    Json(payload): Json<PutEnvelope>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Basic-Checks
    if payload.to_id.trim().is_empty() || payload.cipher_blob.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "ok": false, "error": "invalid_input" })),
        );
    }

    // Base64 & Größen-Check
    match base64::decode(&payload.cipher_blob) {
        Ok(bytes) if bytes.len() == CIPHER_BLOB_BYTES => { /* passt */ }
        Ok(bytes) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "ok": false,
                    "error": "invalid_size",
                    "expected_bytes": CIPHER_BLOB_BYTES,
                    "got_bytes": bytes.len()
                })),
            );
        }
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "ok": false, "error": "invalid_base64" })),
            );
        }
    }

    // --- Rate-Limit pro Empfänger (`to_id`) ---
    {
        let mut rl = state.ratelimit.lock().await;
        let times = rl.entry(payload.to_id.clone()).or_default();
        let now = Instant::now();

        // Fenster säubern (nur Events < 60s behalten)
        times.retain(|t| now.saturating_duration_since(*t) < RL_WINDOW);

        if times.len() >= RL_MAX_PER_WINDOW {
            // 429 Too Many Requests
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::json!({
                    "ok": false,
                    "error": "rate_limited",
                    "limit": RL_MAX_PER_WINDOW,
                    "window_seconds": RL_WINDOW.as_secs()
                })),
            );
        }

        // Ereignis speichern
        times.push(now);
    }

    // Speichern in Mailbox
    {
        let mut boxes = state.mailboxes.lock().await;
        let queue = boxes.entry(payload.to_id).or_default();
        queue.push(Envelope {
            cipher_blob: payload.cipher_blob,
            expires_at: Instant::now() + state.ttl,
        });
    }

    (StatusCode::OK, Json(serde_json::json!({ "ok": true })))
}

/// GET /v1/mailbox?for=<id>
/// Antwort: { "ok": true, "envelopes": [ { "cipher_blob": "..." }, ... ] }
async fn get_mailbox(
    State(state): State<AppState>,
    Query(q): Query<MailboxQuery>,
) -> Json<serde_json::Value> {
    let mut boxes = state.mailboxes.lock().await;
    let mut out = Vec::new();

    if let Some(queue) = boxes.get_mut(&q.r#for) {
        let now = Instant::now();
        queue.retain(|e| e.expires_at > now);
        let drained: Vec<Envelope> = queue.drain(..).collect();
        for e in drained {
            out.push(serde_json::json!({ "cipher_blob": e.cipher_blob }));
        }
    }

    Json(serde_json::json!({ "ok": true, "envelopes": out }))
}
