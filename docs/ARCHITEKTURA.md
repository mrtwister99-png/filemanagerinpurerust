# Architektura

## Přehled

```
┌──────────────────────────────────────────────┐
│ Frontend  (apps/filemanager/src)             │  vanilla HTML / CSS / JS, jeden IIFE
│  index.html · main.js · styles.css           │  žádný framework, žádný build krok
└───────────────┬──────────────────────────────┘
                │ window.__TAURI__.core.invoke(...)   + události (listen)
┌───────────────▼──────────────────────────────┐
│ Tauri backend (apps/filemanager/src-tauri)   │  tenké příkazy, async + spawn_blocking
│  lib.rs · main.rs · commands.rs              │
└───────────────┬──────────────────────────────┘
                │ běžné volání funkcí
┌───────────────▼──────────────────────────────┐
│ filecore  (crates/filecore)                  │  čistý Rust, žádný Tauri ani UI
│  scanner · search · ops · transfer · archive │  testovatelný bez okna
│  thumbs · sizes · places · models · format   │
└──────────────────────────────────────────────┘
```

Zásady:

1. **Veškerá logika je v `filecore`.** Příkazy v Tauri jsou jen tenké obálky; frontend nic
   nedělá se soubory sám.
2. **Jádro nezná Tauri ani UI** – dá se testovat (`cargo test -p filecore`) a používat
   z terminálu (`cli/`).
3. **Jádro používá jen čisté Rust závislosti** (bez C knihoven), aby se přeložilo i pro
   Android bez další toolchain.
4. **Cokoli, co může trvat, běží mimo hlavní vlákno** a jde zrušit.
5. Frontend je záměrně bez frameworku a bez build kroku (HTML se vkládá do exe při překladu).

## Workspace

| Cesta | Crate | Účel |
|---|---|---|
| `crates/filecore` | `filecore` | jádro |
| `apps/filemanager/src-tauri` | `filemanager` (lib `filemanager_lib`) | Tauri aplikace |
| `cli` | `filemanager-cli` | pomocné CLI nad jádrem |

`apps/filemanager/src-tauri` je zároveň **lib** a **bin**: celá aplikace (`run()`) je v `lib.rs`
s atributem `#[cfg_attr(mobile, tauri::mobile_entry_point)]` (Android potřebuje knihovnu
`cdylib`), `main.rs` jen volá `filemanager_lib::run()` pro desktop.

Soubor `.cargo/config.toml` nastavuje pro Android targety (`aarch64`, `x86_64`) linker flag
`-Wl,-z,max-page-size=16384`, aby byla knihovna zarovnaná na 16KB stránky (Android 15+).

`scripts/android_manifest_perms.py` (volaný z build skriptů) navíc přepíše vygenerovaný
`MainActivity.kt` (složka `gen/` není v gitu): aktivita je „od kraje ke kraji“ (Android 15+),
ale obsah posune o `systemBars + displayCutout + ime` (`setOnApplyWindowInsetsListener`),
nastaví tmavé pozadí a světlé ikony lišt. Díky tomu aplikace nezajíždí pod hodiny/baterii ani
pod tlačítka Zpět/Domů a klávesnice odsune obsah.

Profily (`[profile.release]` a optimalizace obrazových knihoven v `[profile.dev.package.*]`)
musí být v **kořenovém** `Cargo.toml`, v podřízeném crate se ignorují.

## Jádro: `crates/filecore`

| Modul | Hlavní funkce | Poznámky |
|---|---|---|
| `models.rs` | `FileEntry`, `FileKind`, `FileInfo`, `Place`, `SpaceInfo`, `SizeEntry`, `SearchResult` | datové typy, serializují se do JSON pro frontend |
| `scanner.rs` | `list_dir_opts`, `list_roots` | výpis složky (složky první, abecedně); kořeny podle platformy |
| `search.rs` | `search_with`, `SearchOptions` | hledání s filtry; `walkdir` + `rayon::par_bridge`, strom se nedrží celý v paměti; zrušitelné; obsah jen u textu do 1 MB |
| `ops.rs` | `create_dir`, `rename_entry`, `remove_entry`, `trash_entry`, `copy_entry`, `move_entry`, `get_info`, `read_text_preview` | jednoduché operace; `copy_entry`/`move_entry` jsou obálky nad `transfer` |
| `transfer.rs` | `transfer_entries`, `TransferProgress`, `TransferSummary` | kopie/přesun po blocích 1 MiB s hlášením průběhu a zrušením |
| `bin.rs` | `bin_list`, `bin_restore`, `bin_purge`, `bin_empty`, `bin_put`, `BinItem`; modul `appbin` | koš: Windows/Linux systémový přes `trash::os_limited`, Android vlastní koš aplikace (viz níže) |
| `archive.rs` | `zip_entries`, `unzip_archive` | ZIP (deflate); `enclosed_name` proti zip-slip; unikátní názvy |
| `thumbs.rs` | `thumbnail_jpeg` | miniatura JPEG (q80), respektuje EXIF orientaci, soubory nad 64 MB odmítá |
| `sizes.rs` | `child_sizes`, `dir_size` | rekurzivní velikosti paralelně, symlinky se nesledují, zrušitelné |
| `places.rs` | `list_places`, `disk_space`, `external_volumes` | místa pro postranní panel; na Androidu i SD karty (`/storage/*` kromě `emulated`, `self`) |
| `format.rs` | `format_size`, `extension_of` | pomocné |

### Zrušení

Dlouhé funkce dostávají `is_cancelled: &dyn Fn() -> bool` a pravidelně se ptají. Jádro tak
nezná žádný konkrétní mechanismus rušení.

### Koš (`bin.rs`)

Jedno API (`BinItem { id, name, original_dir, deleted_unix, size, is_dir }`), dvě implementace:

- **Windows / Linux**: `trash::os_limited::{list, metadata, restore_all, purge_all}`; `id` je
  identifikátor položky v systémovém koši. Seznam se vždy znovu načte a položky se vyberou
  podle `id`.
- **Android** (`appbin`): systémový koš není, proto má každé úložiště skrytou složku
  `.FileManagerTrash/`. Smazaná položka dostane složku `<id>/` s `meta.json`
  (původní cesta, čas, velikost) a samotnou položkou. `id` v UI je cesta ke složce
  `<id>/`; před obnovením a smazáním se ověřuje, že leží **přímo** v některém koši
  (ochrana proti smazání cizí cesty). Přesun do koše používá `move_entry` (rychlý
  `rename`, mezi disky kopie + smazání). Koš nelze hodit do sebe.

Kód `appbin` se překládá na všech platformách, takže ho jde testovat na PC
(`cargo test -p filecore`).

### Kopírování a přesun (`transfer.rs`)

- Před startem se změří celková velikost a počet souborů (`measure`) → ukazatel zná 100 %.
- Soubor se kopíruje po 1 MiB; po každém bloku se kontroluje zrušení a (nejvýše ~12× za
  sekundu, `TICK = 80 ms`) se volá `on_progress`.
- Cíl je vždy **nový** (unikátní název), takže při chybě/zrušení se rozdělaný cíl bezpečně
  smaže a zdroj zůstane nedotčený. Postup se u takové položky vrátí zpět.
- Přesun zkusí `fs::rename` (v rámci jednoho disku okamžitý); jinak kopie + smazání zdroje
  až po úspěšné kopii.
- Ochrany: složku nejde kopírovat/přesunout do ní samotné; přesun do stejné složky je no-op.
- Symlinky se při kopírování složek přeskakují.

## Tauri backend: `src-tauri/src/commands.rs`

Příkazy, které mohou trvat, jsou `async` a práci dělají ve `spawn_blocking`
(neasynchronní příkazy běží na hlavním vlákně a zamrazily by okno).

| Příkaz | Co dělá |
|---|---|
| `list_dir_cmd(path, show_hidden)` | výpis složky |
| `list_roots_cmd()` | kořenové cesty (domov, standardní složky, disky) |
| `list_places_cmd()` | místa pro postranní panel s volným místem |
| `search_cmd(dir, opts, job)` | hledání s filtry, zrušitelné klíčem `search-<job>` |
| `dir_sizes_cmd(path, job)` | velikosti potomků, klíč `sizes-<job>` |
| `thumbnail_cmd(path, max)` | JPEG miniatura jako binární odpověď |
| `transfer_cmd(paths, dest, mode, job)` | kopie (`"copy"`) / přesun (`"move"`) s událostmi průběhu, klíč `transfer-<job>` |
| `cancel_job_cmd(job)` | zruší běžící úlohu podle klíče |
| `zip_cmd(paths, dest, name)` / `unzip_cmd(path)` | ZIP |
| `delete_cmd(path, permanent)` | do koše, nebo nevratně (vždy nevratně na Androidu) |
| `trash_available_cmd()` | je k dispozici koš? (Windows i Android ano) |
| `bin_list_cmd()` | obsah koše |
| `bin_restore_cmd(ids)` | obnoví položky na původní místo, vrací počet |
| `bin_purge_cmd(ids)` / `bin_empty_cmd()` | smaže vybrané / všechny položky koše nevratně |
| `create_dir_cmd`, `rename_cmd`, `get_info_cmd` | jednoduché operace |
| `read_bytes_cmd(path, max_bytes)` | surové bajty pro náhled (binární `ipc::Response`, ne base64) |
| `read_text_cmd(path)` | textový náhled |
| `open_with_system_cmd(path)` | otevře v systémovém programu (plugin opener) |
| `storage_state_cmd()` | `"granted"` / `"missing"` na Androidu, `"desktop"` jinde |

### Zrušitelné úlohy (generace)

`jobs()` je mapa `klíč → generace`. Nová úloha se stejným klíčem zvýší generaci; starší
úloha při další kontrole zjistí, že už není aktuální (`job_current`), a skončí. `cancel_job_cmd`
jen zvýší generaci, čímž úlohu zruší. Klíče: `search-<panel>`, `sizes-<panel>`, `transfer-<n>`.

### Události

| Událost | Payload |
|---|---|
| `transfer-progress` | `{ job, done_bytes, total_bytes, done_files, total_files, current }` |

Frontend se přihlásí přes `window.__TAURI__.event.listen` (zapnuto `withGlobalTauri`).

### Oprávnění Tauri

`src-tauri/capabilities/default.json` povoluje `core:default`, `opener:default`, `fs:*`
a `dialog:default`. Vlastní příkazy (`invoke_handler`) se registrují v `lib.rs`.

## Frontend: `apps/filemanager/src`

Všechen kód je v jediném IIFE v `main.js`, aby jeho proměnné nekolidovaly s globálními
jmény, která do stránky vkládá Tauri (např. `isTauri`).

### Stav

```
app   = { panes[2], active, dual, view, showHidden, pins[], clip, … }
pane  = { idx, tabs[], tab, sort, sel:Set, query, filtry, analýza, … }
tab   = { path, history[] }
```

DOM panelu se vytváří z `<template>` v `index.html`. Existuje jeden nebo dva panely;
`activePane()` vrací ten, který ovládá horní lišta a postranní panel.

### Perzistence

`localStorage` klíč `fm.state.v1`: karty (cesta + historie, posledních 30), aktivní karta,
řazení, aktivní panel, dva panely, zobrazení, skryté soubory, připnuté složky. Ukládá se
s prodlevou po změně; při načtení se neexistující složky potichu nahradí domovskou.

### Miniatury

`IntersectionObserver` načítá miniatury líně, najednou nejvýše 4 požadavky.
Výsledek (`blob:` URL) se cachuje v paměti podle `cesta + mtime + velikost`, limit 300
položek (nejdéle nepoužité se vyhazují a jejich URL se uvolní).

### Hledání

Prodleva ~250 ms po psaní; každý dotaz nese `job = search-<panel>`; frontend zahazuje
zastaralé odpovědi a nový dotaz zruší starý i v backendu.

### Upravitelná cesta

`renderCrumbs` kreslí drobečky + `✎`; `startEditPath(p)` je nahradí textovým polem
(`p.editingPath` brání přepsání při překreslení), Enter volá `goToTypedPath` – nejdřív ověří
cestu přes `list_dir_cmd` (chyba → dialog, původní složka zůstane), pak `openPath`.
Zkratka Ctrl+L je globální a platí pro aktivní panel.

### Koš a tažení v UI

Panel má režim `binMode` (podobně jako `sizesMode`): `startBin` / `loadBin` / `renderBin`;
otevření složky z něj vyjde. Tažení dolů (`wirePullRefresh`) používá dotykové události na
posuvném kontejneru panelu: spustí se jen při `scrollTop == 0`, převažuje-li svislý pohyb;
práh 64 px; po puštění se zavolá `refresh(p)` a `loadPlaces()`. CSS `overscroll-behavior-y: contain`
brání dalšímu posunu stránky.

### Dialogy

`window.prompt/confirm/alert` ve webview nefungují spolehlivě, proto vlastní
`askText`, `askConfirm` (volitelně s checkboxem) a `notify`. Okno průběhu je `#progress`.

### Odezva na šířku okna

- ≥ 900 px: postranní panel je stále vidět, jsou k dispozici dva panely.
- < 900 px: postranní panel je vysouvací (☰), dva panely se skryjí (preference zůstává uložená).
- ≤ 640 px: kompaktnější seznam, v úzkých panelech se skrývá sloupec s datem.

## Platformní rozdíly

| | Windows | Android |
|---|---|---|
| Kořeny / místa | domov, standardní složky, disky `C:`–`Z:` | `/storage/emulated/0`, jeho standardní složky, SD karty |
| Mazání | systémový koš (`trash`) nebo nevratně | vlastní koš aplikace (`.FileManagerTrash`) nebo nevratně |
| Dva panely | ano (≥ 900 px) | ne |
| Oprávnění | – | `MANAGE_EXTERNAL_STORAGE` (Přístup ke všem souborům) |
| `trash` crate | ano | nepřekládá se (`cfg(not(target_os = "android"))`) |
