# File manager in RUST

Rychlý správce souborů napsaný v Rustu. Jádro (práce se soubory) je čistý Rust bez UI,
okno a mobilní aplikaci dělá [Tauri 2](https://tauri.app). **Jeden kód, stejná aplikace
na Windows i na Androidu.**

> Projekt je hobby, určený pro vlastní PC a vlastní telefon. Pracuje s opravdovými soubory
> (maže, přesouvá, přepisuje), takže ho zkoušej nejdřív na nedůležitých složkách.

## Co aplikace umí

| | |
|---|---|
| **Procházení** | seznam nebo mřížka, řazení (název / velikost / datum / typ), skryté soubory, drobečková navigace, Zpět / Nahoru / Domů, tlačítka disků (C:, D:…) |
| **Cesta** | klik na ✎ / Ctrl+L = přepsání celé cesty textem (Enter přejde, Esc zruší) |
| **Postranní panel** | Místa (domov, Stažené, Dokumenty…, disky s volným místem, na Androidu i SD karta) a vlastní Připnuté složky |
| **Karty** | více karet v každém panelu, každá s vlastní složkou a historií |
| **Dva panely** | vedle sebe, jen na desktopu; kopírování a přesun do druhého panelu |
| **Miniatury** | v mřížce skutečné náhledy obrázků (jpg, png, gif, webp, bmp), generuje je Rust |
| **Hledání** | v názvu i v obsahu textových souborů; filtry typ / velikost / datum / hloubka; zrušitelné |
| **Kopírování a přesun** | složky i soubory, ukazatel průběhu a tlačítko Zrušit, kolize názvů `foto (1).jpg` |
| **Koš** | mazání do koše (Windows: systémový koš, Android: vlastní koš aplikace); položka „Koš“ dole v postranním panelu – obnovit, smazat nevratně, vysypat |
| **ZIP** | zabalit výběr, rozbalit archiv (s ochranou proti `../` cestám) |
| **Analýza místa** | co ve složce zabírá nejvíc, s proklikem do podsložek |
| **Náhled** | obrázky, audio, video a text přímo v aplikaci; „Otevřít v systému“ |
| **Telefon** | tažením seznamu dolů se obnoví; obsah se drží mezi stavovou a navigační lištou (hodiny, baterie, tlačítka Zpět/Domů) |
| **Paměť** | po startu se obnoví karty, řazení, zobrazení, připnuté složky |

Podrobně v [uživatelské příručce](docs/UZIVATELSKA-PRIRUCKA.md).

## Rychlý start

Potřebuješ jen [Rust](https://rustup.rs). Na Windows:

```powershell
git clone https://github.com/mrtwister99-png/filemanagerinpurerust.git
cd filemanagerinpurerust
cargo run -p filemanager
```

První překlad trvá několik minut, další jsou rychlé.

Do telefonu (Android) je potřeba navíc Java, Android SDK/NDK a Tauri CLI –
celý postup je v [docs/INSTALACE.md](docs/INSTALACE.md).

## Dokumentace

| Dokument | O čem |
|---|---|
| [docs/UZIVATELSKA-PRIRUCKA.md](docs/UZIVATELSKA-PRIRUCKA.md) | jak aplikaci používat |
| [docs/INSTALACE.md](docs/INSTALACE.md) | instalace nástrojů, build pro PC a Android, nahrání do telefonu, řešení problémů |
| [docs/ARCHITEKTURA.md](docs/ARCHITEKTURA.md) | jak je projekt poskládaný, moduly jádra, příkazy, události, stav |
| [docs/VYVOJ.md](docs/VYVOJ.md) | vývoj, testování, jak přidat funkci, poučení z potíží |
| [CHANGELOG.md](CHANGELOG.md) | co přibylo a kdy |

## Struktura repozitáře

```
crates/filecore/        čistý Rust – soubory, hledání, ZIP, miniatury, přenosy (bez Tauri, bez UI)
apps/filemanager/
  src-tauri/            Tauri backend (tenké příkazy nad filecore) + konfigurace
  src/                  frontend: vanilla HTML / CSS / JS, žádný framework ani build krok
cli/                    pomocné příkazy pro zkoušení jádra v terminálu
scripts/                build skripty pro Android a vývojový běh na desktopu
docs/                   dokumentace
```

## Technologie

Rust 2021 · Tauri 2 · `rayon` (paralelní hledání a velikosti) · `walkdir` · `image`
(jen pure-Rust dekodéry) · `zip` (jen deflate) · `fs4` (volné místo) · `trash` (jen desktop).
Závislosti jádra jsou schválně bez C knihoven, aby se vše bez potíží přeložilo i pro Android.

## Licence

[MIT](LICENSE) – kód můžeš volně používat, upravovat i šířit, stačí ponechat jméno autora
a text licence. Bez záruky.

## Stav projektu

Verze 0.1.0, funkční na Windows 11 a na Androidu (debug build). Vyzkoušeno hlavně na
Windows; na telefonu zatím jen základní spuštění a výpis souborů. Co chybí a co se
dá vylepšit, je v sekci „Nápady“ v [docs/VYVOJ.md](docs/VYVOJ.md).
