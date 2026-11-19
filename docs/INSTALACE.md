# Instalace a sestavení

Tento návod je ověřený na Windows 11. Na macOS a Linuxu funguje desktopová část stejně
(`bash scripts/…` místo `.bat`), Android postup je obdobný.

## 1. Co nainstalovat (jednou)

### Pro desktop (stačí jen toto)

1. **Rust**: <https://rustup.rs>. Po instalaci restartuj terminál.
   Ověření: `rustc --version`, `cargo --version`.
2. Na Windows ještě **Microsoft C++ Build Tools** (Rust je při instalaci nabídne) a
   **WebView2** (ve Windows 10/11 už bývá).

### Navíc pro Android

3. **Java 17 nebo 21**: `winget install Microsoft.OpenJDK.17`
4. **Android SDK** (Android Studio není potřeba): z
   <https://developer.android.com/studio#command-line-tools> stáhni *command line tools*
   a rozbal třeba do `C:\Android\Sdk` (nebo použij `%LOCALAPPDATA%\Android\Sdk`, pokud
   už Android Studio máš). Uvnitř musí být `cmdline-tools\latest\bin\sdkmanager`.
5. Doinstaluj komponenty:
   ```powershell
   sdkmanager "platform-tools" "platforms;android-34" "build-tools;34.0.0" "ndk;27.1.12297006"
   ```
6. **Rust targety pro Android**:
   ```powershell
   rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
   ```
7. **Tauri CLI** (kompiluje se, trvá několik minut):
   ```powershell
   cargo install tauri-cli --version "^2" --locked
   ```
   Ověření: `cargo tauri --version`.
8. **Vývojářský režim Windows** – Tauri při buildu pro Android vytváří symbolické odkazy,
   což Windows bez něj odmítnou („Creation symbolic link is not allowed“).
   Zapni: **Nastavení → Systém → Pro vývojáře → Režim pro vývojáře**
   (u některých verzí Windows 11: Soukromí a zabezpečení → Pro vývojáře).
9. Proměnné prostředí (build skripty si je umí dohledat, ale takhle je to jistější):
   - `ANDROID_HOME` = složka SDK (např. `C:\Users\jmeno\AppData\Local\Android\Sdk`)
   - `NDK_HOME` = konkrétní verze NDK (`…\Sdk\ndk\27.1.12297006`)
   - `JAVA_HOME` = složka JDK

## 2. Desktop (Windows)

Rychlý běh z terminálu:

```powershell
cargo run -p filemanager
```

nebo jen přeložit a spustit ručně:

```powershell
cargo build -p filemanager
target\debug\filemanager.exe
```

Vývoj s Tauri CLI: `scripts\dev-desktop.bat` (macOS/Linux: `bash scripts/dev-desktop.sh`).

> **Pozor:** frontend (`apps/filemanager/src`) se při překladu vkládá přímo do exe.
> Po každé změně HTML/CSS/JS je potřeba znovu `cargo build -p filemanager`
> (a běžící aplikaci předtím zavřít – spuštěné exe nejde přepsat).

### Instalátor pro Windows

```powershell
cd apps\filemanager
cargo tauri build
```

Výsledek najdeš v `target\release\bundle\`:

- `nsis\File manager in RUST_0.1.0_x64-setup.exe` (~2,7 MB) – instalátor jedním klikem,
- `msi\File manager in RUST_0.1.0_x64_en-US.msi` (~4 MB) – klasický MSI.

Aplikace se nainstaluje do nabídky Start s ikonou a jde odinstalovat v Nastavení → Aplikace.
První build stáhne nástroje WiX a NSIS (dělá to Tauri samo). Instalátor je nepodepsaný,
takže Windows SmartScreen může při prvním spuštění ukázat varování („Další informace →
Přesto spustit“).

## 3. Android – APK z terminálu

### První inicializace (jednou)

```powershell
cd apps\filemanager
cargo tauri android init
```

Vygeneruje složku `apps/filemanager/src-tauri/gen/android`. Ta se **neukládá do gitu**
(je v `.gitignore`) – po naklonování repozitáře ji znovu vytvoř tímto příkazem.

### Sestavení

Jednoduše skriptem (dohledá NDK, doplní oprávnění do manifestu, upraví `MainActivity.kt` tak,
aby obsah nelezl pod systémové lišty, a postaví APK):

```powershell
scripts\build-android.bat
```

Nebo ručně, jen pro 64bitové telefony a v debug variantě (nejrychlejší):

```powershell
cd apps\filemanager
python ..\..\scripts\android_manifest_perms.py
cargo tauri android build --apk --debug --target aarch64
```

Release varianta (mnohem menší soubor, ale chce podepsání, viz níže):

```powershell
scripts\build-android.bat --apk --release
```

První build trvá déle (stahuje se Gradle a překládá celý Rust), další jsou rychlejší.

Hotové APK:

```
apps/filemanager/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
```

> Debug APK je velké (~200 MB), protože obsahuje ladicí informace. Release bývá o řád menší.

## 4. Nahrání do telefonu

### Přes kabel (doporučeno)

1. V telefonu: **Nastavení → O telefonu** → 7× klepni na „Číslo sestavení“.
2. **Nastavení → Pro vývojáře → Ladění USB** zapnout.
3. Připoj kabel a v telefonu potvrď „Povolit ladění USB?“.
4. Ověř, že PC telefon vidí:
   ```powershell
   adb devices
   ```
   Musí tam být zařízení se stavem `device` (ne `unauthorized`).
5. Nainstaluj:
   ```powershell
   adb install -r apps\filemanager\src-tauri\gen\android\app\build\outputs\apk\universal\debug\app-universal-debug.apk
   ```

### Bez kabelu

Zkopíruj APK do telefonu (Disk, e-mail, sdílení souborů), otevři ho a povol
„Instalovat z neznámých zdrojů“.

### Rychlá iterace s logy

```powershell
cd apps\filemanager
cargo tauri android dev
```

Logy aplikace: `adb logcat -s RustStdoutStderr`.

## 5. Oprávnění na Androidu

Android 11+ používá *scoped storage*. Aplikace potřebuje **Přístup ke všem souborům**:

- manifest to řeší skript `scripts/android_manifest_perms.py` (spouští se z build skriptu;
  doplní `READ/WRITE_EXTERNAL_STORAGE`, `MANAGE_EXTERNAL_STORAGE` a `READ_MEDIA_*`),
- po instalaci povol v telefonu: **Nastavení → Aplikace → File manager in RUST →
  Oprávnění → Přístup ke všem souborům**.

Když přístup chybí, aplikace ukáže banner s návodem. Pro Google Play by se místo toho
musel použít Storage Access Framework (`MANAGE_EXTERNAL_STORAGE` Google u správců souborů
povoluje jen výjimečně); pro vlastní telefon takto stačí.

## 6. Release build a podepsání (volitelné)

Debug APK podepisuje Android Studio/Gradle automaticky ladicím klíčem. Pro release
si vytvoř vlastní klíč (`keytool -genkey -v -keystore moje.jks -keyalg RSA -keysize 2048
-validity 10000 -alias klic`) a podepiš podle
<https://v2.tauri.app/distribute/sign/android/>. **Klíč (`*.jks`) a hesla nikdy
nenahrávej do gitu.**

## 7. Řešení problémů

| Příznak | Příčina / řešení |
|---|---|
| `Additional properties are not allowed ('ios' was unexpected)` | v `tauri.conf.json` musí být klíč `"iOS"` (velké OS), ne `"ios"` |
| `no library targets found in package filemanager` | Android potřebuje knihovnu: `src-tauri/src/lib.rs` + sekce `[lib]` v `Cargo.toml` (už je opraveno) |
| `Creation symbolic link is not allowed for this system` | zapni Vývojářský režim Windows (bod 8 výše) |
| `sdkmanager: command not found` | chybí `cmdline-tools\latest\bin` v `PATH` |
| `NDK not found` | nastav `NDK_HOME` na složku s číslem verze |
| `JAVA_HOME not set` | nastav na JDK 17/21 |
| `cargo tauri: command not found` | `cargo install tauri-cli --version "^2" --locked` a restart terminálu |
| `adb devices` je prázdné | jiný kabel/port, znovu povolit Ladění USB, režim USB „Přenos souborů“ |
| `adb devices` ukáže `unauthorized` | v telefonu potvrď dotaz „Povolit ladění USB“ |
| Na telefonu obsah zajíždí pod hodiny nahoře nebo pod tlačítka dole | `MainActivity.kt` v `gen/` není upravený (po novém `cargo tauri android init` se přepíše): spusť `python scripts/android_manifest_perms.py` (nebo `scripts\build-android.bat`) a postav APK znovu |
| Na telefonu prázdné nebo nedostupné složky | chybí Přístup ke všem souborům (bod 5) |
| Android ukáže „Kompatibilita aplikací… 16KB“ | knihovna není zarovnaná na 16KB stránky (Android 15+). Řeší to `.cargo/config.toml` s linker flagem `-z max-page-size=16384`; pokud varování vidíš u starého APK, sestav a nainstaluj ho znovu. Ověření: `llvm-readelf -lW libfilemanager_lib.so \| findstr LOAD` – zarovnání musí být `0x4000` |
| `The process cannot access the file … filemanager.exe` při buildu | aplikace ještě běží: zavři ji (`taskkill /F /IM filemanager.exe`) |
| Po změně frontendu se nic nezměnilo | frontend je zabudovaný v exe – přelož znovu (`cargo build -p filemanager`) |
| `Couldn't find a .ico icon` při `cargo tauri build` | v `tauri.conf.json` musí být `bundle.icon` se seznamem ikon (už je) |
| Chci aplikaci vrátit do výchozího stavu | stav (karty, připnuté složky…) je v úložišti webview; na Windows smaž složku `%LOCALAPPDATA%\cz.tvojefm.filemanager` při zavřené aplikaci |
