# Vývoj

## Každodenní práce

```powershell
cargo run -p filemanager            # přeloží a spustí desktopovou aplikaci
cargo test -p filecore              # testy jádra (pracují jen v dočasných složkách)
cargo build -p filemanager-cli      # pomocné CLI
cargo check -p filecore --target aarch64-linux-android   # jádro se přeloží i pro Android?
```

Před každým překladem aplikace zavři běžící okno (exe je zamčené):
`taskkill /F /IM filemanager.exe`.

Pomocné CLI umí zkoušet jádro bez okna:

```powershell
cargo run -p filemanager-cli -- list C:\Users\jmeno
cargo run -p filemanager-cli -- search C:\Users\jmeno\Documents faktura --max 20
cargo run -p filemanager-cli -- sizes C:\Users\jmeno\Downloads
cargo run -p filemanager-cli -- places
```

(`zip`, `unzip`, `copy`, `move`, `mkdir`, `info`, `roots` také; viz `-- ` bez argumentů.)

## Poučení z potíží (aby se neopakovaly)

| Problém | Příčina | Řešení v kódu |
|---|---|---|
| Okno po startu rozbité, nic nereaguje | Tauri vkládá do stránky globální `isTauri`; vlastní `const isTauri` shodil celý skript na `SyntaxError` | veškerý JS je v jednom IIFE, vlastní funkce se jmenuje `inTauri` |
| Všechna skrytá okna (náhled, dialog…) viditelná naráz | CSS `display:flex` přebíjí atribut `hidden` | v `styles.css` je `[hidden]{display:none!important}` – nemazat |
| Okno zamrzá při hledání / kopírování | neasynchronní Tauri příkaz běží na hlavním vlákně | dlouhé příkazy jsou `async` + `spawn_blocking` |
| `prompt/confirm` nefungují | webview je neřeší spolehlivě | vlastní dialogy `askText` / `askConfirm` / `notify` |
| Změna frontendu se neprojeví | HTML/CSS/JS se vkládají do exe při překladu | po změně znovu `cargo build -p filemanager` |
| `[profile.release]` v podřízeném crate se ignoruje | profily čte jen kořen workspace | profily jsou v kořenovém `Cargo.toml` |
| Android: „no library targets found“ | Tauri pro mobil potřebuje `cdylib` | `lib.rs` + `[lib]` v `src-tauri/Cargo.toml` |
| Android: „Additional properties… 'ios'“ | klíč v `tauri.conf.json` se jmenuje `iOS` | opraveno |
| Android: „Creation symbolic link is not allowed“ | Windows bez Vývojářského režimu | zapnout režim (viz [INSTALACE.md](INSTALACE.md)) |
| Disky se vracely jako `C:\\` | zdvojené zpětné lomítko ve `format!` | opraveno v `scanner.rs` |

## Jak přidat novou funkci

Pořadí, které se osvědčilo:

1. **Jádro** (`crates/filecore`): napiš čistou funkci v příslušném modulu, exportuj ji v `lib.rs`.
   Má-li trvat dlouho, dej jí parametr `is_cancelled` (a případně `on_progress`).
   Přidej test s dočasnou složkou (`tempfile`).
2. **Příkaz** (`src-tauri/src/commands.rs`): tenká obálka; dlouhou práci obal `blocking(...)`.
   Zaregistruj ho v `generate_handler![...]` v `lib.rs`.
3. **Frontend** (`apps/filemanager/src/main.js`): zavolej `invoke("nazev_cmd", {...})`.
   Pozor na pojmenování argumentů: Tauri převádí `snake_case` v Rustu na `camelCase`
   v JS (`show_hidden` → `showHidden`).
4. **Chová-li se jinak na Androidu**, použij `#[cfg(target_os = "android")]` v jádru
   a ověř `cargo check -p filecore --target aarch64-linux-android`.
5. Doplň [uživatelskou příručku](UZIVATELSKA-PRIRUCKA.md), [architekturu](ARCHITEKTURA.md)
   a [CHANGELOG](../CHANGELOG.md).

## Testování

- **Testy jádra**: `cargo test -p filecore` – zip round-trip včetně zip-slip, filtry hledání,
  velikosti složek, kopírování s průběhem a zrušením, kolize názvů, SD karty.
- **Ruční test aplikace**: vždy na **pokusné složce** s vymyšlenými soubory, ne na vlastních
  datech (aplikace opravdu maže a přesouvá). Při testu mazání do koše nezapomeň koš vysypat.
- **Ladění webview na Windows** (volitelné): spuštění s vzdáleným laděním
  ```powershell
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9333"
  .\target\debug\filemanager.exe
  ```
  a pak <http://127.0.0.1:9333/json> ukáže stránku, ke které se dá připojit přes DevTools
  protokol (např. kvůli screenshotům nebo výpisu chyb).
- **Android**: `adb logcat -s RustStdoutStderr` ukazuje výstup Rustu a chyby webview.

## Co v repozitáři záměrně není

- `target/` – build artefakty,
- `apps/filemanager/src-tauri/gen/` – generované Tauri soubory (včetně celého Android
  projektu); vytvoří se příkazem `cargo tauri android init`,
- `*.apk`, `*.aab` – sestavené balíčky,
- podpisové klíče a hesla (nikdy je nenahrávej).

## Nápady na další vývoj

- klávesové zkratky (Ctrl+C/V/X, Delete, F2, Enter) a kontextové menu pravým tlačítkem,
- oprava 16KB zarovnání knihovny pro Android 15+ (linker flag `-z max-page-size=16384`),
- release build a podepsání APK, případně ikona aplikace,
- přesun mezi disky a velké soubory vyzkoušet i na Androidu,
- Storage Access Framework místo „Přístup ke všem souborům“ (kvůli Google Play),
- vlastní rozsahy ve filtru velikosti, ukládání filtrů hledání,
- ZIP s časy v místním čase (vyžaduje zavést podporu časových zón),
- fronta více kopírování najednou (teď běží vždy jedno).
