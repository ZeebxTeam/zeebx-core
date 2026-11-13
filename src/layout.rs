//! The guest memory map of a BREW module, and the vtable/helper tables the loader writes into it.
//!
//! These are the addresses the AEE contract needs to agree on: the module image, the heap, the
//! object region, and the trampoline ranges the dispatcher decodes. The actual mapping (and the
//! call to `AEEMod_Load`) belongs to whoever owns the CPU — this crate only says **where**.

use crate::brew::aee::{self, Interface};

/// Base of the range reserved for the BREW vtables: every slot points into it, and an execution
/// that lands here is an API call. This is how the dispatcher finds out which API the game called.
pub const API_BASE: u32 = 0xf000_0000;
/// Size of the API range.
pub const API_SIZE: u32 = 0x0100_0000;
/// Sentinel address `lr` receives to mark "returned from the API".
pub const RETURN_MAGIC: u32 = 0xfff0_0000;

/// Where the module image is loaded.
///
/// The linker script sets `ro-base = 0`, but the module **cannot** sit at address 0: the stub
/// that `elf2mod` puts at the start computes its own base with PC-relative arithmetic and then
/// reads two words **before** it (`ldr r0, [r1, #-8]` and `[r1, #-4]`, with `r1` = base). The
/// AEE loader therefore reserves a prefix before the image. Since the stub is
/// position-independent, any base works — as long as there is valid memory right below it.
pub const MODULE_BASE: u32 = 0x0001_0000;
/// Size of the prefix reserved before the image, for the words the stub reads at negative
/// offsets. One page is more than enough slack.
pub const MODULE_PREFIX: u32 = 0x1000;
/// Zeroed space reserved after the image, for the module's `.bss`.
///
/// The `.mod` stores only the sections with content; the `.bss` exists at run time and not in
/// the file. Until we know how to read its real size from the header, we reserve a generous
/// slack — leftover zeroed memory is harmless.
pub const MODULE_BSS_SLACK: usize = 1024 * 1024;
/// The module's heap — where `MALLOC` serves from.
///
/// Only the **base** is fixed. The size is the host's choice, and it is what [`Heap::new`]
/// receives: anything up to [`HEAP_MAX_SIZE`] fits before the stack. A host with little memory
/// can go below [`DEFAULT_HEAP_SIZE`], at the cost of the games that measure free memory before
/// loading.
///
/// [`Heap::new`]: crate::brew::heap::Heap::new
pub const HEAP_BASE: u32 = 0x1000_0000;
/// The largest heap that fits between [`HEAP_BASE`] and [`STACK_BASE`].
pub const HEAP_MAX_SIZE: usize = (STACK_BASE - HEAP_BASE) as usize;
/// The recommended heap size.
///
/// 64 MiB because Quake measures free memory before loading the `.pak` files and gives up with
/// "Not enough free memory" if it is small. The console has 128 MB of RAM, and the size here
/// only needs to be roomy enough for the game to accept the device.
///
/// **Measured, and the number stayed:** doubling to 128 MiB does not move the Z-Wheel. It
/// allocates ~64 MiB in its own pool (its `memcheck` prints `Free(67001392)`, the same number at
/// both sizes) and then asks for **all that is left** — 67 001 488 with 64 MiB, 134 110 352 with
/// 128 MiB. The request follows the heap instead of being a need of its own, so growing the heap
/// only moves the target. What remained from that measurement is the honest accounting of "how
/// much is free": see [`crate::brew::heap::Heap::largest_block`].
pub const DEFAULT_HEAP_SIZE: usize = 64 * 1024 * 1024;
/// Kept so code written against 0.1.0 still builds.
#[deprecated(since = "0.1.1", note = "the heap size is the host's choice; use `DEFAULT_HEAP_SIZE` or your own value")]
pub const HEAP_SIZE: usize = DEFAULT_HEAP_SIZE;
/// Stack. `sp` starts at the top because the ARM stack grows downwards.
pub const STACK_BASE: u32 = 0x2000_0000;
pub const STACK_SIZE: usize = 1024 * 1024;
/// Objects the host exposes to the guest — each one starts with the vtable pointer.
///
/// Four megabytes give 65,536 objects, and the number is not a whim: with 64 KB it was **one
/// thousand and twenty-four**, and the Z-Wheel used them up in a single run. After that
/// everything fails at once — `Couldn't open DB: 3`, `Unable to create vector model`, forms with
/// error 3 — and none of those symptoms looks like the cause.
///
/// One thousand and twenty-four was **our** limit, not the console's: there, objects come out of
/// the BREW heap, which has tens of megabytes. The region is zeroed and only takes up what gets
/// touched, so the slack does not cost real memory.
///
/// This does **not** excuse fixing whatever leaks. But a ceiling of a thousand objects turns any
/// small leak into a cascading failure, and a cascading failure hides the cause.
pub const OBJECT_BASE: u32 = 0x3000_0000;
pub const OBJECT_SIZE: usize = 4 * 1024 * 1024;
/// Memory for the surfaces' pixels.
///
/// It must live in the guest's space because `IDIB` hands the game the buffer pointer so it can
/// draw directly — that is how the commercial games write to the screen.
pub const SURFACE_BASE: u32 = 0x4000_0000;
pub const SURFACE_SIZE: usize = 8 * 1024 * 1024;

/// Memory that **the host** requests in the guest's space, outside the game's heap.
///
/// `IThread` stacks, the `IPeek` line buffer, the `AEEImageInfo`, the `eglQueryString` strings
/// and the out pointers passed to the game's callbacks. Interleaving them with what the game
/// requests would make the address of every game structure depend on the **host's
/// implementation**: one extra block and everything after it shifts. Keeping them apart makes the
/// address of every game structure depend only on what the game requests.
///
/// What the game requests **through an API** — `STRDUP`, `LoadResData`, `IHeap_Malloc` — stays
/// in the heap: there it is the system allocating on the game's behalf, and the size comes from
/// the game, not from us.
///
/// 16 MB because `IPeek` reserves the size of the whole font and each thread takes at least
/// 64 KB.
pub const INTERNAL_BASE: u32 = 0x3400_0000;
pub const INTERNAL_SIZE: usize = 16 * 1024 * 1024;

/// The address belongs to a block of the internal region, and not to the game's heap.
pub fn is_internal(addr: u32) -> bool {
    (INTERNAL_BASE..INTERNAL_BASE + INTERNAL_SIZE as u32).contains(&addr)
}

/// Where the **extension** modules are mapped.
///
/// An extension module is a `.mod` like any other: same image linked at base zero, same
/// `AEEMod_Load`, same two helper words right before the base. What changes is who calls it —
/// the console loads it when a game asks for a class it provides, and not because the user
/// opened a title.
///
/// Mapping them at load time, and not when they are requested, is a simplicity decision: the
/// core has no API to map a region after `reset`, and the modules are small (90 KB and 164 KB in
/// the two packages we have). What is left for request time is **calling** `AEEMod_Load`.
pub const EXT_MODULE_BASE: u32 = 0x0800_0000;
/// Distance between two extensions. It must fit the image, the prefix and the `.bss`.
pub const EXT_MODULE_SPACING: u32 = 0x0100_0000;
/// How many extensions fit before running into the guest heap, at `0x10000000`.
pub const MAX_EXTENSIONS: usize = 8;

/// BREW's stdlib helper table.
///
/// Dynamic modules do not link against libc: they call `MALLOC`, `STRLEN` and friends through a
/// table of function pointers that the loader hands them. The generated code does
/// `ldr r1, [r0, #<offset>]` followed by `bx r1`, where `r0` is the pointer to that table — the
/// same pattern appears in `helloworld.mod` and in `tectoy.mod`, byte for byte.
///
/// We fill each entry with a trampoline address, so any helper used shows up as
/// `AEEHelpers::slot[n]` in the log instead of jumping into the void.
pub const HELPERS_BASE: u32 = 0x3100_0000;
/// How many pointers the table exposes. `struct AEEHelperFuncs` has 117 fields; we round up in
/// case some Zeebo build has a few more at the end.
pub const HELPERS_SLOTS: u32 = 256;

/// Snippets of ARM code that the host writes for the guest to execute.
///
/// They serve the helpers that always return the same thing. Answering one of them through the
/// trampoline costs stopping and restarting the ARM core, and `GetAppInstance` alone accounts for
/// more than half of Quake's API calls — BREW games keep their globals inside the applet, so
/// every access to a global goes through it. As code, the call never even leaves the CPU.
pub const STUB_BASE: u32 = 0x3200_0000;
pub const STUB_SIZE: usize = 0x1000;

/// Vtables: read-only memory whose slots point into the trampoline range.
pub const VTABLE_BASE: u32 = 0xe000_0000;
/// Version of the helper table, read by the module via `GET_HELPER_VER()`.
pub const HELPER_VERSION: u32 = 1;

/// How many slots we reserve per vtable. No BREW interface comes close to this.
pub const SLOTS_PER_VTABLE: u32 = 256;
/// Interfaces whose objects are created right at load time.
pub const BOOT_INTERFACES: [Interface; 2] = [Interface::Shell, Interface::Module];

/// Builds the helper table: each entry is the trampoline of the corresponding helper.
pub fn build_helper_table() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(HELPERS_SLOTS as usize * 4);
    for slot in 0..HELPERS_SLOTS {
        bytes.extend_from_slice(&aee::encode(Interface::Helpers, slot).to_le_bytes());
    }
    bytes
}

/// Address of an interface's vtable.
pub fn vtable_addr(iface: Interface) -> u32 {
    VTABLE_BASE + (iface as u32) * SLOTS_PER_VTABLE * 4
}

/// Builds all the vtables: each slot holds the trampoline address of the corresponding method.
pub fn build_vtables() -> Vec<u8> {
    let mut bytes = Vec::new();
    // Walks the canonical list, which is indexed by the enum value — the same index that
    // `vtable_addr` uses to compute the address.
    for iface in Interface::ALL {
        for slot in 0..SLOTS_PER_VTABLE {
            bytes.extend_from_slice(&aee::encode(iface, slot).to_le_bytes());
        }
    }
    bytes
}
