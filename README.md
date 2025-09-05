# Aegis 🛡️ — Der Messenger, der dich und deine Grundrechte schützt

**Aegis** ist ein auf Privatsphäre ausgelegtes Messaging-Projekt.  
Aktuell enthält das Repo einen **Relay-Server (Rust/Axum)** und einen **Mini-Client (Flutter)** zum lokalen Testen.

> Ziel: Absolute Anonymität, minimale Metadaten, feste Paketgröße, saubere Trennung von Transport und Inhalt.

---

## ✨ Aktueller Stand (MVP)

- **Relay (Rust/Axum)**
  - `GET /ping` → Lebenszeichen
  - `GET /health` → aggregierte Statistiken (queues, total, uptime)
  - `PUT /v1/envelopes` → Cipher-Blob ablegen (Base64, **exakt 4096 Bytes** nach Decoding)
  - `GET /v1/mailbox?for=<id>` → Mailbox abholen & sofort leeren (store-and-delete)
  - **TTL** (Standard: 72h) + periodisches **GC**
  - **Rate-Limit**: 20 Nachrichten / 60s pro `to_id` (konfigurierbar)
  - **Persistenz** via Event-Log (`mailbox.log`) + periodische **Kompaktion**
  - **ENV-Konfig** (Port, TTL, Limits, Datenpfad, PoW-Schwierigkeit)
  - **Proof-of-Work**: Header `X-POW-Nonce`, SHA-256(nonce + to_id) muss mit Präfix matchen

- **Mini-Client (Flutter Desktop/Android/iOS)**
  - Ping, Health
  - 4096-Byte Blobs senden (inkl. automatischer PoW-Nonce-Suche)
  - Mailbox holen & anzeigen

---

## 📂 Projektstruktur

