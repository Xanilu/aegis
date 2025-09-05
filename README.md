# Aegis — Der Messenger, der dich und deine Grundrechte schützt

**Mission:** *Sprechen, wie es das Grundgesetz vorsieht – der Messenger, der dich und deine Grundrechte schützt.*

Aegis ist ein grundrechts-orientierter, anonym nutzbarer Messenger mit Ende-zu-Ende-Verschlüsselung und strenger Metadaten-Minimierung.

## Kernprinzipien
- **Anonym:** Keine Telefonnummer, keine E-Mail. Identität = Schlüssel.
- **Sicher:** E2EE mit Forward/Backward Secrecy (Signal-Style v1).
- **Metadatenarm:** Feste Paketgrößen, kurze TTL, keine Logs.
- **Geprüft:** Unabhängige Sicherheits-Audits (Closed-Source/Hybrid-Ansatz).

## Projektstatus
Phase 0 (Fundament): Architektur, Bedrohungsmodell, ADRs.  
→ Siehe `docs/` und `specs/`.

## Ordner
- `core/` – Rust Core-SDK (Kryptografie, Protokoll, Storage, FFI)
- `server/` – Minimaler Relay-Server (Rust/Go) mit Tor-Anbindung
- `app/` – Flutter App (Android, iOS, Desktop)
- `docs/` – Charta, Security, Roadmap, ADRs
- `specs/` – Threat Model, Architektur, API

## Lizenz
Siehe `LICENSE`.
