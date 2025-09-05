# Roadmap

## Phase 0 – Fundament
- Name/Branding ✅
- Privacy-Charta ✅
- ADR-Entscheidungen (Protokoll, Stack, Identität)
- Threat Model + Architektur-Skizze

## Phase 1 – Prototyp (MVP, Android zuerst)
- Rust Core: X3DH/Double Ratchet, Key-Vault
- Flutter App: 1:1 Chat, QR-Key-Verifizierung, Panik-PIN
- Relay-Server: TTL-Mailbox, keine Logs, Tor-HS
- Interne Tests (Android APK)

## Phase 2 – Multi-Platform & Premium
- iOS-Port (Silent Push = Wecksignal, Pull-On-Open)
- Desktop (Flutter)
- Gruppen (klein, Sender-Keys), Multi-Device (Cross-Signing)
- Monetarisierung: Free vs. Premium (Backups, große Gruppen, Multi-Device)

## Phase 3 – Härtung & Launch
- Externes Audit
- Store-Compliance (iOS), Website
- Öffentlicher Launch

## Phase 4 – Skalierung
- MLS, Mixnets
- Internationalisierung, Partner-Server
