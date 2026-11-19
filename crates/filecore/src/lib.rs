//! filecore - ciste Rust jadro file manageru
//! Zadny Tauri, zadny UI. Jen filesystem, rychle a bezpecne.
//!
//! Pouziti:
//!   let entries = filecore::list_dir(".")?;
//!   let hits = filecore::search(".", "fotka", 50);

pub mod archive;
pub mod format;
pub mod models;
pub mod ops;
pub mod places;
pub mod scanner;
pub mod search;
pub mod sizes;
pub mod thumbs;
pub mod transfer;

pub use archive::{unzip_archive, zip_entries};
pub use models::{FileEntry, FileInfo, FileKind, Place, SearchResult, SizeEntry, SpaceInfo};
pub use ops::{
    copy_entry, create_dir, delete_entry, get_info, move_entry, read_text_preview, remove_entry,
    rename_entry, trash_available, trash_entry,
};
pub use places::{disk_space, list_places};
pub use scanner::{list_dir, list_dir_opts, list_roots};
pub use search::{search, search_with, SearchOptions};
pub use sizes::{child_sizes, dir_size};
pub use transfer::{transfer_entries, TransferProgress, TransferSummary};
pub use thumbs::{thumbnail_jpeg, thumbnail_supported};
