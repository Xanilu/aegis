# ADR-002: Tech-Stack

**Entscheidung:** Core in Rust (Sicherheit, Performance, FFI), App in Flutter (Android/iOS/Desktop), Server in Rust/Go.  
**Begründung:** Einmal sicher, mehrfach nutzbar. Flutter beschleunigt Multi-Plattform.  
**Risiken:** FFI-Komplexität, iOS-Background-Limits. **Mitigation:** klare FFI-Schnittstellen; Pull-On-Open UX.
