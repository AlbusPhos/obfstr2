# obfstr2

> **Polymorphic compile-time string/wide-str/wide-cstr/bytes/int/float/cstr/file obfuscation**

[![Crates.io](https://img.shields.io/crates/v/obfstr2?logo=rust)](https://crates.io/crates/obfstr2)
[![Docs.rs](https://img.shields.io/docsrs/obfstr2?logo=docs.rs)](https://docs.rs/obfstr2)
[![CI](https://img.shields.io/github/actions/workflow/status/AlbusPhos/obfstr2/ci.yml?logo=github&label=CI)](https://github.com/AlbusPhos/obfstr2/actions)
[![MSRV](https://img.shields.io/crates/msrv/obfstr2?logo=rust&label=MSRV)](https://crates.io/crates/obfstr2)
[![Downloads](https://img.shields.io/crates/d/obfstr2?color=blue)](https://crates.io/crates/obfstr2)
[![License](https://img.shields.io/crates/l/obfstr2)](https://mozilla.org/MPL/2.0/)

**Language:** English | [简体中文](README.md)

A drop-in, single-macro compile-time polymorphic obfuscation framework for Rust. Literal in, secure expression out—all encryption and control-flow mutations happen at compile time. Every build generates completely distinct binary signatures and ciphertext layouts, thoroughly defeating static signature scanning, Mandiant `FLOSS`, and dedicated Rust deobfuscators (such as [`rust-deobf`](https://github.com/VAHPEM/rust-deobf)).

Comprehensive coverage for **strings, C-strings, UTF-16 wide strings, byte arrays, external files, integers, floats, and format strings**. Supports dereferencing directly to raw values or storing in managed containers; all string and byte containers automatically perform volatile memory zeroing on `Drop` to prevent sensitive plaintext from lingering on the stack or heap.

## Table of Contents

- [Quick Start](#quick-start)
- [Core Highlights & Adversarial Validation](#core-highlights--adversarial-validation)
- [Design Philosophy & Scope](#design-philosophy--scope)
- [Macro Overview](#macro-overview)
- [Feature Flags](#feature-flags)
- [Benchmarks](#benchmarks)
- [Security & Safety](#security--safety)
- [Contributing & Development Verification](#contributing--development-verification)
- [License](#license)

## Quick Start

```toml
[dependencies]
obfstr2 = "0.4"
```

All obfuscation macros use a numeric suffix to denote the intensity tier: `1` (low latency), `2` (balanced, recommended), and `3` (high-intensity anti-RE);
additionally, the [`*_user!`](#custom-intensity-tier-_user) custom tier allows overriding parameters field-by-field on top of the balanced tier using a configuration block.

```rust
use obfstr2::{b2, cs2, cw2, f2, fl2, i2, s2, s_fmt2, w1};

fn main() {
    // Strings: returns an owned secure container; can be bound, passed, and reused, and automatically zeroes memory on Drop
    let hello = s2!("hello");
    print!("{hello}");

    // C-strings: dereferences to &CStr; as_ptr() can be passed directly to low-level FFI/syscalls
    let sh = cs2!(c"/bin/sh");
    assert_eq!(&*sh, c"/bin/sh");

    // Wide C-strings: UTF-16 code units + automatic trailing 0u16; as_ptr() is ready for Win32 PCWSTR
    let w = cw2!("kernel32.dll");
    assert_eq!(w[w.len() - 1], 0);

    // Wide strings: pure UTF-16 code units without a trailing NUL; interior NULs are allowed
    let ws = w1!("a\0b");
    assert_eq!(&*ws, &[0x61u16, 0, 0x62][..]);

    // Byte strings & byte arrays: dereferences to &[u8]
    let b = b2!(b"abc");
    let c = b2!([0x61, 98, 99]);
    assert_eq!(&*b, &*c);

    // Scalar integers & floats: evaluates directly to primitive values ready for arithmetic
    let x = i2!(42u32);
    assert_eq!(x + 1, 43u32);
    let y = fl2!(3.15);
    assert_eq!(y.to_bits(), 3.15f64.to_bits());

    // External files: path resolved relative to the current crate's CARGO_MANIFEST_DIR, read and obfuscated at compile time
    let asset = f2!("assets/fixture.bin");

    // Format strings: obfuscates static literal fragments individually before feeding into format!, returning a String
    let name = "world";
    let greeting = s_fmt2!("hello, {}!", name);
    print!("{greeting}");
}
```

For more real-world examples, see [`examples/showcase.rs`](examples/showcase.rs) (run via `cargo run --example showcase`).

---

## Core Highlights & Adversarial Validation

`obfstr2` is not a simple "compile-time XOR" macro, but a polymorphic cryptographic obfuscation pipeline deeply tailored against **dedicated deobfuscators, white-box reverse engineering, decompilers (IDA / Ghidra F5), and symbolic execution engines (angr / Z3)**.

### Real-World Adversarial Validation: Full Immunity Against `rust-deobf`

While traditional compile-time obfuscation crates can bypass `strings` and general-purpose tools like `FLOSS`, they completely collapse against [**`rust-deobf`**](https://github.com/VAHPEM/rust-deobf)—a dedicated deobfuscator specifically engineered for Rust binary signatures and control-flow patterns (as those crates rely on fixed inline XOR patterns, static key layouts, or standard AEAD algorithm signatures).

Powered by a polymorphic primitive pipeline, cross-chunk ghost state coupling, and call-graph shattering, when tested against [`examples/showcase.rs`](examples/showcase.rs) containing diverse protected payloads, **not a single macro-protected string in `obfstr2` was recovered (0 recovered)**:

| Obfuscation Crate / Technique             |  `strings`   |   `FLOSS`    | [`rust-deobf`](https://github.com/VAHPEM/rust-deobf) | Anti-Analysis & Deobfuscation Assessment                                                                                 |
|:------------------------------------------|:------------:|:------------:|:----------------------------------------------------:|:-------------------------------------------------------------------------------------------------------------------------|
| `litcrypt` / `litcrypt2`                  |    0 / 5     |    0 / 5     |                **5 / 5 (All Broken)**                | ❌ Obvious static byte-XOR signatures                                                                                    |
| `obfstr` (Inline XOR+ADD)                 |    0 / 5     |    0 / 5     |                **5 / 5 (All Broken)**                | ❌ Fixed inline unrolling patterns, easily recovered via instruction emulation                                           |
| `cryptify` (Default Key)                  |    0 / 5     |    0 / 5     |                **5 / 5 (All Broken)**                | ❌ Fixed decryption routine and static key layout                                                                        |
| `muddy` (ChaCha20-Poly1305)               |    0 / 5     |    0 / 5     |                **5 / 5 (All Broken)**                | ❌ Standard cryptographic constants and AEAD call structures are accurately fingerprinted                                |
| Hand-rolled compact stack str / XOR / ADD |    0 / 5     |    0 / 5     |                **5 / 5 (All Broken)**                | ❌ Lacks control-flow barriers and cross-block state coupling                                                            |
| **`obfstr2` (`showcase` benchmark)**      | **0 dumped** | **0 dumped** |             **0 dumped (Fully Immune)**              | ✅ **Non-standard polymorphic primitives + cross-chunk avalanche ratchet + vtable trampolines block automated recovery** |

*Note: The first 6 baseline entries are cited directly from the [official `rust-deobf` evaluation benchmark](https://github.com/VAHPEM/rust-deobf).*

> **Commitment to Continuous Adversarial Evolution:**
> Reverse and anti-reverse engineering is a dynamic cat-and-mouse game. Thanks to our highly decoupled underlying registry architecture, **the maintainers commit to periodically updating and rotating the built-in cryptographic primitives and steganographic pseudo-signatures**. This continuously refreshes code expansion shapes and data signatures, ensuring the dynamic polymorphic security of the latest `obfstr2` releases and rendering targeted extraction rules obsolete.

### 1. Cryptographic Primitives Resistant to Symbolic Execution & Decompilation
Built-in library of 17+ polymorphic encryption/decryption primitives spanning stream ciphers, Substitution-Permutation Networks (SPN), dynamic box transforms, and block ciphers—quantitatively graded by runtime latency and security score:
- **Proprietary MSE / MSE2 Algorithm Family**: Fuses CFB ciphertext feedback, dynamic S-Boxes / P-Boxes that mutate byte-by-byte based on historical state, 128-bit non-linear key derivation based on fixed-point approximations of transcendental numbers ( $\pi$ , $1/e$ ), and 64B-block 4-round ARX diffusion. This completely eliminates the possibility of attackers identifying standard magic constants (e.g., AES / ChaCha20 constants) and invoking off-the-shelf libraries for batch decryption.
- **Cross-Domain Floating-Point Algebraic Obfuscation**: Equivalently expands basic integer bitwise operations (rotations, XOR) into IEEE-754 floating-point divisions, truncations, and exponent manipulations. While drastically increasing the cognitive load of decompiler (F5) pseudocode, it directly triggers path explosion and solver timeouts in SMT solvers under non-linear floating-point constraints.
- **Dynamic Box Inversion & Modular Multiplicative Inverse**: Encrypts at compile time using $\pmod{2^8}$ odd multiplicative inverses and decrypts at runtime via a single multiplication; shuffles and inverts S-Boxes / P-Boxes on-the-fly at runtime from the key, completely eliminating static lookup table signatures in the binary.
- **Global Positional Diffusion**: Introduces Fisher-Yates permutations backed by 128-bit unbiased multiplicative interval mapping to break byte-boundary alignment.

### 2. Reverse Key Ratchet & Full-Chain Avalanche Coupling
Refuses to expose independent per-layer keys in the binary, constructing a bidirectional state-coupling chain across "compile-time forward encryption" and "runtime reverse decryption":
- **Zero Extra Key Storage ( $\pmod{2^{64}}$ Multiplicative Inverse Ratchet)**: Each chunk stores only a single root key (XOR-masked at compile time with the entry `ghost_state`, ensuring even single-layer chunks strongly depend on global state). Multi-layer encryption backward-derives outer keys at compile time using 64-bit odd multiplicative inverses combined with the current layer's tail byte; during runtime decryption, the next layer's key is forward-recovered from the tail byte:

$$K _ {\text{next}} = (K _ {\text{cur}} + \text{ghost\\_state}) \times (\text{last\\_byte} \mid 1) \pmod{2^{64}}$$

- **Cross-Chunk Ghost State (`ghost_state`)**: After each chunk is decrypted and appended to the main container, a sliding window of up to the last 16 bytes of the main container is hashed via `LightHasher` and XORed into the global `ghost_state` (when dynamic keys are enabled, the initial state also mixes in the key hash). Any attempt to skip preceding chunks in a debugger, patch a single byte in memory, or statically rip out an individual sub-function for emulated execution will trigger a total avalanche across all subsequent chunks and layer keys.

### 3. Superlinear Chunking, High-Fidelity Junk Chunks & Polymorphic Algebraic Opaque Predicates
- **Superlinear Chunking & Short-Chunk Padding**: Randomly slices payloads across $\lceil i^{1.4} \rceil$ increasing step intervals (fine-grained at the head to scatter signatures, smoothly growing at the tail to bound total chunk count); medium and high intensity tiers enforce a `pkcs7_16` pre-padding pass on short chunks (< 16B) to eliminate length side-channels on short strings.
- **100% High-Fidelity Junk Chunks**: Injects decoy data chunks according to tier probabilities. Junk chunks undergo the exact same multi-layer encryption, ciphertext storage, and runtime decryption pipeline as real chunks, making them indistinguishable in decompiler views.
- **Polymorphic Algebraic & Number-Theoretic Opaque Predicates**: Randomly injects 4 classes of high-strength integrity checks at the end of chunk decryption—covering **non-linear MBA (Mixed Boolean-Arithmetic) identities & $\pmod{2^{64}}$ multiplicative inverse duality**, **Hamming weight `popcnt` inclusion-exclusion principle & orthogonal bit splitting (SMT solver poison)**, **2-adic ring Hensel odd-power fixed points ( $v^2 \equiv 1 \pmod 8$ , $v^4 \equiv 1 \pmod{16}$ ) & parity checks**, and **De Morgan symmetric difference bilateral MBA**. These predicates deeply bind the runtime buffer pointer address with `ghost_state`. Guarded by `black_box` to prevent compile-time constant folding, junk chunks and state-tampered chunks deterministically trigger `wipe_data()` self-erasure at runtime, keeping the main container and global state tightly interlocked.

### 4. Call-Graph Shattering & LLVM Optimization Barriers
Purpose-built defenses against static cross-reference (Xrefs) tracking, batch function hooking, and aggressive compiler optimizations:
- **Dual-Granularity Adaptive Outlining & Vtable Trampolines**: Across both "single-layer decryption" and "chunk aggregation" granularities, the probability of outlining code into a standalone function grows exponentially with consecutive inline steps. Outlined functions randomly emit (50/50) either a named `#[inline(never)]` function or a dynamic-dispatch trampoline invoked indirectly through a `&mut dyn FnMut` fat pointer—turning direct calls into vtable indirect calls and severing the static call graph.
- **O(log M) Bottom-Up Multi-Way Folding Tree**: When the chunk count is large, adjacent groups of 4–12 chunk nodes are recursively encapsulated into a multi-level call tree, preventing the root function from growing linearly with payload size.
- **Cold/Hot Section Splitting & Anti-IPO (Interprocedural Optimization)**: Randomly attaches the `#[cold]` attribute by probability to evict decryption logic into the `.text.unlikely` section, disrupting spatial locality; casts function pointers to `usize` and feeds them through `black_box` to fake address escape, preventing LLVM from performing interprocedural constant propagation and dead argument elimination.

### 5. Polymorphic Steganographic Storage & Managed Memory Lifecycle
- **Polymorphic Ciphertext & Key Disguise**: Both ciphertext and each chunk's 8-byte initial key are encoded through randomized storage strategies—disguised as plain byte streams, `u64`/`u128` integer arrays, MAC addresses, UUIDs, IPv6 addresses, PEM certificates, or HB2 whitespace steganography to evade high-entropy data section scans. Notably, HB2 steganography embeds realistic text corpora (code snippets, logs, scripts, etc.) at compile time with randomly generated whitespace symbol tables, ensuring neither the host text nor symbol patterns can be fingerprinted.
- **Full-Chain Volatile Wiping**: Not only does the final returned container perform volatile zeroing across its entire capacity on `Drop`, but intermediate temporary containers in variable-length decryption layers are also immediately destroyed and wiped upon handoff, leaving zero plaintext or intermediate state residue in memory.
- **Fine-Grained Stack Budget (`STACK_BUDGET`)**: Control pure stack overhead caps via `budget-1k` through `budget-8m` feature flags. Compile-time analysis automatically tracks peak expansion sizes across the main container and multi-layer chunk decryption, routing intelligently between stack and heap containers and adapting seamlessly to `no_std` bare-metal and embedded firmware.

### 6. Blazing-Fast Compilation for Multi-MB Payloads (No `long_running_const_eval` Needed)
When handling multi-megabyte binary firmware or resource files, unrolling a massive payload into a single giant function causes the LLVM backend to suffer an $O(V^2)$ complexity blowup during register allocation. `obfstr2` uses native procedural macro evaluation (requiring no `#![allow(long_running_const_eval)]` attributes) combined with superlinear chunking, exponential inline backpressure, and an **O(log M) multi-way folding tree** to strictly bound the child node count of every function body to a small constant ( $\le 12$ ). This enables the LLVM backend to compile multi-MB payloads in near-linear time.

### 7. Anti-Disassembly Junk Instruction Injection
Targeting linear sweep and instruction pattern recognition in disassemblers, architecture-gated inline assembly (junk/rogue bytes) with zero data dependency on real state is probabilistically injected at the prologue and epilogue of non-inlined wrapper functions, inline decryption fragments, multi-level folding aggregators, and macro expansion root scopes—deeply interleaving garbage bytes with real decryption instructions in disassembler views.

---

## Design Philosophy & Scope

### Core Principles
1. **White-Box Threat Model**: Traditional cryptography is unbreakable in a black-box model, but in a white-box model where the attacker has total control over the binary, once standard algorithm constants (e.g., ChaCha20 magic words) are identified, extracting the Key/IV allows instant recovery via scripts or deobfuscators. `obfstr2` optimizes for **maximizing reverse-engineering cognitive friction and automated script recovery cost**, forcing attackers to manually untangle non-standard polymorphic logic instance by instance.
2. **Complementary Probability Security Model**: Encryption layer selection is never a fixed template. Instead, primitives are randomly sampled and stacked from the pool within the tier's latency cap until the combined security score reaches `min_security` (supporting per-layer contribution scaling via `security_factor`):

$$\text{Security} _ {\text{combined}} = 1 - \prod _ {i=1}^{k} \left(1 - \frac{\text{Security} _ i}{100}\right) \ge \text{min\\_security}$$

3. **Decoupled Registries & Periodic Rotation**: Encryption primitives and steganographic storage strategies are isolated into standalone static registries, leaving the orchestrator responsible solely for the "chunk → encrypt → store → fold & emit" pipeline. Backed by this pluggable architecture, maintainers periodically rotate and expand underlying algorithms and pseudo-signatures to continuously counter evolving reverse-engineering rules.
4. **Compile-Time Evaluation & Zero Redundant Checks**: The proc-macro completes all encryption, state simulation, and length/alignment invariant verification at compile time; runtime execution uses `unwrap_unchecked` directly to eliminate redundant branching overhead.

### Trade-offs

| We Chose                                 | Over                                  | Why                                                                                                                                             |
|:-----------------------------------------|:--------------------------------------|:------------------------------------------------------------------------------------------------------------------------------------------------|
| **Anti-RE Strength First**               | Minimal Binary Size                   | Cascaded decryption, realistic junk chunks, and polymorphic unrolling inevitably increase code size—a necessary price to block RE and emulators |
| **Non-Deterministic Polymorphic Builds** | Reproducible Builds                   | Guaranteeing completely distinct binary signatures on every build is the core defense against automated signature extraction and batch scripts  |
| **White-Box Engineering Resistance**     | Theoretical Everlasting Crypto Safety | The goal is to drastically raise the attacker's time and compute costs during static analysis, symbolic execution, and dynamic debugging        |

### Recommended Use Cases vs. Non-Goals

| ✅ Recommended Use Cases                                                                                                                                                                                                                                                                                                                                                                                         | ❌ Non-Goals / When Not to Use                                                                                                                                                                                                                                                                                                                                                                        |
|:-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| • Commercial software DRM, license verification constants, built-in public key / API key protection<br>• Hiding sensitive syscall/DLL names (e.g., `ntdll.dll`) in game anti-cheat and security SDKs<br>• Hiding hardcoded credentials and protocol magic numbers in `no_std` embedded firmware and bare-metal binaries<br>• Embedding and obfuscating multi-MB binary assets while requiring fast compile times | • Release pipelines requiring deterministic, bit-for-bit reproducible binary hashes<br>• Scenarios requiring compliance auditing with human-readable plaintext<br>• **Not a replacement** for standard cryptographic libraries in network transport or persistent storage encryption<br>• **Does not** apply control-flow flattening to host business logic or include built-in anti-debugging checks |

---

## Macro Overview

All macros are available in three preset tiers: `1` (low latency), `2` (balanced), and `3` (high intensity):

| Macro Family          | Input Literal Example                                       | Return Type (`Deref` Target)                                 | Features & Constraints                                                                                                                                                                           |
|:----------------------|:------------------------------------------------------------|:-------------------------------------------------------------|:-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `s1!` ~ `s3!`         | `"hello"`                                                   | `StackStr<N>` / `HeapStr<N>`<br>(`Deref<Target = str>`)      | Automatically zeroes memory on `Drop`.                                                                                                                                                           |
| `b1!` ~ `b3!`         | `b"abc"` or `[0x41, 66]`                                    | `StackBytes<N>` / `HeapBytes<N>`<br>(`Deref<Target = [u8]>`) | Array elements must be in `0..=255`; automatically zeroes memory on `Drop`.                                                                                                                      |
| `cs1!` ~ `cs3!`       | `c"..."` / `"..."` / `b"..."`                               | `StackCStr<N>` / `HeapCStr<N>`<br>(`Deref<Target = CStr>`)   | Automatically appends and obfuscates trailing `\0`, zero-copy recovery at runtime; use `b"..."` for non-UTF-8; **interior NULs are strictly forbidden** (prevents truncation); zeroes on `Drop`. |
| `w1!` ~ `w3!`         | `"a\0b"`                                                    | `StackU16<N>` / `HeapU16<N>`<br>(`Deref<Target = [u16]>`)    | Encodes to UTF-16 code units at compile time; **no** trailing NUL appended, interior NULs **allowed**; ideal for length-prefixed APIs; zeroes on `Drop`.                                         |
| `cw1!` ~ `cw3!`       | `"kernel32.dll"`                                            | `StackU16<N>` / `HeapU16<N>`<br>(`Deref<Target = [u16]>`)    | Encodes to UTF-16 at compile time and **automatically appends trailing `0u16`**; **interior NULs strictly forbidden**; `as_ptr()` can be passed directly to Win32 `PCWSTR`; zeroes on `Drop`.    |
| `f1!` ~ `f3!`         | `"assets/data.bin"`                                         | `StackBytes<N>` / `HeapBytes<N>`<br>(`Deref<Target = [u8]>`) | Path is relative to the target crate's `CARGO_MANIFEST_DIR`; zeroes on `Drop`.                                                                                                                   |
| `i1!` ~ `i3!`         | `42u32` / `-1` / `0xFFu16`                                  | Primitive integer (`u8`~`i128` / `usize`)                    | Un-suffixed literals default to `i32`; `usize`/`isize` are encoded with 64-bit semantics; no `Drop` zeroing.                                                                                     |
| `fl1!` ~ `fl3!`       | `3.15f32` / `-1.0` / `1e10`                                 | Primitive float (`f32` / `f64`)                              | Un-suffixed literals default to `f64`; rejects `inf`/`NaN`; preserves `-0.0` sign bit (use `to_bits()` for assertions); no `Drop` zeroing.                                                       |
| `s_fmt1!` ~ `s_fmt3!` | `"hi, {}!", name`                                           | `alloc::string::String`                                      | Obfuscates only static literal fragments in the format string before invoking `format!`; requires the `alloc` feature.                                                                           |
| `*_user!` (All)       | `{ max_latency: 50, ... }, payload`                         | Same as corresponding family macro                           | Custom tier: first argument is a config block overriding parameters field-by-field on top of the balanced tier (`2`).                                                                            |
| `*_key!` (All)        | `{ build_key, run_key }, payload`                           | Same as corresponding family macro                           | Dynamic key supply: encrypts with `build_key` at compile time, decrypts with `run_key` at runtime; compile-time constant `run_key` expressions are rejected.                                     |
| `*_key_user!` (All)   | `{ build_key, run_key }, { max_latency: 50, ... }, payload` | Same as corresponding family macro                           | Combines dynamic key supply with custom tier intensity configuration.                                                                                                                            |

> **Usage Note (Stack/Heap Routing & Type Inference):**
> When `alloc` (default) is enabled, payloads $\le 16\text{B}$ always use stack containers, payloads reaching the budget cap (main container `STACK_BUDGET` / intermediate chunk container `STACK_BUDGET / 2`) always use heap containers, and sizes in between are routed probabilistically based on the tier's preference. Consequently, **the concrete returned container type (`Stack*` or `Heap*`) may vary dynamically across builds**. Always rely on type inference (`let val = s2!("...");`) rather than hardcoding explicit container type annotations.

### Custom Intensity Tier `*_user!`

All families share a unified two-argument syntax: **`({ config }, payload)`**. The first argument is a braced configuration block `{ key: val, ... }`, and the second argument matches the payload of the corresponding preset macro. Configuration overrides fields on top of **Tier `2` (Balanced) as the base**:

```rust
use obfstr2::{cs_user, s_user};
fn main() {
    let greeting = s_user!(
        {
            prefer_stack: true,      // Stack bias for heap/stack coin flip
            max_latency: 60,         // Per-layer latency cap (>= 2)
            min_security: 75,        // Combined security floor (<= 99, multiplicative accumulation)
            junk_pct: 12,            // Junk chunk injection probability (0..=100)
            pad_short: true,         // Force PKCS7 pre-padding on short chunks (< 16B)
        },
        "custom-tier",
    );
    assert_eq!(&*greeting, "custom-tier");

    // An empty config block {} is equivalent to the balanced tier s2!
    assert_eq!(&*s_user!({}, "hello"), "hello");
    // Literal fragments in s_fmt_user! are obfuscated using the custom tier
    assert_eq!(cs_user!({ max_latency: 40 }, c"/bin/sh").to_str().ok(), Some("/bin/sh"));
}
```

| Config Key                  | Value Type            | Preset Defaults (`1` / **`2` Base** / `3`) | Description                                                                                                                           |
|:----------------------------|:----------------------|:------------------------------------------:|:--------------------------------------------------------------------------------------------------------------------------------------|
| `prefer_stack`              | `bool`                |       `true` / **`false`** / `false`       | Stack preference during heap/stack coin flip (`true` = 76% stack, `false` = 40% stack)                                                |
| `max_latency`               | `u8` (≥ 2)            |          `30` / **`80`** / `100`           | Latency cap for individual encryption primitives and storage strategies                                                               |
| `min_security`              | `u8` (≤ 99)           |           `7` / **`63`** / `93`            | Combined security score floor (accumulated multiplicatively; stops once reached)                                                      |
| `security_factor`           | `f64` (finite, > 0.0) |         `1.0` / **`1.0`** / `1.0`          | Per-layer security scaling factor: `< 1.0` discounts single-layer contribution (stacks more layers), `> 1.0` amplifies (fewer layers) |
| `wrap_pct`                  | `0..=100`             |           `8` / **`50`** / `85`            | Base probability of outlining layers and chunks into non-inlined functions (grows exponentially with consecutive inlines)             |
| `junk_pct`                  | `0..=100`             |           `2` / **`18`** / `35`            | Companion injection probability for high-fidelity junk chunks                                                                         |
| `cold_pct`                  | `0..=100`             |           `10` / **`20`** / `50`           | Probability of attaching `#[cold]` (cold/hot section splitting) to outlined functions                                                 |
| `pad_short`                 | `bool`                |       `false` / **`true`** / `true`        | Force `pkcs7_16` pre-padding on short chunks (< 16B) (does not count toward layer or security budgets)                                |
| `anti_disasm_pct`           | `0..=100`             |           `5` / **`40`** / `80`            | Anti-disassembly junk instruction injection probability (requires `anti-disassembly` feature)                                         |
| `anti_disasm_count_range`   | `lo..=hi` (`u8`)      |      `1..=1` / **`1..=2`** / `2..=3`       | Random range for the number of anti-disassembly blocks injected per site                                                              |
| `anti_disasm_garbage_range` | `lo..=hi` (`u8`)      |     `1..=5` / **`4..=10`** / `8..=15`      | Random range for the base number of garbage bytes per anti-disassembly block                                                          |

- Keys accept both `key: val` and `key = val` syntax, comma-separated, with trailing commas permitted both inside the config block and after the payload.

### Dynamic Key Tier `*_key!`

Adds a key-pair block to the corresponding macro family, decoupling the compile-time encryption key from the runtime decryption key:

```rust
use obfstr2::s_key;

// Fetch key from a remote server or environment; an incorrect key will yield garbage or trigger UB.
fn get_server_key() -> &'static [u8] {
    "obfstr2-demo-key".as_bytes()
}

fn main() {
    // build_key is only used for compile-time encryption and is never embedded in plaintext in the binary
    let s = s_key!(
        { build_key: "obfstr2-demo-key", run_key: get_server_key() },
        "hello",
    );
    assert_eq!(&*s, "hello");
}
```

- `build_key` accepts `"..."`, `b"..."`, `[0x12, 0x34, ...]`, and `env!("VAR")` (read at compile time and automatically registered as a Cargo environment variable dependency);
- `run_key` must be a runtime expression evaluating to `&[u8]`; compile-time constant forms such as literals, `.as_bytes()` on literals, or `env!` are rejected at compile time;
- `run_key` must match `build_key` byte-for-byte at runtime; mismatches produce undefined output and may trigger integrity self-wiping along the decryption chain (**an intentional adversarial design** to poison emulated, symbolic, or sliced execution states).

---

## Feature Flags

| Feature                   | Default | Description                                                                                                                                             |
|:--------------------------|:-------:|:--------------------------------------------------------------------------------------------------------------------------------------------------------|
| `default`                 |   ✅    | Enables `alloc` and `anti-disassembly` by default.                                                                                                      |
| `alloc`                   |   ✅    | Enables heap container (`Heap*`) support and `s_fmt*!` macros. Compatible with `no_std + alloc` environments.                                           |
| `anti-disassembly`        |   ✅    | Enables x86 / x86_64 anti-disassembly inline assembly injection with Red-Zone safety protection. Can be disabled in size- or latency-critical builds.   |
| `budget-1k` ~ `budget-8m` |   ❌    | Stack budget cap control: `1k`, `4k` (default), `16k`, `64k`, `256k`, `512k`, `1m`, `4m`, `8m`. If multiple are enabled, the largest tier takes effect. |

**Pure-Stack Bare-Metal Mode (no `alloc`, no junk assembly injection):**
```toml
[dependencies]
obfstr2 = { version = "0.4", default-features = false }
```
*Note: When `alloc` is disabled, all tiers exclusively use stack containers (`Stack*`). If the main container size reaches `STACK_BUDGET` or a single chunk's peak decryption size reaches `STACK_BUDGET / 2`, a compile-time error will prompt you to enable a larger `budget-*` feature.*

---

## Benchmarks

For small payloads (such as the 1 KiB sample below), `b1!` incurs near-zero runtime overhead; `b2!` and `b3!` trade higher runtime and compile-time costs for stronger anti-analysis resistance. Conversely, when handling **multi-MB large payloads**, thanks to native proc-macro evaluation and the multi-way folding tree design, `obfstr2` compiles significantly faster than traditional crates relying on `const` evaluation. Choose the tier that best balances your runtime performance budget and anti-RE requirements.

Below is a single-run measurement for a 1 KiB payload over 512 decryption loops (polymorphic builds fluctuate slightly across runs; intended for order-of-magnitude reference):

| Case               | Deobfuscation / Iter | Total Runtime (512 loops, incl. startup) | Incremental Build    | Cold Build           | Binary Size (unstripped) | Expanded Chars  |
|:-------------------|:---------------------|:-----------------------------------------|:---------------------|:---------------------|:-------------------------|:----------------|
| Plaintext Baseline | —                    | 10.30702ms                               | 171.832628ms         | 404.063369ms         | 445264                   | 1437            |
| CasualX obfbytes!  | 0.10µs               | 10.360403ms (1%)                         | 202.252557ms (18%)   | 1.280738746s (217%)  | 448304 (1%)              | 7780 (441%)     |
| obfstr2 b1!        | 19.75µs              | 20.419212ms (98%)                        | 353.501118ms (106%)  | 4.838922938s (1098%) | 500664 (12%)             | 77153 (5269%)   |
| obfstr2 b2!        | 317.91µs             | 173.078567ms (1579%)                     | 888.272897ms (417%)  | 4.005122498s (891%)  | 612816 (38%)             | 224527 (15525%) |
| obfstr2 b3!        | 829.70µs             | 435.110996ms (4122%)                     | 2.436355796s (1318%) | 5.346699777s (1223%) | 838360 (88%)             | 630688 (43789%) |

<details>
<summary>Click to view benchmark parameters & reproduction steps</summary>

Data is generated by `tests/perf.rs` and can be reproduced via `cargo test --test perf -- --ignored --nocapture`.
- **Parameters**: `PAYLOAD_LEN = 1024` (all `'a'`), `LOOPS = 512` (decryption + checksum accumulation, return value passed through `black_box` to prevent loop-invariant code motion), `profile = release`, median of 3 runs per case.
- **Deobfuscation / Iter**: `(Current Row Runtime - Baseline Runtime) / 512`, factoring out fixed process startup overhead.
- **Build Time**: Cold build compiles a fresh temporary project from scratch (including dependencies); incremental build rewrites only `main.rs` against pre-warmed shared dependencies.

</details>

---

## Security & Safety

> **Disclaimer**: This project is intended strictly for legitimate and lawful use cases such as commercial software IP protection, anti-reverse engineering research, and firmware security hardening. Use in malware development or unauthorized penetration testing is strictly prohibited.

- **Platform Support**: Pure Rust implementation supporting major desktop OSes and `no_std` bare-metal targets (verified on `x86_64-unknown-none`; Wasm is theoretically supported outside of x86 inline assembly features, though not currently in CI).
- **Unsafe Boundaries**: Expanded code uses `unsafe` strictly in two places: `unwrap_unchecked` where length invariants have already been rigorously proven at compile time (to eliminate redundant runtime branches), and volatile memory overwrites inside container `Drop` implementations.
- **Vulnerability Disclosure**: If you discover a security vulnerability, please report it privately via [GitHub Private Vulnerability Reporting](https://github.com/AlbusPhos/obfstr2/security/advisories/new) or by emailing `owo.zzz@proton.me`. Please **do not** open a public issue.

---

## Contributing & Development Verification

Issues and Pull Requests are welcome! Adding a new obfuscation primitive or storage strategy only requires appending an entry to the static registry in `obfstr2-macros/src/crypto.rs` or `storage.rs` without touching the core orchestration pipeline.

<details>
<summary>Click to expand local test matrix & PR checklist</summary>

**Standard Test & Calibration Suite:**
```bash
# 1. Core functionality & macro correctness (milliseconds)
cargo test --test smoke --test s_fmt
cd obfstr2-macros && cargo test --lib

# 2. Compile-time rejection tests for invalid inputs
cargo test --test compile_fail

# 3. Bare-metal no_std pipeline verification (requires x86_64-unknown-none target installed)
cargo test --test nostd

# 4. Performance benchmark report (requires network access for comparison crates and cargo-expand)
cargo test --test perf -- --ignored --nocapture

# 5. Storage strategy & crypto primitive latency/security calibration (long-running; last two require Python angr environment)
cd obfstr2-macros && cargo test --lib test_storages_latency -- --ignored --nocapture
cd obfstr2-macros && cargo test --lib tests::crypto::test_cryptos_latency -- --ignored --nocapture
ANGR_PYTHONPATH=/path/to/python-with-angr cargo test --manifest-path obfstr2-macros/Cargo.toml --lib tests::crypto::test_cryptos_security -- --ignored --nocapture
ANGR_PYTHONPATH=/path/to/python-with-angr cargo test --test angr -- main --ignored --nocapture
```

**Pre-submission Formatting & Linting:**
```bash
cargo fmt --all -- --check
cd obfstr2-macros && cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cd obfstr2-macros && cargo clippy --all-targets --all-features -- -D warnings
```
</details>

---

## License

This project is licensed under the [MPL-2.0](./LICENSE) License. See [CHANGELOG.md](CHANGELOG.md) for release history and version notes.