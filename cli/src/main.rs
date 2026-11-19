use std::env;
use std::path::PathBuf;

fn print_help() {
    println!("filemanager-cli - pomocny test jadra filecore (File manager in RUST)");
    println!("");
    println!("  cargo run -p filemanager-cli -- list <cesta>");
    println!("  cargo run -p filemanager-cli -- roots");
    println!("  cargo run -p filemanager-cli -- search <cesta> <dotaz> [--max 20]");
    println!("  cargo run -p filemanager-cli -- info <cesta>");
    println!("  cargo run -p filemanager-cli -- mkdir <rodic> <nazev>");
    println!("  cargo run -p filemanager-cli -- copy <zdroj> <cilova_slozka>");
    println!("  cargo run -p filemanager-cli -- move <zdroj> <cilova_slozka>");
    println!("  cargo run -p filemanager-cli -- zip <cilova_slozka> <nazev> <cesta>...");
    println!("  cargo run -p filemanager-cli -- unzip <archiv.zip>");
    println!("  cargo run -p filemanager-cli -- sizes <cesta>");
    println!("  cargo run -p filemanager-cli -- places");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_help();
        return;
    }
    match args[1].as_str() {
        "list" => {
            let dir = args.get(2).map(|s| s.as_str()).unwrap_or(".");
            match filecore::list_dir(dir) {
                Ok(entries) => {
                    println!("{} polozek v {}", entries.len(), dir);
                    for e in entries {
                        let icon = if e.is_dir { "[DIR] " } else { "      " };
                        println!("{}{}  {}  {}", icon, e.name, e.size_human, e.path.display());
                    }
                }
                Err(err) => eprintln!("Chyba: {}", err),
            }
        }
        "roots" => {
            for r in filecore::list_roots() {
                println!("{}", r.display());
            }
        }
        "search" => {
            let dir = args.get(2).map(|s| s.as_str()).unwrap_or(".");
            let q = args.get(3).map(|s| s.as_str()).unwrap_or("");
            let hits = filecore::search(dir, q, 50);
            println!("Nalezeno {} pro '{}'", hits.len(), q);
            for h in hits {
                println!("{}  {}", if h.is_dir { "[DIR]" } else { "     " }, h.path.display());
            }
        }
        "info" => {
            let p = args.get(2).map(|s| PathBuf::from(s)).unwrap_or_else(|| PathBuf::from("."));
            match filecore::get_info(&p) {
                Ok(info) => {
                    println!("Nazev: {}", info.entry.name);
                    println!("Cesta: {}", info.entry.path.display());
                    println!("Velikost: {}", info.entry.size_human);
                    println!("Je slozka: {}", info.entry.is_dir);
                    if let Some(c) = info.children_count {
                        println!("Polozek uvnitr: {}", c);
                    }
                }
                Err(e) => eprintln!("Chyba: {}", e),
            }
        }
        "mkdir" => {
            let parent = args.get(2).map(|s| s.as_str()).unwrap_or(".");
            let name = args.get(3).map(|s| s.as_str()).unwrap_or("Nova slozka");
            match filecore::create_dir(parent, name) {
                Ok(p) => println!("Vytvoreno: {}", p.display()),
                Err(e) => eprintln!("Chyba: {}", e),
            }
        }
        "copy" | "move" => {
            let from = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let dest = args.get(3).map(|s| s.as_str()).unwrap_or(".");
            let result = if args[1] == "copy" {
                filecore::copy_entry(from, dest)
            } else {
                filecore::move_entry(from, dest)
            };
            match result {
                Ok(p) => println!("Hotovo: {}", p.display()),
                Err(e) => eprintln!("Chyba: {}", e),
            }
        }
        "zip" => {
            let dest = args.get(2).map(|s| s.as_str()).unwrap_or(".");
            let name = args.get(3).map(|s| s.as_str()).unwrap_or("archiv");
            let paths: Vec<&String> = args.iter().skip(4).collect();
            match filecore::zip_entries(&paths, std::path::Path::new(dest), name) {
                Ok(p) => println!("Zabaleno: {}", p.display()),
                Err(e) => eprintln!("Chyba: {}", e),
            }
        }
        "unzip" => {
            let archive = args.get(2).map(|s| s.as_str()).unwrap_or("");
            match filecore::unzip_archive(archive) {
                Ok(p) => println!("Rozbaleno do: {}", p.display()),
                Err(e) => eprintln!("Chyba: {}", e),
            }
        }
        "sizes" => {
            let dir = args.get(2).map(|s| s.as_str()).unwrap_or(".");
            match filecore::child_sizes(dir, &|| false) {
                Ok(list) => {
                    for e in list {
                        println!("{:>10}  {}{}", e.size_human, e.name, if e.is_dir { "/" } else { "" });
                    }
                }
                Err(e) => eprintln!("Chyba: {}", e),
            }
        }
        "places" => {
            for p in filecore::list_places() {
                let space = p
                    .space
                    .map(|s| format!("  volno {} z {}", s.free_human, s.total_human))
                    .unwrap_or_default();
                println!("[{}] {}  {}{}", p.kind, p.label, p.path.display(), space);
            }
        }
        _ => print_help(),
    }
}
