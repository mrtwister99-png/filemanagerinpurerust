# Uživatelská příručka

Ovládání je stejné na PC i na telefonu. Rozdíly jsou vždy zmíněné.

## Horní lišta

| Tlačítko | Co dělá |
|---|---|
| ☰ | jen na telefonu: otevře postranní panel (Místa, Připnuté) |
| **C:**, **D:**… | skočí rovnou na daný disk (jen Windows) |
| **Domů** | vrátí do domovské složky (na Androidu `/storage/emulated/0`) |
| **← Zpět** | zpět v historii dané karty |
| **↑ Nahoru** | o úroveň výš |
| **+ Složka** | vytvoří novou složku v aktuální složce |
| **◫ Dva panely** | zapne druhý panel; jen na desktopu a od šířky okna cca 900 px |
| **⋯** | menu: seznam/mřížka, skryté soubory, analýza místa, připnout složku, nová karta |

## Procházení souborů

- **Klik na složku** ji otevře, **klik na soubor** otevře náhled.
- Zaškrtávací políčko vlevo vybírá položky (viz Výběr).
- Pod lištou je **drobečková navigace** (`Domů / C: / Users / …`) – každý díl je klikací.
- **Řazení** (Název / Velikost / Datum / Typ) je vedle vyhledávacího pole. Složky jsou vždy první.
- Tlačítko s mřížkou přepíná **seznam ↔ mřížka**. V mřížce mají obrázky miniatury.
- **Skryté soubory** (začínají tečkou) se zapínají v menu `⋯`.

## Postranní panel

- **Místa**: Domů, Plocha, Stažené, Dokumenty, Obrázky, Hudba, Videa a disky s ukazatelem
  volného místa. Na Androidu interní úložiště, jeho standardní složky a SD karty.
- **Připnuté**: tvoje oblíbené složky. Připneš je tlačítkem 📌 u složky v seznamu, nebo
  `📌 +` pro aktuální složku. Odepnutí stejným tlačítkem.
- Na širokém okně je panel vidět pořád, na telefonu se vysouvá přes ☰.

## Karty a dva panely

- **`+`** vedle karet otevře novou kartu na aktuální složce. Každá karta má svou složku
  a svou historii. Zavření karty křížkem (nebo prostředním tlačítkem myši).
- **Dva panely**: každý panel má vlastní karty, řazení a výběr. Aktivní panel je
  zvýrazněný oranžově; horní lišta, tlačítka disků i postranní panel ovládají ten aktivní.
  Klepnutím do panelu ho aktivuješ.

## Výběr a hromadné akce

Zaškrtni jednu nebo více položek – objeví se lišta s akcemi:

| Akce | Popis |
|---|---|
| **Kopírovat / Přesunout** | uloží výběr do schránky; v cílové složce klikni **Vložit** |
| **Do druhého panelu** | jen při dvou panelech: rovnou zkopíruje/přesune do složky druhého panelu |
| **Zabalit do ZIP** | zeptá se na název archivu, vznikne v aktuální složce |
| **Smazat** | viz Mazání |
| **Zrušit** | zruší výběr |

Pokud cíl už obsahuje stejný název, nový soubor dostane číslo: `foto.jpg` → `foto (1).jpg`.
Nic se nepřepisuje.

### Průběh a zrušení

Delší kopírování a přesun ukáže okno s názvem souboru, procenty, velikostí a počtem souborů.
Tlačítko **Zrušit** (nebo klávesa Esc) operaci zastaví:

- už hotové položky zůstanou,
- rozdělaná položka se uklidí (nezůstane poloviční soubor),
- u přesunu se zdroj maže až po úplném zkopírování, takže zrušením o nic nepřijdeš.

Přesun v rámci jednoho disku je jen přejmenování a proběhne okamžitě.
Najednou může běžet jen jedna operace.

## Mazání

- **Windows**: smazání jde do **Koše**. Chceš-li smazat nevratně, zaškrtni v potvrzení
  „nevratně“, nebo klikni na 🗑 se stisknutým **Shiftem**.
- **Android**: systémový koš není, mazání je vždy **nevratné** – potvrzovací okno to říká.

## Přejmenování a nová složka

Tlačítko ✎ u položky přejmenuje, **+ Složka** vytvoří novou. Názvy s `/`, `\` nebo `..`
se odmítnou.

## Hledání

Pole „Hledat v této složce…“ hledá při psaní (s malou prodlevou) v aktuální složce
a všech podsložkách. Starší hledání se samo zruší, když napíšeš další písmeno.

Tlačítko **⚙ Filtry**:

| Filtr | Možnosti |
|---|---|
| Typ | obrázek, video, audio, dokument, archiv, kód, ostatní |
| Velikost | do 1 MB, 1–100 MB, nad 100 MB, nad 1 GB |
| Změněno | dnes, 7 dní, 30 dní, rok |
| Hloubka | 3, 6, **12** (výchozí), 40 úrovní podsložek |
| Hledat i v obsahu | prohledá i text uvnitř textových souborů (do 1 MB, binární se přeskočí) |

U výsledků je vidět, v které podsložce leží. Výsledků je nejvýše 500 (aplikace to napíše).
Skryté složky se prohledávají jen při zapnutých skrytých souborech.

## Analýza místa

Menu `⋯` → **Analýza místa**. Spočítá velikost každé položky ve složce (včetně podsložek),
seřadí od největší a nakreslí pruhy. Klik na složku jde dovnitř, „O úroveň výš“ zpět,
„Zavřít analýzu“ vrátí běžný pohled. Počítá se na pozadí; u velkých složek se chvíli
ukazuje „Počítám…“. Symbolické odkazy se nesledují.

## ZIP

- **Zabalit**: vyber položky → **Zabalit do ZIP** → název. Složky se balí i s obsahem.
  Když archiv s názvem existuje, dostane číslo.
- **Rozbalit**: u souboru `.zip` tlačítko 📤 (nebo **Rozbalit** v náhledu). Obsah jde do
  nové složky pojmenované podle archivu. Archiv s nebezpečnými cestami (`../`) se odmítne
  a nerozbalí se nic.
- Časy souborů v ZIPu se ukládají v UTC, takže je Průzkumník Windows může ukázat posunuté
  o hodinu. Po rozbalení v aplikaci jsou časy správně.

## Náhled souboru

Klik na soubor otevře náhled s informacemi (název, cesta, typ, velikost, čas změny):

- **obrázky** (do 32 MB), **audio a video** (do 200 MB) přímo v aplikaci,
- **text** (do 1 MB),
- **Otevřít v systému** předá soubor systémovému programu (na Androidu systémovému výběru),
- odtud jde také přejmenovat, smazat nebo rozbalit ZIP.

## Co si aplikace pamatuje

Po zavření a znovuotevření se obnoví otevřené karty, řazení, seznam/mřížka, skryté soubory,
zapnuté dva panely a připnuté složky. Pokud složka mezitím zmizela, otevře se domovská.
Stav je uložený v prohlížečovém úložišti aplikace (viz [INSTALACE.md](INSTALACE.md),
sekce Řešení problémů, jak ho vynulovat).

## Android: oprávnění

Android 11+ chrání úložiště. Aby aplikace viděla soubory, povol:
**Nastavení → Aplikace → File manager in RUST → Oprávnění → Přístup ke všem souborům.**
Dokud chybí, aplikace ukáže žlutý banner s tímto návodem.

Pokud Android při spuštění ukáže varování o „16KB režimu“, máš starší APK – sestav ho
a nainstaluj znovu (viz [INSTALACE.md](INSTALACE.md)), nová verze je zarovnaná správně.
