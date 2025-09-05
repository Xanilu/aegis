# Threat Model (v0.1)

## Akteure
- Opportunistische Angreifer (offene WLANs)
- Netzwerkbeobachter (ISP/Backbone)
- Serverbetreiber/Insider
- Massenüberwachung/Behörden
- Kriminelle/Gegenseite
- Forensik am Endgerät (Verlust/Diebstahl)

## Schutzziele
- Vertraulichkeit (Inhalte), Integrität, Authentizität
- Metadata minimization (wer/mit wem/wann)
- Deniability (keine beweissicheren Sender-Signaturen im Payload)

## Nicht-Ziele (v1)
- Schutz gegen allmächtige Angreifer mit Physical Access + Zero-Day auf OS-Level
- Social Engineering
- Lückenlose Verfügbarkeit unter extremer Netz-Zensur

## Gegenmaßnahmen
- E2EE (X3DH + Double Ratchet)
- Sealed-Sender-Prinzip (Absender im Ciphertext)
- Feste Paketgrößen + Padding, kurze TTL, keine Logs
- Transport über TLS + optional Tor/PT
- Panik-PIN, gesicherter Key-Vault, Crash-sichere Speicherung
