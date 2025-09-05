# 🛡️ Aegis – Der Messenger, der dich und deine Grundrechte schützt

Aegis ist ein ***privacy-first Messenger***, inspiriert vom Grundgesetz:  
**Keine Zensur, keine Backdoors, absolute Anonymität.**  
Alle Nachrichten sind **Ende-zu-Ende verschlüsselt**, der Relay speichert nur **opaque Blobs** und kennt keine Metadaten außer `to_id`.

---

## ✅ Status (MVP)

- **Rust Relay (Axum/Tokio)**
  - `/ping`, `/health`
  - `PUT /v1/envelopes` (**4096B Cipher-Blobs** mit PoW + TTL)
  - `GET /v1/mailbox?for=ID` (*drain*)
  - **TTL:** 72h Garbage Collection  
  - **Rate-Limit:** 20 Requests / 60s pro `to_id`  
  - **Proof-of-Work:** `SHA256(nonce+id)` → Prefix

- **Flutter App**
  - Android Emulator + Windows getestet
  - **Keygen** (X25519 → Shared Secret → AES-GCM-256)
  - **4096B Frame-Format** mit Nonce & Padding
  - **QR-Code Austausch** (Public Keys)
  - Text senden + entschlüsseln

---

## 🚀 Projektstruktur
aegis/
├─ Server/ # Rust Relay
└─ app/
└─ aegis_app/ # Flutter App (Android + Windows


---

## 🔧 Installation & Start

### 1. Relay starten (Rust)
```powershell
cd Server
# optional PoW aus:
# $env:POW_PREFIX=""
cargo run
Standard-Port: 3000

Standard-Bind: 127.0.0.1

Für echtes Handy im WLAN: 0.0.0.0 binden + Firewall-Port 3000 freigeben.

Endpoints:

GET /ping → pong

PUT /v1/envelopes → Nachricht ablegen

GET /v1/mailbox?for=alice → Postfach abholen
2. App starten (Flutter)

Android Emulator (empfohlen):

cd app/aegis_app
flutter pub get
flutter run -d emulator-5554


Windows Desktop (ohne QR-Scanner):

flutter run -d windows
---
```
## 📱 App-Konfiguration
```
Relay URL

Windows: http://127.0.0.1:3000

Android Emulator: http://10.0.2.2:3000

Echtes Handy (WLAN): http://<LAN-IP>:3000

PoW Prefix

Muss mit Server übereinstimmen (0000, oder leer wenn PoW aus)

Key-Exchange

„Keygen“ → Public Key erzeugen + QR

„QR scannen“ → Kontakt-Key übernehmen

to_id = Inbox-Name des Empfängers
---
```
## 📩 Testablauf
```
Alice & Bob starten App, drücken Keygen

Beide tauschen Public Keys (QR oder Copy/Paste)

Alice setzt to_id = bob → TEXT SENDEN (E2E)

Bob klickt HOLEN & ENTCRYPTEN → 📩 Nachricht erscheint
---
```
## 🛠️ Troubleshooting
```
| Problem                 | Ursache / Hinweis                       | Lösung                                                           |
| ----------------------- | --------------------------------------- | ---------------------------------------------------------------- |
| Emu sieht Server nicht  | Falsche URL im Emulator                 | `http://10.0.2.2:3000` nutzen                                    |
| `pow_required` Meldung  | Prefix stimmt nicht überein             | Prefix in App & Server abgleichen oder PoW am Server ausschalten |
| 429 Too Many Requests   | Rate-Limit (20 Requests / 60s) erreicht | Kurz warten                                                      |
| Mailbox leer            | falsche `to_id`                         | Korrekte ID eintragen                                            |
| QR-Kamera schwarz (Emu) | Emulator-Cam nicht gesetzt              | Back Camera → Webcam0 + „Cold Boot“                              |
| Echtes Handy im WLAN    | Server bindet nur auf 127.0.0.1         | An `0.0.0.0` binden + Firewall freigeben
---                       |
```
## 📍 Roadmap
```
 Preset-Buttons für Relay-URL & to_id-Guard

 Kontakt-QR erweitert (aegis:pk1:<pubkey>?id=<to_id>)

 HKDF-SHA256 statt SHA-256(raw) als KDF

 Dockerfile + HTTPS (Caddy/Nginx + Certbot)

 CI: Build, Format, Tests
---
```
„Jeder hat das Recht, seine Meinung in Wort, Schrift und Bild frei zu äußern und zu verbreiten… Eine Zensur findet nicht statt.“ – GG Art. 5

Aegis will diese Freiheit digital garantieren.
