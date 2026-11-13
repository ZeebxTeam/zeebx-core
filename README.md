# zeebx-core

The Qualcomm **BREW** platform as the [Zeebo](https://en.wikipedia.org/wiki/Zeebo) ran it, extracted
from the [Zeebx](https://github.com/ZeebxTeam/zeebx-emu) emulator.

This is the part of Zeebx that knows BREW and nothing about the host: no CPU, no window, no
sound card. Use it to write tools that read BREW games, or as the platform layer of your own
emulator.

## What is inside

| Module | What it does |
|---|---|
| `formats::modfile` | The `.mod` executable: raw and `BREW`-header variants, entry point. |
| `formats::miffile` | The `.mif` manifest: applet ClassIDs, required/provided classes. |
| `formats::resfile` | The `.bar` resource file: strings (UTF-16/Latin-1), images, blobs. |
| `brew::aee` | The interface list and the `(interface, slot)` ⇄ trampoline address encoding. |
| `brew::aee_slots` | The method names, in vtable order, of every interface. |
| `brew::aee_helpers` | The `AEEHelperFuncs` stdlib table (`MALLOC`, `STRLEN`, …). |
| `brew::heap` | The guest allocator behind `MALLOC`/`FREE`, with all bookkeeping on the host. |
| `brew::objects` | The store of objects the emulator exposes to the guest. |
| `brew::vfs` | The file system the game sees, translated as text and confined to its virtual root. |
| `brew::cformat` | `printf`-style formatting for `DBGPRINTF` (the subset games use). |
| `brew::fmath` | The stdlib floating point (`f_calc`, `FCALC_*`, `FO_*`, `FGET_*`). |
| `brew::crypto` | AES-128 CBC (`AEECLSID_BlockAES128`), checked against FIPS-197 and SP 800-38A. |
| `brew::sql` | `AEECLSID_SQLMGR` over real SQLite (feature `sql`, on by default). |
| `layout` | The guest memory map, and the vtable/helper tables to write into it. |
| `save_state` | A versioned save state format with named sections and a CRC. |

## Example

```rust
use zeebx_core::brew::aee::{self, Interface};
use zeebx_core::formats::miffile::MifFile;

let mif = MifFile::parse(&std::fs::read("game.mif")?)?;
println!("main applet: {:x?}", mif.main_applet());

// Your CPU backend stopped at an address in the API range: which call was it?
let addr = aee::encode(Interface::Shell, 3);
println!("the game called {}", aee::describe(addr));
```

A tool that dumps a `.mod`, `.mif` or `.bar`:

```sh
cargo run --example inspect -- game.mif game.mod game.bar
```

## Features

- `sql` (default): `brew::sql`, which builds a bundled SQLite. Disable it with
  `default-features = false` if you only need the formats and the AEE tables.

## License

GPL-2.0-or-later, the same as Zeebx.
