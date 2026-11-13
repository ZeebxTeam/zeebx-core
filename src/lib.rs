//! A host-independent implementation of the Qualcomm BREW platform, as shipped on the Zeebo.
//!
//! The crate knows BREW and nothing about the host: it has no CPU, no window and no sound card.
//! What it gives you:
//!
//! - [`formats`]: parsers for the `.mod` executable, the `.mif` manifest and the `.bar` resources;
//! - [`brew::aee`] and [`brew::aee_slots`]: the interface list, the order of the methods in each
//!   vtable, and the encoding of a `(interface, slot)` pair into a trampoline address;
//! - [`layout`]: the guest memory map and the vtable/helper tables to write into it;
//! - [`brew::heap`], [`brew::objects`], [`brew::vfs`]: the guest allocator, the object store and
//!   the file system as the game sees it;
//! - [`brew::cformat`], [`brew::fmath`], [`brew::crypto`], `brew::sql`: `DBGPRINTF`, the soft
//!   float helpers, AES and `SQLMGR` (the latter behind the `sql` feature);
//! - [`save_state`]: the versioned, sectioned save state format the pieces above serialize into.
//!
//! # How a BREW call reaches you
//!
//! Every slot of every vtable the loader writes ([`layout::build_vtables`]) holds an address in
//! the [`layout::API_BASE`] range. When the guest CPU jumps there, your CPU backend stops, and
//! [`brew::aee::decode`] tells you which interface and which method the game called:
//!
//! ```
//! use zeebx_core::brew::aee::{self, Interface};
//! use zeebx_core::brew::heap::Heap;
//! use zeebx_core::layout;
//!
//! let addr = aee::encode(Interface::Shell, 3);
//! assert!(addr >= layout::API_BASE && addr < layout::API_BASE + layout::API_SIZE);
//! let (iface, slot) = aee::decode(addr).unwrap();
//! assert_eq!(iface, Interface::Shell);
//! println!("the game called {}", aee::describe(addr));
//!
//! // The guest heap hands out guest addresses; the bytes live in your memory map.
//! let mut heap = Heap::new(layout::HEAP_BASE, layout::HEAP_SIZE);
//! let ptr = heap.alloc(64).unwrap();
//! assert_eq!(heap.size_of(ptr), Some(64));
//! # let _ = slot;
//! ```
//!
//! # What is not here
//!
//! The ARM CPU, the guest memory, and the implementation of each interface (`IDisplay`,
//! `IMedia`, GLES…) belong to the host, because they are bound to video, audio and input.

pub mod brew;
pub mod formats;
pub mod layout;
pub mod save_state;

#[cfg(all(test, feature = "sql"))]
pub(crate) mod scratch;

// Compiles the code blocks of the README, so the examples there cannot rot.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
