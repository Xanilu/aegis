# Relay-API (v0.1)

Base: `https://relay.aegis/`

## Auth
- Anonyme Tokens via Privacy-Pass ODER PoW-Proof im Header für Erstkontakte.

## Endpunkte
### `PUT /v1/envelopes`
- Body: `cipher_blob` (feste Größe, z. B. 4096 B), `to_id`
- Effekt: Speichert Umschlag mit TTL.

### `GET /v1/mailbox?since=<token>`
- Antwort: Liste `cipher_blob`s + neuer `since`-Token (Opaque Cursor).
- Hinweis: Keine Absender-Metadaten.

### `POST /v1/token`
- Optional: Privacy-Pass Token holen/verlängern (Blind Signatures).

## Richtlinien
- Rate-Limits pro anonymer Token/Proof.
- Keine Logs personenbezogener Daten.
