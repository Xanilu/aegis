# Architektur (v0.1)

## Übersicht
- **App (Flutter):** UI/State, QR-Scan, Push-Wecksignale, Pull-On-Open.
- **Core-SDK (Rust):** Kryptografie, Sessions, Storage (SQLCipher/Sqlite), Netzwerk (Client).
- **Relay-Server (Rust/Go):** Store-and-Forward verschlüsselter Umschläge, TTL, Rate-Limits via Privacy-Pass/PoW, Tor-HS.

## Datenflüsse
1) Registrierung: Client erzeugt Schlüsselpaar lokal → anonyme ID.
2) Key-Verifikation: QR-Scan/Fingerprint (optional Wörterliste).
3) Nachricht: Client verschlüsselt (DR-Session) → packt in festes Frame → Relay → Empfänger zieht → entschlüsselt.
4) Push: APNs/FCM nur als „Wecksignal“, nie mit Content/Meta.

## Storage
- **Client:** verschlüsselte Key-Vaults; Nachrichten optional lokal bis X Tage.
- **Server:** verschlüsselte Umschläge (TTL 72 h), keine Nutzerprofile, keine IP-Logs.

## Skalierung
- Horizontal skalierbarer Relay (Sharding nach Mailbox-ID)
- Später: Mixnet-Schicht, MLS für Gruppen
