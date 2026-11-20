use filecore::{FileEntry, FileInfo, Place, SearchOptions, SearchResult, SizeEntry};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Vsechny commandy jsou tenke wrappery nad filecore.
/// UI vola tyhle funkce, logika zustava v cistem Rustu v crates/filecore.
///
/// Pozor: ne-async commandy bezi na hlavnim vlakne a zamrazi okno. Cokoli, co muze
/// trvat dlouho, je proto async a samotnou praci dela ve spawn_blocking.

async fn blocking<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

/// Generace uloh podle klice (napr. "search-0", "sizes-1"). Novejsi uloha se stejnym
/// klicem zvysi generaci a starsi pri dalsi kontrole zjisti, ze ma skoncit.
fn jobs() -> &'static Mutex<HashMap<String, u64>> {
    static JOBS: OnceLock<Mutex<HashMap<String, u64>>> = OnceLock::new();
    JOBS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn job_start(key: &str) -> u64 {
    let mut map = jobs().lock().unwrap();
    let gen = map.entry(key.to_string()).or_insert(0);
    *gen += 1;
    *gen
}

fn job_current(key: &str, gen: u64) -> bool {
    jobs().lock().unwrap().get(key).copied() == Some(gen)
}

#[tauri::command]
pub async fn list_dir_cmd(path: String, show_hidden: Option<bool>) -> Result<Vec<FileEntry>, String> {
    blocking(move || {
        filecore::list_dir_opts(path, show_hidden.unwrap_or(false)).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub fn list_roots_cmd() -> Vec<PathBuf> {
    filecore::list_roots()
}

/// Mista pro postranni panel vcetne volneho mista na discich.
#[tauri::command]
pub async fn list_places_cmd() -> Result<Vec<Place>, String> {
    blocking(|| Ok(filecore::list_places())).await
}

/// Hledani s filtry. `job` je klic panelu - novejsi hledani stejneho panelu to starsi zrusi.
#[tauri::command]
pub async fn search_cmd(dir: String, opts: SearchOptions, job: Option<String>) -> Result<SearchResult, String> {
    let key = format!("search-{}", job.unwrap_or_default());
    let gen = job_start(&key);
    blocking(move || {
        let cancelled = || !job_current(&key, gen);
        Ok(filecore::search_with(dir, &opts, &cancelled))
    })
    .await
}

/// Zrusi probihajici ulohu (hledani / analyzu) daneho klice.
#[tauri::command]
pub fn cancel_job_cmd(job: String) {
    job_start(&job);
}

#[tauri::command]
pub async fn dir_sizes_cmd(path: String, job: Option<String>) -> Result<Vec<SizeEntry>, String> {
    let key = format!("sizes-{}", job.unwrap_or_default());
    let gen = job_start(&key);
    blocking(move || {
        let cancelled = || !job_current(&key, gen);
        let out = filecore::child_sizes(&path, &cancelled).map_err(|e| e.to_string())?;
        if cancelled() {
            return Err("zrušeno".to_string());
        }
        Ok(out)
    })
    .await
}

/// JPEG nahled obrazku (delsi strana max `max` px) jako binarni odpoved.
#[tauri::command]
pub async fn thumbnail_cmd(path: String, max: Option<u32>) -> Result<tauri::ipc::Response, String> {
    blocking(move || {
        filecore::thumbnail_jpeg(&path, max.unwrap_or(256))
            .map(tauri::ipc::Response::new)
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn zip_cmd(paths: Vec<String>, dest: String, name: String) -> Result<String, String> {
    blocking(move || {
        filecore::zip_entries(&paths, std::path::Path::new(&dest), &name)
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn unzip_cmd(path: String) -> Result<String, String> {
    blocking(move || {
        filecore::unzip_archive(&path)
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub fn get_info_cmd(path: String) -> Result<FileInfo, String> {
    filecore::get_info(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_dir_cmd(parent: String, name: String) -> Result<PathBuf, String> {
    filecore::create_dir(parent, &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_cmd(from: String, new_name: String) -> Result<PathBuf, String> {
    filecore::rename_entry(from, &new_name).map_err(|e| e.to_string())
}

/// Je k dispozici systemovy kos? (desktop ano, Android ne)
#[tauri::command]
pub fn trash_available_cmd() -> bool {
    filecore::trash_available()
}

/// Smazani: do kose, nebo nevratne pri `permanent = true` (a vzdy na Androidu).
/// Obsah kose (Windows: systemovy kos, Android: vlastni kos appky).
#[tauri::command]
pub async fn bin_list_cmd() -> Result<Vec<filecore::BinItem>, String> {
    blocking(|| filecore::bin_list().map_err(|e| e.to_string())).await
}

/// Obnovi polozky kose na puvodni misto; vraci pocet obnovenych.
#[tauri::command]
pub async fn bin_restore_cmd(ids: Vec<String>) -> Result<usize, String> {
    blocking(move || filecore::bin_restore(&ids).map_err(|e| e.to_string())).await
}

/// Nenavratne smaze vybrane polozky z kose.
#[tauri::command]
pub async fn bin_purge_cmd(ids: Vec<String>) -> Result<(), String> {
    blocking(move || filecore::bin_purge(&ids).map_err(|e| e.to_string())).await
}

/// Vysype cely kos.
#[tauri::command]
pub async fn bin_empty_cmd() -> Result<(), String> {
    blocking(|| filecore::bin_empty().map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn delete_cmd(path: String, permanent: Option<bool>) -> Result<(), String> {
    blocking(move || {
        filecore::remove_entry(path, permanent.unwrap_or(false)).map_err(|e| e.to_string())
    })
    .await
}

#[derive(Clone, serde::Serialize)]
struct TransferEvent {
    job: String,
    #[serde(flatten)]
    progress: filecore::TransferProgress,
}

/// Kopie (`mode = "copy"`) nebo presun (`"move"`) vice polozek do slozky `dest`.
/// Prubeh chodi jako udalost `transfer-progress`; zrusit jde pres
/// `cancel_job_cmd` s klicem `transfer-<job>`.
#[tauri::command]
pub async fn transfer_cmd(
    app: tauri::AppHandle,
    paths: Vec<String>,
    dest: String,
    mode: String,
    job: String,
) -> Result<filecore::TransferSummary, String> {
    use tauri::Emitter;
    let key = format!("transfer-{}", job);
    let gen = job_start(&key);
    blocking(move || {
        let on_progress = |p: &filecore::TransferProgress| {
            let _ = app.emit(
                "transfer-progress",
                TransferEvent { job: job.clone(), progress: p.clone() },
            );
        };
        let cancelled = || !job_current(&key, gen);
        filecore::transfer_entries(
            &paths,
            std::path::Path::new(&dest),
            mode == "move",
            &on_progress,
            &cancelled,
        )
        .map_err(|e| e.to_string())
    })
    .await
}

/// Syrove bytes pro nahled (obrazek, audio, video). Frontend z nich udela blob URL.
/// Vraci `tauri::ipc::Response`, takze se neposila base64 - jen binarni data.
#[tauri::command]
pub async fn read_bytes_cmd(path: String, max_bytes: Option<u64>) -> Result<tauri::ipc::Response, String> {
    blocking(move || {
        let limit = max_bytes.unwrap_or(64 * 1024 * 1024);
        let md = std::fs::metadata(&path).map_err(|e| e.to_string())?;
        if md.len() > limit {
            return Err(format!(
                "Soubor je pro náhled moc velký ({}). Otevři ho v systému.",
                filecore::format::format_size(md.len())
            ));
        }
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        Ok(tauri::ipc::Response::new(bytes))
    })
    .await
}

#[tauri::command]
pub fn read_text_cmd(path: String) -> Result<String, String> {
    filecore::read_text_preview(&path, 1024 * 1024).map_err(|e| e.to_string())
}

/// Otevre soubor nebo slozku systemovym programem (na Androidu systemovym pickerem).
#[tauri::command]
pub fn open_with_system_cmd(app: tauri::AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}

/// "granted" / "missing" na Androidu, "desktop" vsude jinde.
/// Android 11+ scoped storage: bez MANAGE_EXTERNAL_STORAGE je /storage/emulated/0 nedostupne.
#[tauri::command]
pub fn storage_state_cmd() -> String {
    #[cfg(target_os = "android")]
    {
        // read_dir korene funguje i bez opravneni (jen vrati okleste obsah: bez APK, PDF...),
        // proto se zkousi zapis souboru do korene uloziste - to projde jen s pristupem
        // ke vsem souborum.
        let probe = std::path::Path::new("/storage/emulated/0/.fm_access_probe");
        return match std::fs::File::create(probe) {
            Ok(_) => {
                let _ = std::fs::remove_file(probe);
                "granted".to_string()
            }
            Err(_) => "missing".to_string(),
        };
    }
    #[cfg(not(target_os = "android"))]
    {
        "desktop".to_string()
    }
}
