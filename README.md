# ZUGFeRD Studio ⚡

> **Lokaler, quelloffener Desktop-Editor & Viewer für ZUGFeRD / Factur-X E-Rechnungen**  
> Plattformübergreifend für Windows, macOS und Linux. 100 % lokal, ohne Cloud-Zwang, ohne Telemetrie.

---

## Über das Projekt

Mit der E-Rechnungspflicht in Deutschland und Europa gewinnen hybride Rechnungsformate wie **ZUGFeRD** und **Factur-X** (PDF/A-3 mit eingebettetem UN/CEFACT CII XML) rasant an Bedeutung. 

**ZUGFeRD Studio** ist eine leichtgewichtige, performante Desktop-Applikation, die es Endanwendern, Freiberuflern und Entwicklern ermöglicht:
- PDF-Rechnungen visuell zu prüfen und gleichzeitig den maschinenlesbaren XML-Datenstrom strukturiert einzusehen.
- Rechnungsdaten und Positionen mit Live-Berechnungen direkt im UI zu bearbeiten.
- Rechnungen in Echtzeit gegen die Geschäftsregeln der europäischen Norm **EN 16931** zu validieren.
- Standard-PDFs in vollwertige ZUGFeRD-Dokumente umzuwandeln oder XML-Anhänge rückstandslos zu entfernen.

---

## Features

- **Integrierter PDF-Viewer:**
  - Scharfes Rendering dank HiDPI/Retina-Skalierung via PDF.js.
  - Interaktive Seitenleiste mit Live-Miniaturansichten (Thumbnails).
  - Voller Text-Layer für pixelgenaue Textmarkierung, Suche und Zwischenablage (Copy-Paste).
- **ZUGFeRD & Factur-X Inspektion:**
  - Automatische Erkennung von `factur-x.xml`, `zugferd-invoice.xml` und `xrechnung.xml`.
  - Strukturierte Anzeige von Profil (`MINIMUM`, `BASIC`, `EN 16931 / COMFORT`, `EXTENDED`), Parteien, Beträgen und Positionen.
- **Reaktiver Rechnungs-Editor:**
  - Bearbeiten von Rechnungsnummer, Datum, Währung, Parteien und Positionen.
  - Automatische, reaktive Neuberechnung von Netto-, Steuer- und Bruttobeträgen bei jeder Eingabe.
  - Dynamisches Hinzufügen und Entfernen von Rechnungspositionen.
- **EN 16931 Live-Validierung:**
  - Prüfung auf formale Vorgaben (Rechnungsnummer, ISO-Währungscodes, Datumsformate).
  - Mathematische Konsistenzprüfung (Summe der Positionen vs. Nettogesamtbetrag, Rundungstoleranzen, Zahlbetrag).
  - Detaillierte Fehler- und Warnmeldungen mit Spezifikations-Regelcodes (z. B. `BR-CO-10`, `BR-CO-15`, `BR-02`).
- **PDF-Mutation & Lifecycle:**
  - **Zurückschreiben:** Aktualisiertes XML mit FlateDecode komprimieren und nahtlos in das PDF einbetten.
  - **Upgrade:** Normale Standard-PDFs durch Einbetten von XML und XMP-Schemas zu ZUGFeRD (PDF/A-3b) aufrüsten.
  - **Bereinigung (`strip_zugferd`):** ZUGFeRD-Anhänge, Katalog-Einträge (`/AF`, `/Names`) und XMP-Metadaten rückstandslos entfernen.
- **Export & Weiterverarbeitung:**
  - Strukturierter Datenexport nach **JSON** und **CSV** (mit `;`-Trennzeichen und UTF-8-BOM für Excel).
  - Druck- und PDF-Exportansicht (DIN A4 optimiert über `@media print`).
- **Natives Drag & Drop:**
  - PDF-Dateien oder externe XML-Dateien einfach per Drag-and-Drop auf das App-Fenster ziehen.

---

## Tech-Stack & Architektur

ZUGFeRD Studio setzt auf einen modernen, ressourcenschonenden Stack ohne Electron-Ballast:

| Ebene | Technologie | Aufgabe |
| :--- | :--- | :--- |
| **Backend & Engine** | **Rust** | Sichere Low-Level-PDF-Objektgraph-Manipulation (`lopdf`), Streaming-XML-Parsing & -Generierung (`quick-xml`), Zlib-Kompression (`flate2`), Validierungslogik. |
| **App-Container** | **Tauri v2** | Leichtgewichtige OS-Webview-Anbindung, natives Fenstermanagement, OS-Dateidialoge und sichere IPC-Bridge. |
| **Frontend UI** | **Svelte 5** | Moderne, deklarative UI mit Runes (`$state`, `$derived`, `$effect`) für zustandslose Live-Berechnungen. |
| **Dokument-Rendering** | **PDF.js** | Clientseitiges Zeichnen der PDF-Seiten auf HTML5-Canvas inklusive TextLayer-Synchronisation. |

---

## Entwicklung & Lokales Setup

### Voraussetzungen

1. **Rust Toolchain:** Aktuelle Version via [rustup.rs](https://rustup.rs/) installieren:
   ```bash
   rustc --version
   cargo --version
   ```
2. **Node.js & pnpm:** Node.js (>= 20) und `pnpm`:
   ```bash
   corepack enable
   corepack prepare pnpm@latest --activate
   ```
3. **Plattform-Abhängigkeiten (nur Linux):**
   ```bash
   sudo apt-get update && sudo apt-get install -y \
     libwebkit2gtk-4.1-dev \
     build-essential \
     curl \
     wget \
     file \
     libxdo-dev \
     libssl-dev \
     libayatana-appindicator3-dev \
     librsvg2-dev
   ```

### Projekt starten

```bash
# 1. Repository klonen
git clone https://github.com/thehmaker/zugferd-studio.git
cd zugferd-studio

# 2. Frontend-Abhängigkeiten installieren
pnpm install

# 3. Lokalen Entwicklungsmodus starten (Hot Reloading für Rust & Svelte)
pnpm tauri dev
```

---

## Produktions-Build erstellen

Um eigenständige Installer für dein aktuelles Betriebssystem zu kompilieren:

```bash
pnpm tauri build
```

Die fertigen Binaries befinden sich anschließend unter `src-tauri/target/release/bundle/`:
- **macOS:** `.dmg` / `.app`
- **Windows:** `.msi` / `.exe`
- **Linux:** `.deb` / `.AppImage`

---

## Automatisierte Multi-OS Releases (CI/CD)

Das Repository verfügt über eine GitHub Actions Pipeline (`.github/workflows/release.yml`), die bei jedem getaggten Git-Push automatisch Installer für alle drei Hauptplattformen baut und an ein GitHub Release anhängt:

```bash
git tag v1.0.0
git push origin v1.0.0
```

---

## Lizenz

Dieses Projekt ist unter der **MIT-Lizenz** lizenziert – siehe die [LICENSE](LICENSE)-Datei für Details.
