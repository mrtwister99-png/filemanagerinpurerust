// Vanilla JS frontend pro Tauri - zadny React, zadny build.
// Vsechno filesystemove jde pres Rust commandy ve filecore.
// Cele je to v IIFE: Tauri vklada do stranky vlastni globalni promenne (napr. isTauri)
// a top-level const se stejnym jmenem by shodil cely skript.
(() => {
"use strict";

const $ = (s, root = document) => root.querySelector(s);

const IMAGE_MAX = 32 * 1024 * 1024;
const MEDIA_MAX = 200 * 1024 * 1024;
const TEXT_MAX = 1024 * 1024;
const STORE_KEY = "fm.state.v1";
const WIDE_MIN = 900; // od teto sirky: stale viditelny panel mist a moznost dvou panelu
const SEARCH_MAX = 500;
const THUMB_SIZE = 256;
const THUMB_PARALLEL = 4;
const THUMB_CACHE_MAX = 300;

const TEXT_EXT = new Set([
  "txt", "md", "json", "toml", "yaml", "yml", "xml", "csv", "log", "ini", "cfg", "conf",
  "rs", "js", "mjs", "ts", "tsx", "jsx", "css", "html", "py", "sh", "bat", "ps1",
  "c", "h", "cpp", "hpp", "java", "kt", "sql", "gitignore", "lock", "env",
]);
const THUMB_EXT = new Set(["jpg", "jpeg", "png", "gif", "webp", "bmp"]);

const MIME = {
  jpg: "image/jpeg", jpeg: "image/jpeg", png: "image/png", webp: "image/webp",
  gif: "image/gif", bmp: "image/bmp", svg: "image/svg+xml", heic: "image/heic",
  mp4: "video/mp4", webm: "video/webm", mkv: "video/x-matroska", mov: "video/quicktime",
  mp3: "audio/mpeg", m4a: "audio/mp4", flac: "audio/flac", wav: "audio/wav",
  ogg: "audio/ogg", opus: "audio/opus",
};

/* ---------- stav ----------
   app = globalni nastaveni; panes = 1-2 panely, kazdy ma vlastni karty, razeni, vyber.
   Karta (tab) = { path, history }. */

const app = {
  home: ".",
  roots: [],
  places: null,
  trash: false,
  panes: [],
  active: 0,
  dual: false,
  view: "list", // "list" | "grid"
  showHidden: false,
  pins: [],
  clip: null, // { mode: "copy" | "move", paths: [] }
};

const wideMq = window.matchMedia(`(min-width: ${WIDE_MIN}px)`);
const dualOn = () => app.dual && wideMq.matches;
const activePane = () => app.panes[dualOn() ? app.active : 0];
const curTab = (p) => p.tabs[p.tab];
const otherPane = (p) => app.panes[1 - p.idx];

const inTauri = () => !!(window.__TAURI__ && window.__TAURI__.core);

async function invoke(cmd, args = {}) {
  if (!inTauri()) {
    // Fallback pro otevreni index.html v prohlizeci - jen ukazkova data.
    if (cmd === "list_roots_cmd") return ["/"];
    if (cmd === "storage_state_cmd") return "desktop";
    if (cmd === "trash_available_cmd") return false;
    if (cmd === "list_places_cmd") return [{ label: "Domů", path: "/", kind: "home", space: null }];
    if (cmd === "search_cmd") return { hits: [], truncated: false, cancelled: false };
    if (cmd === "list_dir_cmd") return [
      { name: "Download", path: "/Download", is_dir: true, kind: "dir", size: 0, size_human: "-", modified_unix: 0, extension: "", is_hidden: false },
      { name: "ukazka.txt", path: "/ukazka.txt", is_dir: false, kind: "document", size: 22, size_human: "22 B", modified_unix: 0, extension: "txt", is_hidden: false },
    ];
    return [];
  }
  return await window.__TAURI__.core.invoke(cmd, args);
}

/* ---------- pomocne ---------- */

function esc(s) {
  return String(s ?? "").replace(/[&<>"']/g, (c) => (
    { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]
  ));
}

function fmtDate(unix) {
  if (!unix) return "";
  try {
    return new Date(unix * 1000).toLocaleString("cs-CZ", {
      day: "2-digit", month: "2-digit", year: "numeric",
      hour: "2-digit", minute: "2-digit",
    });
  } catch { return ""; }
}

/** Cesky plural: 1 soubor, 2-4 soubory, 5+ souboru */
function plural(n, one, few, many) {
  return `${n} ${n === 1 ? one : (n >= 2 && n <= 4 ? few : many)}`;
}

function fmtSize(bytes) {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let s = bytes, u = 0;
  while (s >= 1024 && u < units.length - 1) { s /= 1024; u++; }
  return u === 0 ? `${bytes} B` : `${s.toFixed(1)} ${units[u]}`;
}

function iconFor(e) {
  if (e.is_dir) return "📁";
  switch (e.kind) {
    case "image": return "🖼️";
    case "video": return "🎬";
    case "audio": return "🎵";
    case "document": return "📄";
    case "archive": return "📦";
    case "code": return "💻";
    default: return "📄";
  }
}

function isPermissionError(err) {
  const s = String(err).toLowerCase();
  return s.includes("permission denied") || s.includes("access is denied")
    || s.includes("os error 5") || s.includes("os error 13") || s.includes("odmítnut");
}

const normPath = (p) => String(p || "").replace(/[\\/]+$/, "").toLowerCase();
const samePath = (a, b) => normPath(a) === normPath(b);

function parentOf(p) {
  const clean = String(p || "").replace(/[\\/]+$/, "");
  const idx = Math.max(clean.lastIndexOf("/"), clean.lastIndexOf("\\"));
  if (idx < 0) return app.home || p;
  let parent = clean.slice(0, idx);
  if (/^[A-Za-z]:$/.test(parent)) parent += "\\";
  return parent || "/";
}

function baseName(p) {
  const clean = String(p || "").replace(/[\\/]+$/, "");
  if (!clean) return "/";
  if (/^[A-Za-z]:$/.test(clean)) return clean.toUpperCase();
  const idx = Math.max(clean.lastIndexOf("/"), clean.lastIndexOf("\\"));
  return clean.slice(idx + 1) || clean;
}

/** Slozka zasahu relativne ke slozce, kde se hledalo ("" = primo v ni). */
function relDir(base, full) {
  const b = String(base || "").replace(/[\\/]+$/, "");
  const parent = parentOf(full).replace(/[\\/]+$/, "");
  if (parent.toLowerCase() === b.toLowerCase()) return "";
  if (parent.toLowerCase().startsWith(b.toLowerCase())) {
    return parent.slice(b.length).replace(/^[\\/]+/, "");
  }
  return parent;
}

function crumbsOf(path) {
  const out = [];
  const re = /[^\\/]+/g;
  let m;
  while ((m = re.exec(path))) {
    let prefix = path.slice(0, m.index + m[0].length);
    if (/^[A-Za-z]:$/.test(prefix)) prefix += "\\";
    if (path.startsWith("/") && !prefix.startsWith("/")) prefix = "/" + prefix;
    out.push({ label: m[0], path: prefix });
  }
  return out;
}

let toastTimer = null;
function toast(msg, ms = 2500) {
  const el = $("#toast");
  el.textContent = msg;
  el.hidden = false;
  clearTimeout(toastTimer);
  if (ms > 0) toastTimer = setTimeout(() => { el.hidden = true; }, ms);
}
function toastHide() { clearTimeout(toastTimer); $("#toast").hidden = true; }

/* ---------- ulozeny stav (localStorage) ---------- */

let saveTimer = null;
function snapshot() {
  return {
    v: 1,
    panes: app.panes.map((p) => ({
      tabs: p.tabs.map((t) => ({ path: t.path, history: t.history.slice(-30) })),
      tab: p.tab,
      sort: p.sort,
    })),
    active: app.active,
    dual: app.dual,
    view: app.view,
    showHidden: app.showHidden,
    pins: app.pins,
  };
}
function saveNow() {
  try { localStorage.setItem(STORE_KEY, JSON.stringify(snapshot())); } catch { }
}
function save() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(saveNow, 200);
}
function loadSaved() {
  try {
    const s = JSON.parse(localStorage.getItem(STORE_KEY) || "null");
    return s && s.v === 1 ? s : null;
  } catch { return null; }
}

/* ---------- panely ---------- */

function makePane(idx, saved) {
  const el = $("#paneTpl").content.firstElementChild.cloneNode(true);
  const p = {
    idx,
    el,
    q: (s) => el.querySelector(s),
    tabs: [],
    tab: 0,
    sort: "name",
    entries: [],
    shown: [],
    sel: new Set(),
    loaded: false,
    loadToken: 0,
    query: "",
    searching: false,
    searchInfo: null,
    searchToken: 0,
    searchTimer: null,
    sizesMode: false,
    sizes: null,
    sizesErr: "",
    sizesToken: 0,
  };
  if (saved && Array.isArray(saved.tabs) && saved.tabs.length) {
    p.tabs = saved.tabs
      .filter((t) => t && typeof t.path === "string" && t.path)
      .map((t) => ({ path: t.path, history: Array.isArray(t.history) ? t.history.filter((h) => typeof h === "string") : [] }));
    p.tab = Math.min(Math.max(0, saved.tab | 0), Math.max(0, p.tabs.length - 1));
    if (["name", "size", "date", "kind"].includes(saved.sort)) p.sort = saved.sort;
  }
  if (!p.tabs.length) p.tabs = [{ path: app.home, history: [] }];
  p.q(".sort").value = p.sort;
  wirePane(p);
  return p;
}

async function listPath(path) {
  return await invoke("list_dir_cmd", { path, showHidden: app.showHidden });
}

function cancelSearch(p) {
  clearTimeout(p.searchTimer);
  p.searchToken++;
  p.searching = false;
  if (p.query) invoke("cancel_job_cmd", { job: `search-${p.idx}` }).catch(() => { });
  p.query = "";
  p.searchInfo = null;
  p.q(".search").value = "";
}

/** Otevre slozku v aktivni karte panelu. silent = pri chybe potichu na domovskou slozku. */
async function openPath(p, path, opts = {}) {
  if (!path) path = app.home || ".";
  const tab = curTab(p);
  const token = ++p.loadToken;
  try {
    const items = await listPath(path);
    if (token !== p.loadToken) return;
    if (opts.push !== false && tab.path && !samePath(tab.path, path)) {
      tab.history.push(tab.path);
      if (tab.history.length > 50) tab.history.splice(0, tab.history.length - 50);
    }
    tab.path = path;
    p.entries = items;
    cancelSearch(p);
    p.sel.clear();
    hidePermBanner();
  } catch (err) {
    if (token !== p.loadToken) return;
    if (opts.silent && !samePath(path, app.home) && !isPermissionError(err)) {
      return openPath(p, app.home, { push: false });
    }
    tab.path = path;
    p.entries = [];
    p.sel.clear();
    cancelSearch(p);
    if (isPermissionError(err)) showPermBanner();
    else notify("Složku nelze otevřít", String(err));
  }
  p.loaded = true;
  if (p.sizesMode) loadSizes(p);
  render(p);
  renderSidebarHere();
  save();
}

async function refresh(p) {
  if (!p.loaded) return;
  if (p.query) { runSearch(p); return; }
  const path = curTab(p).path || app.home;
  const token = ++p.loadToken;
  try {
    const items = await listPath(path);
    if (token !== p.loadToken) return;
    p.entries = items;
    hidePermBanner();
  } catch (err) {
    if (token !== p.loadToken) return;
    p.entries = [];
    if (isPermissionError(err)) showPermBanner();
  }
  // Vyber, ktery uz neexistuje, zahodime
  const alive = new Set(p.entries.map((e) => e.path));
  for (const s of [...p.sel]) if (!alive.has(s)) p.sel.delete(s);
  if (p.sizesMode) loadSizes(p);
  render(p);
}

function refreshAll() {
  for (const p of visiblePanes()) refresh(p);
}

function visiblePanes() {
  return dualOn() ? app.panes : [app.panes[0]];
}

function setActive(idx) {
  if (!dualOn() || app.active === idx) return;
  app.active = idx;
  for (const p of app.panes) p.el.classList.toggle("active", p.idx === app.active);
  renderSidebarHere();
  renderMenu();
  save();
}

/* ---------- karty ---------- */

function switchTab(p, i) {
  if (i === p.tab || !p.tabs[i]) return;
  p.tab = i;
  p.sel.clear();
  openPath(p, curTab(p).path, { push: false, silent: true });
}

function newTab(p) {
  p.tabs.push({ path: curTab(p).path || app.home, history: [] });
  p.tab = p.tabs.length - 1;
  p.sel.clear();
  openPath(p, curTab(p).path, { push: false, silent: true });
}

function closeTab(p, i) {
  if (p.tabs.length <= 1) return;
  const wasActive = i === p.tab;
  p.tabs.splice(i, 1);
  if (p.tab > i || p.tab >= p.tabs.length) p.tab = Math.max(0, p.tab - 1);
  if (wasActive) openPath(p, curTab(p).path, { push: false, silent: true });
  else { renderTabs(p); save(); }
}

function renderTabs(p) {
  p.q(".tabList").innerHTML = p.tabs.map((t, i) => `
    <div class="tab${i === p.tab ? " on" : ""}" data-t="${i}" title="${esc(t.path)}">
      <span>${esc(baseName(t.path))}</span>
      ${p.tabs.length > 1 ? `<button class="tabX" data-x="${i}" title="Zavřít kartu">×</button>` : ""}
    </div>`).join("");
}

/* ---------- vykresleni ---------- */

function sortedEntries(p) {
  const sort = p.sort;
  const items = [...p.entries];
  const byName = (a, b) => a.name.localeCompare(b.name, "cs");
  items.sort((a, b) => {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
    if (sort === "size") return b.size - a.size;
    if (sort === "date") return b.modified_unix - a.modified_unix;
    if (sort === "kind") return (a.kind || "").localeCompare(b.kind || "") || byName(a, b);
    return byName(a, b);
  });
  return items;
}

function isZip(e) { return !e.is_dir && (e.extension || "").toLowerCase() === "zip"; }
function isPinned(path) { return app.pins.some((x) => samePath(x, path)); }

function actionsHtml(e, i) {
  let h = "";
  if (e.is_dir) {
    const pinned = isPinned(e.path);
    h += `<button data-act="pin" data-i="${i}" class="${pinned ? "on" : ""}" title="${pinned ? "Odepnout z postranního panelu" : "Připnout do postranního panelu"}">📌</button>`;
  }
  if (isZip(e)) h += `<button data-act="unzip" data-i="${i}" title="Rozbalit">📤</button>`;
  h += `<button data-act="rename" data-i="${i}" title="Přejmenovat">✎</button>`;
  h += `<button data-act="delete" data-i="${i}" title="${app.trash ? "Do koše (Shift = nevratně)" : "Smazat"}">🗑</button>`;
  return h;
}

function subText(p, e) {
  let s = e.is_dir ? "složka" : e.size_human;
  if (p.query) {
    const rd = relDir(curTab(p).path, e.path);
    s += " • " + (rd ? `📂 ${rd}` : "v této složce");
  }
  return s;
}

function thumbKey(e) { return `${e.path}|${e.modified_unix}|${e.size}`; }

function render(p) {
  renderTabs(p);
  renderCrumbs(p);
  renderSelection(p);
  renderClipboard(p);
  p.q(".btnView").textContent = app.view === "grid" ? "☰" : "▦";
  p.q(".btnView").title = app.view === "grid" ? "Zobrazit jako seznam" : "Zobrazit jako mřížku s náhledy";
  p.q(".sizesHead").hidden = !p.sizesMode;
  updateFilterBtn(p);
  if (p.sizesMode) { renderSizes(p); return; }

  const items = sortedEntries(p);
  p.shown = items;
  const listEl = p.q(".list");
  const grid = app.view === "grid";
  listEl.classList.toggle("grid", grid);
  const path = curTab(p).path;

  const emptyEl = p.q(".empty");
  emptyEl.hidden = items.length !== 0 || p.searching;
  const qText = p.filterOnly ? "podle filtrů" : `pro „${p.query}“`;
  emptyEl.textContent = p.query ? `Nic nenalezeno ${qText}` : "Prázdná složka";
  const st = p.q(".status");
  if (p.searching) st.textContent = `Hledám ${p.filterOnly ? "podle filtrů" : `„${p.query}“`} v ${path}…`;
  else if (p.query) {
    const more = p.searchInfo && p.searchInfo.truncated ? ` (zobrazeno prvních ${SEARCH_MAX}, upřesni hledání)` : "";
    st.textContent = `${items.length} výsledků ${qText}${more} • ${path}`;
  } else st.textContent = `${plural(items.length, "položka", "položky", "položek")} • ${path}`;

  let html = "";
  for (let i = 0; i < items.length; i++) {
    const e = items[i];
    const picked = p.sel.has(e.path);
    const cb = `<label class="pick"><input type="checkbox" data-i="${i}" ${picked ? "checked" : ""} /></label>`;
    if (grid) {
      let thumb = `<div class="thumb">${iconFor(e)}</div>`;
      if (e.kind === "image" && THUMB_EXT.has((e.extension || "").toLowerCase())) {
        const key = thumbKey(e);
        const url = thumbGet(key);
        thumb = url
          ? `<div class="thumb"><img src="${url}" alt="" /></div>`
          : `<div class="thumb" data-thumb="${i}">${iconFor(e)}</div>`;
      }
      html += `<div class="item tile${picked ? " is-sel" : ""}" data-i="${i}">
        ${cb}${thumb}
        <div class="tname" title="${esc(e.name)}">${esc(e.name)}</div>
        <small title="${esc(subText(p, e))}">${esc(subText(p, e))}</small>
        <div class="rowActions">${actionsHtml(e, i)}</div>
      </div>`;
    } else {
      html += `<div class="item row${picked ? " is-sel" : ""}" data-i="${i}">
        ${cb}
        <div class="icon">${iconFor(e)}</div>
        <div class="name">
          <b title="${esc(e.name)}">${esc(e.name)}</b>
          <small title="${esc(e.path)}">${esc(subText(p, e))}</small>
        </div>
        <div class="meta">${esc((e.extension || "").toUpperCase())}<br>${esc(fmtDate(e.modified_unix))}</div>
        <div class="rowActions">${actionsHtml(e, i)}</div>
      </div>`;
    }
  }
  listEl.innerHTML = html;
  if (grid) {
    listEl.querySelectorAll(".thumb[data-thumb]").forEach((t) => {
      const e = items[Number(t.dataset.thumb)];
      t._entry = e;
      thumbObserver.observe(t);
    });
  }
}

function renderCrumbs(p) {
  const el = p.q(".crumbs");
  const parts = crumbsOf(curTab(p).path);
  el.innerHTML = `<b data-home="1">🏠 Domů</b>`
    + parts.map((c, i) => ` / <b data-c="${i}" data-path="${esc(c.path)}">${esc(c.label)}</b>`).join("");
}

function renderSelection(p) {
  const n = p.sel.size;
  p.q(".selbar").hidden = n === 0 || p.sizesMode;
  p.q(".selCount").textContent = `${n} vybráno`;
  p.q('[data-sel="delete"]').textContent = app.trash ? "Do koše" : "Smazat";
}

function renderClipboard(p) {
  const btn = p.q(".btnPaste");
  const n = app.clip ? app.clip.paths.length : 0;
  btn.hidden = n === 0 || p.sizesMode;
  if (n > 0) btn.textContent = app.clip.mode === "move" ? `Vložit ${n} (přesun)` : `Vložit ${n} (kopie)`;
}

function renderAll() {
  for (const p of visiblePanes()) render(p);
}

/* ---------- nahledy obrazku v mrizce ---------- */

const thumbCache = new Map(); // klic -> blob URL, nebo null kdyz nahled nejde udelat
const thumbQueue = [];
let thumbRunning = 0;

function thumbGet(key) {
  if (!thumbCache.has(key)) return undefined;
  const url = thumbCache.get(key);
  // LRU: posunout na konec
  thumbCache.delete(key);
  thumbCache.set(key, url);
  return url;
}

function thumbPut(key, url) {
  thumbCache.set(key, url);
  while (thumbCache.size > THUMB_CACHE_MAX) {
    const [oldKey, oldUrl] = thumbCache.entries().next().value;
    thumbCache.delete(oldKey);
    if (oldUrl) URL.revokeObjectURL(oldUrl);
  }
}

const thumbObserver = new IntersectionObserver((list) => {
  for (const en of list) {
    if (!en.isIntersecting) continue;
    thumbObserver.unobserve(en.target);
    thumbQueue.push(en.target);
  }
  pumpThumbs();
}, { rootMargin: "300px" });

function pumpThumbs() {
  while (thumbRunning < THUMB_PARALLEL && thumbQueue.length) {
    const holder = thumbQueue.shift();
    if (!holder.isConnected || !holder._entry) continue;
    const e = holder._entry;
    const key = thumbKey(e);
    const cached = thumbGet(key);
    if (cached !== undefined) { showThumb(holder, cached); continue; }
    thumbRunning++;
    invoke("thumbnail_cmd", { path: e.path, max: THUMB_SIZE })
      .then((buf) => {
        const url = buf && buf.byteLength ? URL.createObjectURL(new Blob([buf], { type: "image/jpeg" })) : null;
        thumbPut(key, url);
        showThumb(holder, url);
      })
      .catch(() => thumbPut(key, null))
      .finally(() => { thumbRunning--; pumpThumbs(); });
  }
}

function showThumb(holder, url) {
  if (!url || !holder.isConnected) return;
  holder.innerHTML = `<img src="${url}" alt="" />`;
}

/* ---------- analyza mista ---------- */

function startSizes(p) {
  cancelSearch(p);
  p.sizesMode = true;
  p.sel.clear();
  loadSizes(p);
  render(p);
}

function stopSizes(p) {
  p.sizesMode = false;
  p.sizesToken++;
  invoke("cancel_job_cmd", { job: `sizes-${p.idx}` }).catch(() => { });
  refresh(p);
}

async function loadSizes(p) {
  const token = ++p.sizesToken;
  p.sizes = null;
  p.sizesErr = "";
  try {
    const list = await invoke("dir_sizes_cmd", { path: curTab(p).path, job: String(p.idx) });
    if (token !== p.sizesToken) return;
    p.sizes = list;
  } catch (err) {
    if (token !== p.sizesToken) return;
    p.sizes = [];
    p.sizesErr = String(err);
  }
  if (p.sizesMode) render(p);
}

function renderSizes(p) {
  const listEl = p.q(".list");
  listEl.classList.remove("grid");
  p.q(".empty").hidden = true;
  const path = curTab(p).path;
  if (p.sizes === null) {
    p.shown = [];
    p.q(".status").textContent = `Počítám velikosti v ${path}…`;
    listEl.innerHTML = `<div class="busy"><div class="spin"></div>Počítám…</div>`;
    return;
  }
  const items = p.sizes;
  p.shown = items;
  const total = items.reduce((a, e) => a + e.size, 0);
  const max = items.length ? Math.max(1, items[0].size) : 1;
  p.q(".status").textContent = p.sizesErr
    ? `Chyba: ${p.sizesErr}`
    : `Celkem ${fmtSize(total)} v ${items.length} položkách • ${path}`;
  if (!items.length && !p.sizesErr) {
    p.q(".empty").hidden = false;
    p.q(".empty").textContent = "Prázdná složka";
  }
  listEl.innerHTML = items.map((e, i) => {
    const pct = total ? (e.size / total) * 100 : 0;
    const w = (e.size / max) * 100;
    const sub = e.is_dir ? `${plural(e.files, "soubor", "soubory", "souborů")} • ${pct.toFixed(1)} %` : `${pct.toFixed(1)} %`;
    return `<div class="item szRow${e.is_dir ? " dir" : ""}" data-i="${i}" title="${esc(e.path)}">
      <div class="icon">${e.is_dir ? "📁" : "📄"}</div>
      <div class="szName"><b>${esc(e.name)}</b><div class="bar"><i style="width:${w.toFixed(2)}%"></i></div><small>${esc(sub)}</small></div>
      <div class="szSize">${esc(e.size_human)}</div>
    </div>`;
  }).join("");
}

/* ---------- hledani ---------- */

function searchOpts(p) {
  const query = p.q(".search").value.trim();
  const kind = p.q(".fKind").value;
  const size = p.q(".fSize").value;
  const date = p.q(".fDate").value;
  const MB = 1024 * 1024;
  const opts = {
    query,
    kinds: kind ? [kind] : [],
    min_size: null,
    max_size: null,
    modified_after: null,
    content: p.q(".fContent").checked,
    max_depth: Number(p.q(".fDepth").value) || 12,
    max_results: SEARCH_MAX,
    show_hidden: app.showHidden,
  };
  if (size === "lt1m") opts.max_size = MB;
  if (size === "1m-100m") { opts.min_size = MB; opts.max_size = 100 * MB; }
  if (size === "gt100m") opts.min_size = 100 * MB;
  if (size === "gt1g") opts.min_size = 1024 * MB;
  const now = Math.floor(Date.now() / 1000);
  if (date === "today") {
    const d = new Date(); d.setHours(0, 0, 0, 0);
    opts.modified_after = Math.floor(d.getTime() / 1000);
  } else if (date) opts.modified_after = now - Number(date) * 86400;
  const filtered = !!(kind || size || date);
  return { opts, active: !!query || filtered, filtered };
}

function updateFilterBtn(p) {
  const { filtered } = searchOpts(p);
  const on = filtered || p.q(".fContent").checked;
  p.q(".btnFilters").classList.toggle("on", on || !p.q(".filters").hidden);
  p.q(".btnFilters").textContent = on ? "⚙ Filtry •" : "⚙ Filtry";
}

async function runSearch(p) {
  clearTimeout(p.searchTimer);
  const { opts, active, filtered } = searchOpts(p);
  const token = ++p.searchToken;
  updateFilterBtn(p);
  if (!active) {
    if (p.query) {
      invoke("cancel_job_cmd", { job: `search-${p.idx}` }).catch(() => { });
      p.query = "";
      p.searchInfo = null;
      p.searching = false;
      await refresh(p);
    }
    return;
  }
  if (p.sizesMode) { p.sizesMode = false; p.sizesToken++; }
  p.query = opts.query || (filtered ? "filtry" : "");
  p.filterOnly = !opts.query;
  p.searching = true;
  render(p);
  try {
    const res = await invoke("search_cmd", { dir: curTab(p).path || ".", opts, job: String(p.idx) });
    if (token !== p.searchToken) return; // mezitim prisel novejsi dotaz
    if (res.cancelled) return;
    p.entries = res.hits;
    p.searchInfo = res;
    p.searching = false;
    p.sel.clear();
    render(p);
  } catch (err) {
    if (token !== p.searchToken) return;
    p.searching = false;
    render(p);
    await notify("Hledat nelze", String(err));
  }
}

/* ---------- operace ---------- */

function entriesFor(p, paths) {
  return paths.map((path) => p.entries.find((e) => e.path === path) || { name: baseName(path), path });
}

async function createFolder() {
  const p = activePane();
  const name = await askText("Nová složka", "Název nové složky", "Nová složka", "Vytvořit");
  if (!name) return;
  try {
    await invoke("create_dir_cmd", { parent: curTab(p).path || ".", name });
    refreshAll();
  } catch (err) { await notify("Složku nelze vytvořit", String(err)); }
}

async function renameEntry(entry) {
  const name = await askText("Přejmenovat", `Nový název pro „${entry.name}“`, entry.name, "Uložit");
  if (!name || name === entry.name) return;
  try {
    await invoke("rename_cmd", { from: entry.path, newName: name });
    closePreview();
    refreshAll();
  } catch (err) { await notify("Přejmenovat nelze", String(err)); }
}

async function removeEntries(p, list, preferPermanent = false) {
  if (!list.length) return;
  const what = list.length === 1 ? `„${list[0].name}“` : plural(list.length, "položka", "položky", "položek");
  let permanent = true;
  if (app.trash) {
    const textFor = (perm) => perm
      ? `${what} bude smazáno nevratně (bez koše). Pokračovat?`
      : `${what} se přesune do koše. Pokračovat?`;
    const res = await askConfirm("Smazat", textFor(preferPermanent), "Do koše", {
      check: "Smazat nevratně (bez koše)",
      checked: preferPermanent,
      onCheck: (on) => {
        $("#dlgText").textContent = textFor(on);
        $("#dlgOk").textContent = on ? "Smazat nevratně" : "Do koše";
      },
    });
    if (!res) return;
    permanent = res.checked;
  } else {
    const ok = await askConfirm("Smazat", `${what} bude smazáno nevratně. Pokračovat?`, "Smazat");
    if (!ok) return;
  }
  toast(permanent ? "Mažu…" : "Přesouvám do koše…", 0);
  try {
    for (const e of list) await invoke("delete_cmd", { path: e.path, permanent });
    toast(permanent ? "Smazáno" : "Přesunuto do koše");
  } catch (err) {
    toastHide();
    await notify("Smazat nelze", String(err));
  }
  for (const e of list) p.sel.delete(e.path);
  closePreview();
  refreshAll();
}

function clipSelection(p, mode) {
  if (!p.sel.size) return;
  app.clip = { mode, paths: [...p.sel] };
  p.sel.clear();
  renderAll();
}

/* Kopirovani / presun s ukazatelem postupu a tlacitkem Zrusit.
   Okno se ukaze az po chvilce, aby u rychlych operaci jen neproblikavalo. */
let transferSeq = 0;
let transferJob = null;

function showProgress(pr) {
  const pct = pr.total_bytes > 0
    ? Math.min(100, (pr.done_bytes / pr.total_bytes) * 100)
    : (pr.total_files > 0 ? (pr.done_files / pr.total_files) * 100 : 0);
  $("#pgFill").style.width = pct.toFixed(1) + "%";
  $("#pgFile").textContent = pr.current || "Připravuji…";
  $("#pgStat").textContent = `${Math.floor(pct)} % • ${fmtSize(pr.done_bytes)} z ${fmtSize(pr.total_bytes)} • `
    + `${pr.done_files} z ${plural(pr.total_files, "souboru", "souborů", "souborů")}`;
}

if (inTauri() && window.__TAURI__.event) {
  window.__TAURI__.event.listen("transfer-progress", (ev) => {
    if (ev.payload && ev.payload.job === transferJob) showProgress(ev.payload);
  });
}

function cancelTransfer() {
  if (!transferJob) return;
  $("#pgCancel").disabled = true;
  $("#pgCancel").textContent = "Ruším…";
  invoke("cancel_job_cmd", { job: `transfer-${transferJob}` }).catch(() => { });
}

async function transfer(paths, dest, mode) {
  if (transferJob) {
    await notify("Chvilku strpení", "Jedno kopírování nebo přesun ještě běží.");
    return false;
  }
  const copy = mode === "copy";
  const job = String(++transferSeq);
  transferJob = job;
  $("#pgTitle").textContent = copy ? "Kopíruji…" : "Přesouvám…";
  $("#pgCancel").disabled = false;
  $("#pgCancel").textContent = "Zrušit";
  showProgress({ done_bytes: 0, total_bytes: 0, done_files: 0, total_files: 0, current: "" });
  const timer = setTimeout(() => { $("#progress").hidden = false; }, 350);
  try {
    const res = await invoke("transfer_cmd", { paths, dest, mode, job });
    if (res && res.cancelled) {
      toast(`Zrušeno – hotovo ${res.done_items} z ${plural(res.total_items, "položky", "položek", "položek")}`);
      return false;
    }
    toast(copy ? "Zkopírováno" : "Přesunuto");
    return true;
  } catch (err) {
    $("#progress").hidden = true;
    await notify(copy ? "Kopírovat nelze" : "Přesunout nelze", String(err));
    return false;
  } finally {
    clearTimeout(timer);
    $("#progress").hidden = true;
    transferJob = null;
  }
}

async function paste(p) {
  if (!app.clip || !app.clip.paths.length) return;
  const { mode, paths } = app.clip;
  const ok = await transfer(paths, curTab(p).path, mode);
  if (ok && mode === "move") app.clip = null;
  refreshAll();
  renderAll();
}

async function toOtherPane(p, mode) {
  if (!dualOn() || !p.sel.size) return;
  const dest = curTab(otherPane(p)).path;
  const paths = [...p.sel];
  p.sel.clear();
  await transfer(paths, dest, mode);
  refreshAll();
}

async function zipSelection(p) {
  if (!p.sel.size) return;
  const paths = [...p.sel];
  const def = paths.length === 1 ? baseName(paths[0]).replace(/\.[^.]+$/, "") || baseName(paths[0]) : "archiv";
  const name = await askText("Zabalit do ZIP", `Název archivu (${plural(paths.length, "položka", "položky", "položek")}, vznikne v této složce)`, def, "Zabalit");
  if (!name) return;
  toast("Balím do ZIP…", 0);
  try {
    const out = await invoke("zip_cmd", { paths, dest: curTab(p).path, name });
    toast(`Vytvořeno „${baseName(out)}“`);
    p.sel.clear();
  } catch (err) {
    toastHide();
    await notify("Zabalit nelze", String(err));
  }
  refreshAll();
}

async function unzipEntry(entry) {
  toast("Rozbaluji…", 0);
  try {
    const out = await invoke("unzip_cmd", { path: entry.path });
    toast(`Rozbaleno do „${baseName(out)}“`);
    closePreview();
  } catch (err) {
    toastHide();
    await notify("Rozbalit nelze", String(err));
  }
  refreshAll();
}

/* ---------- postranni panel ---------- */

function togglePin(path) {
  if (!path) return;
  if (isPinned(path)) app.pins = app.pins.filter((x) => !samePath(x, path));
  else app.pins.push(path);
  renderPins();
  renderMenu();
  renderAll();
  save();
}

const PLACE_ICON = { home: "🏠", folder: "📁", drive: "💽", storage: "📱", sd: "💾" };

function spaceHtml(sp) {
  if (!sp || !sp.total) return "";
  const used = Math.min(100, Math.max(0, ((sp.total - sp.free) / sp.total) * 100));
  return `<div class="space"><div class="spaceBar"><i class="${used > 90 ? "full" : ""}" style="width:${used.toFixed(1)}%"></i></div>
    <small>volné ${esc(sp.free_human)} z ${esc(sp.total_human)}</small></div>`;
}

function renderPlaces() {
  const el = $("#places");
  const list = app.places || [];
  if (!list.length) { el.innerHTML = `<small class="sbNote">Žádná místa</small>`; return; }
  el.innerHTML = list.map((pl) => `
    <button class="place" data-path="${esc(pl.path)}" title="${esc(pl.path)}">
      <span class="pl">${PLACE_ICON[pl.kind] || "📁"} ${esc(pl.label)}</span>
      ${spaceHtml(pl.space)}
    </button>`).join("");
  renderSidebarHere();
}

function renderPins() {
  const el = $("#pins");
  if (!app.pins.length) {
    el.innerHTML = `<small class="sbNote">Zatím nic. Složku připneš tlačítkem 📌 u složky nebo „📌 +“ pro aktuální složku.</small>`;
    return;
  }
  el.innerHTML = app.pins.map((path) => `
    <div class="pin">
      <button class="place" data-path="${esc(path)}" title="${esc(path)}"><span class="pl">📌 ${esc(baseName(path))}</span></button>
      <button class="unpin" data-unpin="${esc(path)}" title="Odepnout">✕</button>
    </div>`).join("");
  renderSidebarHere();
}

function renderSidebarHere() {
  const p = activePane();
  if (!p) return;
  const here = curTab(p).path;
  document.querySelectorAll("#sidebar .place").forEach((b) => {
    b.classList.toggle("here", samePath(b.dataset.path, here));
  });
}

async function loadPlaces() {
  try {
    app.places = await invoke("list_places_cmd");
  } catch {
    app.places = (app.roots || []).map((r) => ({ label: baseName(r), path: r, kind: "folder", space: null }));
  }
  renderPlaces();
}

function openDrawer() {
  document.body.classList.add("drawerOpen");
  $("#backdrop").hidden = false;
}
function closeDrawer() {
  document.body.classList.remove("drawerOpen");
  $("#backdrop").hidden = true;
}

/* ---------- rozlozeni / nastaveni ---------- */

function applyLayout() {
  const dual = dualOn();
  $("#panes").classList.toggle("dual", dual);
  app.panes[1].el.hidden = !dual;
  for (const p of app.panes) p.el.classList.toggle("active", dual && p.idx === app.active);
  $("#btnDual").classList.toggle("on", app.dual);
  $("#btnDual").textContent = app.dual ? "◫ Jeden panel" : "◫ Dva panely";
  if (wideMq.matches) closeDrawer();
  if (dual && !app.panes[1].loaded) openPath(app.panes[1], curTab(app.panes[1]).path, { push: false, silent: true });
  renderSidebarHere();
  renderMenu();
}

function renderMenu() {
  $("#mView").textContent = app.view === "grid" ? "☰ Zobrazit jako seznam" : "▦ Zobrazit jako mřížku";
  $("#showHidden").checked = app.showHidden;
  const p = activePane();
  if (p) $("#mPin").textContent = isPinned(curTab(p).path) ? "📌 Odepnout tuto složku" : "📌 Připnout tuto složku";
}

function setView(v) {
  app.view = v;
  renderMenu();
  renderAll();
  save();
}

/* ---------- nahled ---------- */

let pvEntry = null;
let pvUrl = null;
let pvPane = null;

function closePreview() {
  if (pvUrl) { URL.revokeObjectURL(pvUrl); pvUrl = null; }
  pvEntry = null;
  $("#pvBody").innerHTML = "";
  $("#preview").hidden = true;
}

function infoHtml(e) {
  const rows = [
    ["Název", e.name],
    ["Cesta", e.path],
    ["Typ", e.is_dir ? "složka" : (e.extension ? e.extension.toUpperCase() : "soubor bez přípony")],
    ["Velikost", e.is_dir ? "—" : e.size_human],
    ["Změněno", fmtDate(e.modified_unix) || "—"],
  ];
  return `<table class="info">${rows
    .map(([k, v]) => `<tr><th>${esc(k)}</th><td>${esc(v)}</td></tr>`)
    .join("")}</table>`;
}

function appendPreview(el) {
  $("#pvBody").appendChild(el);
}

async function openPreview(p, entry) {
  closePreview();
  pvEntry = entry;
  pvPane = p;
  $("#pvName").textContent = entry.name;
  $("#pvBody").innerHTML = infoHtml(entry);
  $("#pvUnzip").hidden = !isZip(entry);
  $("#pvDelete").textContent = app.trash ? "Do koše" : "Smazat";
  $("#preview").hidden = false;

  const ext = (entry.extension || "").toLowerCase();
  const mime = MIME[ext] || "application/octet-stream";

  try {
    if (entry.kind === "image" && entry.size <= IMAGE_MAX) {
      const buf = await invoke("read_bytes_cmd", { path: entry.path, maxBytes: IMAGE_MAX });
      if (pvEntry !== entry) return;
      if (buf && buf.byteLength !== undefined) {
        pvUrl = URL.createObjectURL(new Blob([buf], { type: mime }));
        const img = document.createElement("img");
        img.className = "pvImage";
        img.src = pvUrl;
        img.alt = entry.name;
        appendPreview(img);
      }
    } else if ((entry.kind === "audio" || entry.kind === "video") && entry.size <= MEDIA_MAX) {
      const buf = await invoke("read_bytes_cmd", { path: entry.path, maxBytes: MEDIA_MAX });
      if (pvEntry !== entry) return;
      if (buf && buf.byteLength !== undefined) {
        pvUrl = URL.createObjectURL(new Blob([buf], { type: mime }));
        const media = document.createElement(entry.kind === "video" ? "video" : "audio");
        media.className = "pvMedia";
        media.controls = true;
        media.autoplay = true;
        media.playsInline = true;
        media.src = pvUrl;
        appendPreview(media);
      }
    } else if (TEXT_EXT.has(ext) && entry.size <= TEXT_MAX) {
      const text = await invoke("read_text_cmd", { path: entry.path });
      if (pvEntry !== entry) return;
      const pre = document.createElement("pre");
      pre.className = "pvText";
      pre.textContent = text;
      appendPreview(pre);
    }
  } catch (err) {
    const el = document.createElement("p");
    el.className = "err";
    el.textContent = String(err);
    appendPreview(el);
  }
}

/* ---------- dialogy (prompt/confirm v TauriWebView nefunguji spolehlive) ---------- */

let dlgDone = null;
let dlgOnCheck = null;

function closeDialog(value) {
  $("#dialog").hidden = true;
  const done = dlgDone;
  dlgDone = null;
  dlgOnCheck = null;
  if (done) done(value);
}

function prepDialog(title, text) {
  $("#dlgTitle").textContent = title;
  $("#dlgText").textContent = text || "";
  $("#dlgCheckWrap").hidden = true;
  dlgOnCheck = null;
  $("#dialog").hidden = false;
}

/** Bez opts.check vraci true/false; s opts.check vraci null nebo { checked }. */
function askConfirm(title, text, okLabel = "OK", opts = {}) {
  return new Promise((resolve) => {
    dlgDone = (v) => {
      if (opts.check) resolve(v === true ? { checked: $("#dlgCheck").checked } : null);
      else resolve(v === true);
    };
    prepDialog(title, text);
    $("#dlgInput").hidden = true;
    $("#dlgCancel").hidden = false;
    const ok = $("#dlgOk");
    ok.textContent = okLabel;
    ok.classList.toggle("danger", okLabel === "Smazat" || !!opts.check);
    if (opts.check) {
      $("#dlgCheckWrap").hidden = false;
      $("#dlgCheckLabel").textContent = opts.check;
      $("#dlgCheck").checked = !!opts.checked;
      dlgOnCheck = opts.onCheck || null;
      if (dlgOnCheck) dlgOnCheck(!!opts.checked);
    }
    setTimeout(() => ok.focus(), 20);
  });
}

function askText(title, text, value, okLabel = "OK") {
  return new Promise((resolve) => {
    dlgDone = (v) => resolve(typeof v === "string" ? v : null);
    prepDialog(title, text);
    const input = $("#dlgInput");
    input.hidden = false;
    input.value = value || "";
    $("#dlgCancel").hidden = false;
    const ok = $("#dlgOk");
    ok.textContent = okLabel;
    ok.classList.remove("danger");
    setTimeout(() => { input.focus(); input.select(); }, 20);
  });
}

function notify(title, text) {
  return new Promise((resolve) => {
    dlgDone = () => resolve();
    prepDialog(title, text);
    $("#dlgInput").hidden = true;
    $("#dlgCancel").hidden = true;
    const ok = $("#dlgOk");
    ok.textContent = "OK";
    ok.classList.remove("danger");
    setTimeout(() => ok.focus(), 20);
  });
}

/* ---------- Android oprávnění ---------- */

function showPermBanner() { $("#permBanner").hidden = false; }
function hidePermBanner() { $("#permBanner").hidden = true; }

/* ---------- udalosti panelu ---------- */

function wirePane(p) {
  const el = p.el;
  el.addEventListener("pointerdown", () => setActive(p.idx), true);
  el.addEventListener("focusin", () => setActive(p.idx));

  p.q(".tabList").addEventListener("click", (ev) => {
    const x = ev.target.closest("[data-x]");
    if (x) { ev.stopPropagation(); closeTab(p, Number(x.dataset.x)); return; }
    const t = ev.target.closest("[data-t]");
    if (t) switchTab(p, Number(t.dataset.t));
  });
  p.q(".tabList").addEventListener("auxclick", (ev) => {
    const t = ev.target.closest("[data-t]");
    if (t && ev.button === 1) closeTab(p, Number(t.dataset.t));
  });
  p.q(".tabNew").onclick = () => newTab(p);

  p.q(".crumbs").addEventListener("click", (ev) => {
    const b = ev.target.closest("b");
    if (!b) return;
    openPath(p, b.dataset.home ? app.home : b.dataset.path);
  });

  p.q(".sort").onchange = () => { p.sort = p.q(".sort").value; render(p); save(); };
  p.q(".btnView").onclick = () => setView(app.view === "grid" ? "list" : "grid");
  p.q(".btnPaste").onclick = () => paste(p);
  p.q(".btnFilters").onclick = () => {
    p.q(".filters").hidden = !p.q(".filters").hidden;
    updateFilterBtn(p);
  };

  const search = p.q(".search");
  search.addEventListener("input", () => {
    clearTimeout(p.searchTimer);
    p.searchTimer = setTimeout(() => runSearch(p), 250);
  });
  search.addEventListener("keydown", (ev) => {
    if (ev.key === "Enter") runSearch(p);
    if (ev.key === "Escape" && search.value) { ev.stopPropagation(); search.value = ""; runSearch(p); }
  });
  for (const s of [".fKind", ".fSize", ".fDate", ".fDepth", ".fContent"]) {
    p.q(s).addEventListener("change", () => runSearch(p));
  }

  p.q(".selbar").addEventListener("click", (ev) => {
    const b = ev.target.closest("[data-sel]");
    if (!b) return;
    const act = b.dataset.sel;
    if (act === "copy") clipSelection(p, "copy");
    else if (act === "move") clipSelection(p, "move");
    else if (act === "zip") zipSelection(p);
    else if (act === "copyOther") toOtherPane(p, "copy");
    else if (act === "moveOther") toOtherPane(p, "move");
    else if (act === "delete") removeEntries(p, entriesFor(p, [...p.sel]), ev.shiftKey);
    else if (act === "clear") { p.sel.clear(); render(p); }
  });

  p.q(".sizesHead").addEventListener("click", (ev) => {
    const b = ev.target.closest("[data-sz]");
    if (!b) return;
    if (b.dataset.sz === "up") openPath(p, parentOf(curTab(p).path));
    else stopSizes(p);
  });

  const list = p.q(".list");
  list.addEventListener("click", (ev) => {
    const item = ev.target.closest(".item");
    if (!item || ev.target.closest(".pick")) return;
    const e = p.shown[Number(item.dataset.i)];
    if (!e) return;
    const btn = ev.target.closest("button[data-act]");
    if (btn) {
      ev.stopPropagation();
      const act = btn.dataset.act;
      if (act === "rename") renameEntry(e);
      else if (act === "delete") removeEntries(p, [e], ev.shiftKey);
      else if (act === "pin") togglePin(e.path);
      else if (act === "unzip") unzipEntry(e);
      return;
    }
    if (p.sizesMode) { if (e.is_dir) openPath(p, e.path); return; }
    if (e.is_dir) openPath(p, e.path);
    else openPreview(p, e);
  });
  list.addEventListener("change", (ev) => {
    const cb = ev.target.closest("input[data-i]");
    if (!cb) return;
    const e = p.shown[Number(cb.dataset.i)];
    if (!e) return;
    if (cb.checked) p.sel.add(e.path);
    else p.sel.delete(e.path);
    cb.closest(".item").classList.toggle("is-sel", cb.checked);
    renderSelection(p);
  });
}

/* ---------- globalni udalosti ---------- */

$("#btnBack").onclick = () => {
  const p = activePane();
  const prev = curTab(p).history.pop();
  if (prev !== undefined) openPath(p, prev, { push: false, silent: true });
};
$("#btnHome").onclick = () => {
  const p = activePane();
  curTab(p).history = [];
  openPath(p, app.home || ".", { push: false });
};
$("#btnUp").onclick = () => { const p = activePane(); openPath(p, parentOf(curTab(p).path)); };
$("#btnNewFolder").onclick = createFolder;
$("#btnDual").onclick = () => {
  app.dual = !app.dual;
  if (!app.dual) app.active = 0;
  applyLayout();
  save();
};

$("#btnMore").onclick = (ev) => {
  ev.stopPropagation();
  renderMenu();
  $("#moreMenu").hidden = !$("#moreMenu").hidden;
};
document.addEventListener("click", (ev) => {
  if (!ev.target.closest(".moreWrap")) $("#moreMenu").hidden = true;
});
$("#moreMenu").addEventListener("click", (ev) => {
  const b = ev.target.closest("[data-cmd]");
  if (!b) return;
  $("#moreMenu").hidden = true;
  const p = activePane();
  const cmd = b.dataset.cmd;
  if (cmd === "view") setView(app.view === "grid" ? "list" : "grid");
  else if (cmd === "sizes") startSizes(p);
  else if (cmd === "pin") togglePin(curTab(p).path);
  else if (cmd === "newtab") newTab(p);
});
$("#showHidden").onchange = () => {
  app.showHidden = $("#showHidden").checked;
  save();
  refreshAll();
};

$("#btnDrawer").onclick = openDrawer;
$("#backdrop").onclick = closeDrawer;
$("#btnPinCur").onclick = () => togglePin(curTab(activePane()).path);
$("#sidebar").addEventListener("click", (ev) => {
  const un = ev.target.closest("[data-unpin]");
  if (un) { togglePin(un.dataset.unpin); return; }
  const pl = ev.target.closest(".place[data-path]");
  if (!pl) return;
  openPath(activePane(), pl.dataset.path);
  closeDrawer();
});

$("#pvClose").onclick = closePreview;
$("#pgCancel").onclick = cancelTransfer;
$("#pvOpen").onclick = async () => {
  if (!pvEntry) return;
  try { await invoke("open_with_system_cmd", { path: pvEntry.path }); }
  catch (err) { await notify("Otevřít nelze", String(err)); }
};
$("#pvRename").onclick = () => { if (pvEntry) renameEntry(pvEntry); };
$("#pvDelete").onclick = (ev) => { if (pvEntry) removeEntries(pvPane || activePane(), [pvEntry], ev.shiftKey); };
$("#pvUnzip").onclick = () => { if (pvEntry) unzipEntry(pvEntry); };

$("#dlgForm").addEventListener("submit", (ev) => {
  ev.preventDefault();
  const input = $("#dlgInput");
  closeDialog(input.hidden ? true : input.value.trim());
});
$("#dlgCancel").onclick = () => closeDialog(null);
$("#dlgCheck").onchange = () => { if (dlgOnCheck) dlgOnCheck($("#dlgCheck").checked); };

document.addEventListener("keydown", (ev) => {
  if (ev.key !== "Escape") return;
  if (!$("#dialog").hidden) closeDialog(null);
  else if (!$("#progress").hidden) cancelTransfer();
  else if (!$("#preview").hidden) closePreview();
  else if (!$("#moreMenu").hidden) $("#moreMenu").hidden = true;
  else if (document.body.classList.contains("drawerOpen")) closeDrawer();
});

wideMq.addEventListener("change", () => {
  // Preference dvou panelu zustava, na uzkem okne se jen nezobrazuje
  applyLayout();
  if (dualOn()) renderAll();
});
window.addEventListener("pagehide", saveNow);

/* ---------- start ---------- */

async function init() {
  let roots = [];
  try { roots = await invoke("list_roots_cmd"); } catch { }
  app.roots = roots || [];
  app.home = (roots && roots[0]) || ".";
  try { app.trash = !!(await invoke("trash_available_cmd")); } catch { app.trash = false; }

  const saved = loadSaved();
  if (saved) {
    app.dual = !!saved.dual;
    app.active = saved.active === 1 ? 1 : 0;
    app.view = saved.view === "grid" ? "grid" : "list";
    app.showHidden = !!saved.showHidden;
    app.pins = Array.isArray(saved.pins) ? saved.pins.filter((x) => typeof x === "string") : [];
  }
  const panesEl = $("#panes");
  for (let i = 0; i < 2; i++) {
    const p = makePane(i, saved && saved.panes ? saved.panes[i] : null);
    app.panes.push(p);
    panesEl.appendChild(p.el);
  }

  // Tlacitka disku (C:, D:, ...) - jen na Windows, jinde zadne disky nejsou.
  const drivesEl = $("#drives");
  for (const r of app.roots) {
    if (!/^[A-Za-z]:\\$/.test(r)) continue;
    const btn = document.createElement("button");
    btn.className = "drive";
    btn.textContent = r.slice(0, 2).toUpperCase();
    btn.title = `Přejít na disk ${btn.textContent}`;
    btn.onclick = () => openPath(activePane(), r);
    drivesEl.appendChild(btn);
  }

  renderPins();
  renderMenu();
  applyLayout();
  await openPath(app.panes[0], curTab(app.panes[0]).path, { push: false, silent: true });
  loadPlaces();

  try {
    const state = await invoke("storage_state_cmd");
    if (state === "missing") showPermBanner();
  } catch { }
}

init();
})();
