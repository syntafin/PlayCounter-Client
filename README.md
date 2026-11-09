# PlayCounter

Eigenständige Desktop-App zur automatischen Erfassung von Spielzeiten und Gaming-Sitzungen. Läuft im Hintergrund im System-Tray, überwacht konfigurierte Spiele und bietet eine Spielebibliothek mit detaillierten Sitzungsstatistiken, Historie sowie Import- und Backup-Funktionen.

Die App funktioniert vollständig autark (Offline / SQLite) und kann optional mit einem kompatiblen Sync-Server verbunden werden.

## Voraussetzungen

- **Node.js:** v24 (LTS) oder neuer
- **Rust:** Aktuelle Stable-Version (`rustup`)
- **Windows:** Visual Studio C++ Build Tools und WebView2 Runtime
- **Linux (Debian / Ubuntu):**
  ```bash
  sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
  ```
- **Linux (Arch Linux / CachyOS):**
  ```bash
  sudo pacman -S webkit2gtk-4.1 base-devel curl wget openssl libappindicator-gtk3 librsvg
  ```

---

## Entwicklung

```bash
# Abhängigkeiten installieren
npm install

# Anwendung im Dev-Modus starten (Vite + Tauri Fenster mit Hot-Reload)
npx @tauri-apps/cli dev

# Nur Web-Frontend im Browser ausführen
npm run dev

# Frontend-Build und TypeScript-Prüfung
npm run build

# Rust-Backend prüfen und Unit-Tests ausführen
cd src-tauri
cargo check
cargo test
```

---

## Lokaler Build

### Windows
```powershell
# Nur Release-Binary kompilieren:
.\build.ps1

# Vollständigen NSIS-Setup-Installer erstellen:
.\build.ps1 -Bundle
```

Der Installer liegt anschließend unter `.\target\release\bundle\nsis\PlayCounter_<version>_x64-setup.exe`.

### Linux
```bash
# Baut die konfigurierten Linux-Pakete (.deb und .AppImage):
npx @tauri-apps/cli build
```

---

## Installation & Pakete

### Windows
Installer `PlayCounter_<version>_x64-setup.exe` aus den [GitHub Releases](https://github.com/syntafin/PlayCounter-Client/releases) herunterladen und ausführen.

### Linux (Debian / Ubuntu)
```bash
sudo dpkg -i playcounter_<version>_amd64.deb
sudo apt-get install -f
```

### Linux (Arch Linux / CachyOS)

- **AppImage:**
  ```bash
  chmod +x PlayCounter_<version>_amd64.AppImage
  ./PlayCounter_<version>_amd64.AppImage
  ```

- **Pacman-Paket über PKGBUILD:**
  ```bash
  cd packaging/arch
  makepkg -si
  ```

---

## Versionierung & Release

Ein einziger Befehl aktualisiert alle Manifeste (`package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `packaging/arch/PKGBUILD`), committet die Änderungen, vergibt das passende Git-Tag und pusht automatisch zu GitHub:

```bash
# Nächstes Patch-Release (z.B. 0.2.4 -> 0.2.5), committen, taggen und pushen:
npm run bump
# oder:
npm run release

# Spezifische Version oder SemVer-Schritt:
npm run bump 0.3.0
npm run bump minor
npm run bump major

# Nur Versionsdateien anpassen ohne automatischen Git-Push:
npm run bump 0.3.0 -- --no-push
npm run bump 0.3.0 -- --no-git
```

---

## CI/CD (GitHub Actions)

Die Workflows liegen unter `.github/workflows/`:

- **`release.yml`:** Baut bei jedem Tag `v*` oder manuellem Start (`workflow_dispatch`):
  - Windows: NSIS Installer (`.exe`)
  - Linux: Debian-Paket (`.deb`) und universelles `.AppImage`
  Erstellt das GitHub Release und lädt die Artefakte sowie `latest.json` für den In-App-Updater hoch.
- **`check.yml`:** Führt bei jedem Push auf `main` und bei Pull Requests Typechecks (`npm run build`), `cargo check` und `cargo test` aus.

---

## Konfiguration (`config.json`)

Die Konfiguration wird im Anwendungsverzeichnis als `config.json` abgelegt:

```json
{
  "db_path": "gaming.db",
  "scan_interval_secs": 15,
  "heartbeat_interval_secs": 900,
  "sync_interval_secs": 30,
  "config_refresh_interval_secs": 3600,
  "watched_games": [
    "StarCitizen",
    "ffxiv_dx11",
    "helldivers2"
  ],
  "api_enabled": false,
  "api_base_url": "",
  "api_token": ""
}
```

| Parameter | Beschreibung |
|---|---|
| `db_path` | Pfad zur lokalen SQLite-Datenbankdatei |
| `scan_interval_secs` | Intervall in Sekunden zur Überprüfung laufender Spielprozesse (Standard: `15`) |
| `heartbeat_interval_secs` | Intervall für Heartbeat-Meldungen aktiver Sitzungen (Standard: `900` / 15 Min) |
| `sync_interval_secs` | Intervall zur Übertragung ungesynchronisierter Events an die API (Standard: `30`) |
| `watched_games` | Liste der überwachten Prozessnamen (mit oder ohne `.exe`) |
| `api_enabled` | Aktiviert oder deaktiviert die optionale Cloud-Synchronisation (Standard: `false`) |
| `api_base_url` | Optionaler REST-Endpunkt des Sync-Servers |
| `api_token` | Optionaler API-Token zur Server-Authentifizierung |
