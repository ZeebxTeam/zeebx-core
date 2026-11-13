# zeebx-core

[![crates.io](https://img.shields.io/crates/v/zeebx-core.svg)](https://crates.io/crates/zeebx-core)
[![docs.rs](https://img.shields.io/docsrs/zeebx-core)](https://docs.rs/zeebx-core)
[![license](https://img.shields.io/crates/l/zeebx-core.svg)](LICENSE)

A host-independent implementation of the **Qualcomm BREW** platform, as shipped on the Zeebo.

`zeebx-core` covers everything that is BREW and nothing that is host: file formats, the AEE
interface tables, the guest memory layout, the heap, the virtual file system and the stdlib
helpers. It has no CPU, no window and no audio device, which makes it a fit both for tooling that
inspects BREW software and as the platform layer of a BREW runtime.

## Highlights

- **File formats** — parsers for the `.mod` executable (raw and `BREW`-header variants), the
  `.mif` manifest and the `.bar` resource file, with typed errors.
- **AEE interface tables** — 63 interfaces with their methods in vtable order, and a reversible
  encoding between `(interface, slot)` and trampoline addresses.
- **Memory layout** — the guest address map and builders for the vtable and `AEEHelperFuncs`
  tables a loader writes into it.
- **Runtime services** — guest heap, reference-counted object store, sandboxed VFS,
  `DBGPRINTF` formatting, stdlib floating point, AES-128 CBC and `SQLMGR` over SQLite.
- **Save states** — a versioned, sectioned binary format with CRC-32 integrity checks.
- **Small dependency tree** — `crc32fast`, plus `rusqlite` behind an optional feature.

## Installation

```toml
[dependencies]
zeebx-core = "0.1"
```

To skip the bundled SQLite build when `SQLMGR` support is not needed:

```toml
[dependencies]
zeebx-core = { version = "0.1", default-features = false }
```

## Usage

### Reading a game package

```rust,no_run
use zeebx_core::formats::{miffile::MifFile, modfile::ModImage, resfile::ResFile};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mif = MifFile::parse(&std::fs::read("game.mif")?)?;
    println!("main applet: {:x?}", mif.main_applet());

    let module = ModImage::parse(std::fs::read("game.mod")?)?;
    println!("{:?} image, {} bytes", module.variant(), module.image().len());

    let resources = ResFile::parse(std::fs::read("game.bar")?)?;
    if let Some(text) = resources.string(1) {
        println!("string 1: {}", String::from_utf16_lossy(&text));
    }
    Ok(())
}
```

### Dispatching an API call

Every vtable slot points into a reserved address range. When the guest CPU branches there, the
address identifies the interface and method being called:

```rust
use zeebx_core::brew::aee::{self, Interface};
use zeebx_core::layout;

let addr = aee::encode(Interface::Shell, 3);
assert!((layout::API_BASE..layout::API_BASE + layout::API_SIZE).contains(&addr));

let (iface, slot) = aee::decode(addr).unwrap();
assert_eq!((iface, slot), (Interface::Shell, 3));
println!("{}", aee::describe(addr)); // interface and method name
```

### Allocating guest memory

```rust
use zeebx_core::brew::heap::Heap;
use zeebx_core::layout::{DEFAULT_HEAP_SIZE, HEAP_BASE};

let mut heap = Heap::new(HEAP_BASE, DEFAULT_HEAP_SIZE);
let ptr = heap.alloc(64).expect("heap has room");
assert_eq!(heap.size_of(ptr), Some(64));
heap.free(ptr);
```

A command-line inspector for `.mod`, `.mif` and `.bar` files ships as an example:

```sh
cargo run --example inspect -- game.mif game.mod game.bar
```

## Modules

| Module | Contents |
|---|---|
| `formats::modfile` | `.mod` executable: variant detection, header, entry point. |
| `formats::miffile` | `.mif` manifest: applet ClassIDs, required and provided classes. |
| `formats::resfile` | `.bar` resources: strings (UTF-16 and Latin-1), images, binary blobs. |
| `brew::aee` | Interface catalogue and trampoline address encoding. |
| `brew::aee_slots` | Method names of every interface, in vtable order. |
| `brew::aee_helpers` | The `AEEHelperFuncs` stdlib table (`MALLOC`, `STRLEN`, …). |
| `brew::heap` | Guest allocator behind `MALLOC`/`FREE`, with host-side bookkeeping. |
| `brew::objects` | Store of reference-counted objects exposed to the guest. |
| `brew::vfs` | Guest file system, resolved as text and confined to its virtual root. |
| `brew::cformat` | `printf`-style formatting for `DBGPRINTF` and `WSPRINTF`. |
| `brew::fmath` | Stdlib floating point (`f_calc`, `FCALC_*`, `FO_*`, `FGET_*`). |
| `brew::crypto` | AES-128 CBC (`AEECLSID_BlockAES128`), tested against FIPS-197 and SP 800-38A. |
| `brew::sql` | `AEECLSID_SQLMGR` over SQLite *(feature `sql`)*. |
| `layout` | Guest memory map and vtable/helper table builders. |
| `save_state` | Versioned save state format with named sections and CRC-32. |

## Guest memory map

| Region | Base | Size |
|---|---|---|
| Module image | `0x0001_0000` | image + 1 MiB `.bss` |
| Extension modules | `0x0800_0000` | 8 × 16 MiB |
| Heap | `0x1000_0000` | host-defined, 64 MiB recommended (max 256 MiB) |
| Stack | `0x2000_0000` | 1 MiB |
| Objects | `0x3000_0000` | 4 MiB |
| Helper table | `0x3100_0000` | 256 slots |
| Code stubs | `0x3200_0000` | 4 KiB |
| Host-internal allocations | `0x3400_0000` | 16 MiB |
| Surfaces | `0x4000_0000` | 8 MiB |
| Vtables | `0xE000_0000` | 256 slots per interface |
| API trampolines | `0xF000_0000` | 16 MiB |

## Feature flags

| Feature | Default | Description |
|---|---|---|
| `sql` | yes | Enables `brew::sql`. Builds a bundled SQLite through `rusqlite`. |

## Minimum supported Rust version

Rust **1.93** (edition 2024).

## License

Licensed under the [GNU General Public License v2.0 or later](LICENSE).
