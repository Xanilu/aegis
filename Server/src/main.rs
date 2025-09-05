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

const CIPHER_BLOB_BYTES: usize = 4096; // feste Nutzlastgröße (nach Base64-Decoding)

/// Ein einzelner Umschlag in der Mailbox (nur Cipher, keine Serde-Derives nötig)
struct Envelope {
    cipher_blob: String,   // gespeicherte Base64-Darstellung (geprüft)
    expires_at: Instant,
}

/// Gemeinsamer Serverzustand
#[derive(Clone)]
struct AppState {
    mailboxes: Arc<Mutex<HashMap<String, Vec<Envelope>>>>, // to_id -> queue
    ttl: Duration,
}

#[derive(Deserialize)]
struct PutEnvelope {
    to_id: String,
    cipher_blob: String, // Base64
}

#[derive(Deserialize)]
struct MailboxQuery {
    // `for` ist ein Schlüsselwort; raw identifier benutzen
    r#for: String,
}

/// Hintergrund-Task: abgelaufene Nachrichten aufräumen
async fn start_gc(state: AppState, mut tick: Interval) {
    loop {
        tick.tick().await;
        let now = Instant::now();
        let mut boxes = state.mailboxes.lock().await;
        for (_id, queue) in boxes.iter_mut() {
            queue.retain(|e| e.expires_at > now);
        }
    }
}

#[tokio::main]
async fn main() {
    // Zustand
    let state = AppState {
        mailboxes: Arc::new(Mutex::new(HashMap::new())),
        ttl: Duration::from_secs(72 * 60 * 60), // 72h
    };

    // Router
    let app = Router::new()
        .route("/ping", get(|| async { "pong" }))
        .route("/v1/envelopes", put(put_envelope))
        .route("/v1/mailbox", get(get_mailbox))
        .with_state(state.clone());

    // GC-Task alle 60 Sekunden
    let interval = tokio::time::interval(Duration::from_secs(60));
    tokio::spawn(start_gc(state.clone(), interval));

    // Listener
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("🚀 Aegis Relay läuft auf http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}

/// PUT /v1/envelopes
/// Body (JSON): { "to_id": "...", "cipher_blob": "BASE64..." }
async fn put_envelope(
    State(state): State<AppState>,
    Json(payload): Json<PutEnvelope>,
) -> (StatusCode, Json<serde_json::Value>) {
    if payload.to_id.trim().is_empty() || payload.cipher_blob.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "ok": false, "error": "invalid_input" })),
        );
    }

    // Base64 prüfen und nach Decoding LÄNGE VALIDIEREN
    match base64::decode(&payload.cipher_blob) {
        Ok(bytes) if bytes.len() == CIPHER_BLOB_BYTES => {
            let mut boxes = state.mailboxes.lock().await;
            let queue = boxes.entry(payload.to_id).or_default();
            queue.push(Envelope {
                cipher_blob: payload.cipher_blob,
                expires_at: Instant::now() + state.ttl,
            });
            (StatusCode::OK, Json(serde_json::json!({ "ok": true })))
        }
        Ok(bytes) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": "invalid_size",
                "expected_bytes": CIPHER_BLOB_BYTES,
                "got_bytes": bytes.len()
            })),
        ),
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "ok": false, "error": "invalid_base64" })),
        ),
    }
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
        // Failsafe: abgelaufene vor dem Drain filtern
        let now = Instant::now();
        queue.retain(|e| e.expires_at > now);

        // Alles abholen & löschen (store-and-delete)
        let drained: Vec<Envelope> = queue.drain(..).collect();
        for e in drained {
            out.push(serde_json::json!({ "cipher_blob": e.cipher_blob }));
        }
    }

    Json(serde_json::json!({ "ok": true, "envelopes": out }))
}
