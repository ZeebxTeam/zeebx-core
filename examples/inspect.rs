//! Prints what a BREW module carries: `cargo run --example inspect -- game.mod [game.mif] [game.bar]`.

use std::path::Path;

use zeebx_core::formats::{miffile::MifFile, modfile::ModImage, resfile::ResFile};

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: inspect <file.mod|file.mif|file.bar>...");
        std::process::exit(2);
    }
    for path in &paths {
        let data = std::fs::read(path).unwrap_or_else(|err| panic!("{path}: {err}"));
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        match ext.as_str() {
            "mod" => match ModImage::parse(data) {
                Ok(module) => println!(
                    "{path}: {:?}, {} bytes, entry {:#010x}",
                    module.variant(),
                    module.image().len(),
                    module.entry()
                ),
                Err(err) => println!("{path}: {err:?}"),
            },
            "mif" => match MifFile::parse(&data) {
                Ok(mif) => {
                    println!("{path}: main applet {:x?}", mif.main_applet());
                    println!("  applets: {:x?}", mif.applets);
                    println!("  classes: {:x?}", mif.classes);
                }
                Err(err) => println!("{path}: {err:?}"),
            },
            "bar" => match ResFile::parse(data) {
                Ok(bar) => {
                    println!("{path}: resource file");
                    // Probes the first ids: the format has no table of contents by name.
                    for id in 1..=32 {
                        if let Some(text) = bar.string(id) {
                            println!("  string {id}: {:?}", String::from_utf16_lossy(&text));
                        }
                    }
                }
                Err(err) => println!("{path}: {err:?}"),
            },
            _ => println!("{path}: unknown extension"),
        }
    }
}
