# Security Policy

- **Krypto-Standards:** X25519/Ed25519, X3DH + Double Ratchet (v1), später MLS für Gruppen.
- **Keys:** Nur clientseitig. Server sieht keine privaten Schlüssel.
- **Transporte:** TLS + optionale Tor/Pluggable Transports. Feste Paketgrößen + Padding.
- **Speicherung:** Minimal, nur verschlüsselte Umschläge mit kurzer TTL (z. B. 72 h).
- **Reports:** Responsible Disclosure per Security-Mail (später Bug-Bounty).
