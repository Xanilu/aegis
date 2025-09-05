# ADR-001: Protokollwahl

**Entscheidung:** Start mit Signal-Style (X3DH + Double Ratchet) für 1:1 und kleine Gruppen.  
**Begründung:** Bewährt, starke Security-Eigenschaften, gute Bibliotheken/Referenzen.  
**Alternativen:** MLS (später für große Gruppen), Matrix/Olm (mehr Komplexität), Eigenbau Noise-basiert (mehr Audit-Bedarf).  
**Risiken:** Lizenz/Bin dings (libsignal). **Mitigation:** Falls nötig, Noise-basierte Implementierung + externes Audit.
