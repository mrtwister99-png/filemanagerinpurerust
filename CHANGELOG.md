# Changelog

Formát vychází z [Keep a Changelog](https://keepachangelog.com/cs/1.1.0/).

## [Nevydáno]

### Opraveno
- Android: nativní knihovna je zarovnaná na 16KB stránky (`.cargo/config.toml`), zmizelo
  varování „aplikace není kompatibilní s 16KB režimem“ na Androidu 15+.

## [0.1.0] – 2026-11-19

První verze pod názvem **File manager in RUST**.

### Přidáno
- Čisté Rust jádro `filecore` (výpis, hledání, kopírování a přesun, ZIP, miniatury,
  velikosti složek, místa a volné místo na discích) a Tauri 2 aplikace nad ním.
- Postranní panel s Místy a Připnutými složkami, tlačítka disků (C:, D:…).
- Karty v panelu a režim dvou panelů (desktop, od šířky okna 900 px).
- Zobrazení seznam / mřížka s miniaturami obrázků.
- Hledání s filtry (typ, velikost, datum, hloubka, obsah textových souborů), zrušitelné.
- Kopírování a přesun s ukazatelem průběhu a tlačítkem Zrušit; zrušení uklidí rozdělanou
  položku a nesmaže zdroj.
- Mazání do systémového koše na desktopu (s volbou nevratně).
- ZIP: zabalení výběru a bezpečné rozbalení (ochrana proti zip-slip).
- Analýza místa ve složce.
- Paměť stavu: karty, řazení, zobrazení, připnuté složky, dva panely.
- Android: build z terminálu (`scripts/build-android.*`), doplnění oprávnění do manifestu,
  banner při chybějícím přístupu ke všem souborům.
- Dokumentace: README, uživatelská příručka, instalace, architektura, vývoj.

### Změněno
- Aplikace se jmenuje **File manager in RUST** (okno, titulek, název v Androidu).
- Dlouhé operace běží na pozadí (`spawn_blocking`), okno nezamrzá.
- Tauri aplikace je rozdělená na `lib.rs` + `main.rs` (nutné pro Android).

### Opraveno
- Skript se po startu nespustil kvůli kolizi jména `isTauri` s globální proměnnou Tauri.
- Skrytá okna se zobrazovala naráz (CSS přebíjelo atribut `hidden`).
- Cesty k diskům se vracely se zdvojeným lomítkem.
- Neplatný klíč `ios` v `tauri.conf.json` (správně `iOS`).
- `[profile.release]` přesunut do kořenového `Cargo.toml`, kde se skutečně použije.

### Známá omezení
- Android: zatím vyzkoušeno jen spuštění a výpis souborů, ne všechny funkce.
- Časy souborů v ZIPu jsou v UTC (Průzkumník Windows je ukáže o hodinu posunuté).
