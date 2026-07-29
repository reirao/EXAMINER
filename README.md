# EXAMINER

EXAMINER ist unser experimentelles Modding-Werkzeug fuer
[Exanima](https://store.steampowered.com/app/362490/Exanima/). Es verwaltet
Spielinstanzen, Profile, Mods und deren Ladereihenfolge. Unser Schwerpunkt liegt
auf direkter, kontrollierbarer Bedienung und einem Workflow, der sich spaeter
weiter automatisieren laesst.

> [!WARNING]
> EXAMINER befindet sich in aktiver Entwicklung. Lege vor Modding-Versuchen
> Sicherungen deiner Spielstaende und Spieldateien an.

## Aktueller Stand

- Exanima-Instanzen und Profile verwalten
- Mods aktivieren, deaktivieren und sortieren
- Verbesserte Drag-Steuerung mit sichtbarem Griff, laufender Rueckmeldung,
  vertikaler Begrenzung und Abbruchbehandlung
- Asset- und Framework-Werkzeuge aus der Emtk-Basis

## Bauen

Voraussetzungen sind Git und die in `rust-toolchain.toml` festgelegte
Rust-Toolchain. Danach im Projektordner:

```powershell
cargo build
```

Der Launcher ist das Standardmitglied des Cargo-Workspaces. Die angepasste
`iced_table`-Komponente liegt reproduzierbar im Branch `iced-table` dieses
Repositories und wird von Cargo automatisch geladen.

## Projektstruktur

- `emtk_launcher`: Launcher und Benutzeroberflaeche
- `emtk_core`: Instanzen, Profile, Mods und gemeinsame Logik
- `emtk_framework`: Laufzeit-Framework und Plugin-Unterstuetzung
- `emtk_asset`: Lesen und Schreiben von Exanima-Assets
- `crates/detours`: native Hooking-Abhaengigkeit

## Herkunft und Lizenz

EXAMINER baut auf dem freien
[Exanima Modding Toolkit](https://codeberg.org/ExanimaModding/Toolkit) auf. Die
urspruenglichen Autoren und ihre Git-Historie bleiben erhalten. Unsere
EXAMINER-Aenderungen werden in diesem Repository weiterentwickelt.

Der Code steht gemaess den vorhandenen Lizenzdateien unter MIT oder Apache-2.0.
EXAMINER ist nicht mit Bare Mettle Entertainment, Exanima oder Sui Generis
verbunden.
