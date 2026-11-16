# obfstr2

> **Polymorphic compile-time string/wide-str/wide-cstr/bytes/int/float/cstr/file obfuscation**

[![Crates.io](https://img.shields.io/crates/v/obfstr2?logo=rust)](https://crates.io/crates/obfstr2)
[![Docs.rs](https://img.shields.io/docsrs/obfstr2?logo=docs.rs)](https://docs.rs/obfstr2)
[![CI](https://img.shields.io/github/actions/workflow/status/AlbusPhos/obfstr2/ci.yml?logo=github&label=CI)](https://github.com/AlbusPhos/obfstr2/actions)
[![MSRV](https://img.shields.io/crates/msrv/obfstr2?logo=rust&label=MSRV)](https://crates.io/crates/obfstr2)
[![Downloads](https://img.shields.io/crates/d/obfstr2?color=blue)](https://crates.io/crates/obfstr2)
[![License](https://img.shields.io/crates/l/obfstr2)](https://mozilla.org/MPL/2.0/)

**Language:** English | [简体中文](README.md)

A drop-in, single-macro polymorphic compile-time obfuscation framework for Rust. Literal inputs in, secure expressions out—all encryption and control-flow transformations are performed entirely at compile time. Every build generates vastly different binary signatures and ciphertext layouts, completely defeating static signature scanning, Mandiant `FLOSS`, and dedicated Rust-targeted deobfuscators (such as [`rust-deobf`](https://github.com/VAHPEM/rust-deobf)).

Provides comprehensive coverage for **strings, C-strings, UTF-16 wide strings, byte arrays, external files, integers, floats, and format strings**. Values can be dereferenced directly into raw types or stored in managed containers; all string and byte containers automatically perform volatile memory zeroization upon leaving scope (`Drop`), preventing sensitive plaintexts from lingering on the stack or heap.

## Table of Contents

- [Quick Start](#quick-start)
- [Key Highlights & Adversarial Verification](#key-highlights--adversarial-verification)
- [Design Philosophy & Scope](#design-philosophy--scope)
- [Macro Overview](#macro-overview)
- [Feature Flags](#feature-flags)
- [Performance Benchmarks](#performance-benchmarks)
- [Security & Safety](#security--safety)
- [Contributing & Development Verification](#contributing--development-verification)
- [License](#license)

## Quick Start

```toml
[dependencies]
obfstr2 = "0.4"
```

All obfuscation macros use a numeric suffix to denote their security tier: `1` (Low Latency), `2` (Balanced / Recommended), and `3` (High-Intensity Anti-RE);
additionally, the [`*_user!`](#custom-tier-_user) custom tier allows overriding tier parameters field-by-field on top of the balanced tier via a configuration block.

```rust
use obfstr2::{b2, cs2, cw2, f2, fl2, i2, s2, s_fmt2, w1};

fn main() {
    // String: returns an owned secure container that can be bound, passed, and reused; auto-zeroed on Drop
    let hello = s2!("hello");
    print!("{hello}");

    // C-string: dereferences to &CStr; as_ptr() can be passed directly to low-level syscalls
    let sh = cs2!(c"/bin/sh");
    assert_eq!(&*sh, c"/bin/sh");

    // Wide C-string: UTF-16 code units + automatic trailing 0u16; as_ptr() is ready for Win32 PCWSTR
    let w = cw2!("kernel32.dll");
    assert_eq!(w[w.len() - 1], 0);

    // Wide string: raw UTF-16 code units without a trailing NUL; interior NULs are permitted
    let ws = w1!("a\0b");
    assert_eq!(&*ws, &[0x61u16, 0, 0x62][..]);

    // Byte string & byte array: dereferences to &[u8]
    let b = b2!(b"abc");
    let c = b2!([0x61, 98, 99]);
    assert_eq!(&*b, &*c);

    // Scalar integers & floats: evaluates directly to raw native primitive values ready for arithmetic
    let x = i2!(42u32);
    assert_eq!(x + 1, 43u32);
    let y = fl2!(3.15);
    assert_eq!(y.to_bits(), 3.15f64.to_bits());

    // External file: path resolved relative to the current crate's CARGO_MANIFEST_DIR, read and obfuscated at compile time
    let asset = f2!("assets/fixture.bin");

    // Format string: static literal segments are individually obfuscated before passing to format!, returning a String
    let name = "world";
    let greeting = s_fmt2!("hello, {}!", name);
    print!("{greeting}");
}
```

For more practical examples, see [`examples/showcase.rs`](examples/showcase.rs) (run via `cargo run --example showcase`).

---

## Key Highlights & Adversarial Verification

`obfstr2` is far more than a simple "compile-time XOR" macro—it is a polymorphic cryptographic obfuscation pipeline purpose-built to counter **specialized deobfuscators, white-box reverse engineering, decompilers (IDA / Ghidra F5), and symbolic execution engines (angr / Z3)**.

### Real-World Adversarial Verification: Complete Immunity to Dedicated Deobfuscators (`rust-deobf`)

While traditional compile-time obfuscation crates can bypass `strings` and generic analysis tools like `FLOSS`, they fail completely against [**`rust-deobf`**](https://github.com/VAHPEM/rust-deobf)—a specialized deobfuscator tailored to Rust binary signatures and control-flow patterns (as legacy crates rely on fixed inline XOR patterns, static key layouts, or recognizable standard AEAD algorithms).

Powered by a polymorphic primitive pipeline, cross-chunk ghost state coupling, and call-graph shattering, when tested against [`examples/showcase.rs`](examples/showcase.rs) across diverse protected payloads, **zero strings protected by `obfstr2` macros were recovered (0 recovered)**:

| Obfuscation Scheme / Tool             |   `strings`    |    `FLOSS`     | [`rust-deobf`](https://github.com/VAHPEM/rust-deobf) | Anti-Analysis & Deobfuscation Assessment                                                        |
|:--------------------------------------|:--------------:|:--------------:|:----------------------------------------------------:|:------------------------------------------------------------------------------------------------|
| `litcrypt` / `litcrypt2`              |     0 / 5      |     0 / 5      |               **5 / 5 (Compromised)**                | ❌ Obvious static byte-XOR signatures                                                           |
| `obfstr` (Inline XOR+ADD)             |     0 / 5      |     0 / 5      |               **5 / 5 (Compromised)**                | ❌ Fixed inline expansion patterns; easily recovered via instruction emulation                  |
| `cryptify` (Default Key)              |     0 / 5      |     0 / 5      |               **5 / 5 (Compromised)**                | ❌ Fixed decryption routines and static key layouts                                             |
| `muddy` (ChaCha20-Poly1305)           |     0 / 5      |     0 / 5      |               **5 / 5 (Compromised)**                | ❌ Standard cryptographic constants and AEAD call structures are accurately fingerprinted       |
| Handwritten Stack Strings / XOR / ADD |     0 / 5      |     0 / 5      |               **5 / 5 (Compromised)**                | ❌ Lacks control-flow barriers and cross-chunk state coupling                                   |
| **`obfstr2` (`showcase` benchmark)**  | **0 (0% rec)** | **0 (0% rec)** |                 **0 (100% Immune)**                  | ✅ **Non-standard polymorphic primitives + cross-chunk avalanche ratchet + vtable trampolines** |

*Note: The first 6 baseline entries above are cited directly from the [official `rust-deobf` evaluation benchmark](https://github.com/VAHPEM/rust-deobf).*

> **Commitment to Continuous Adversarial Evolution:**
> Reverse and anti-reverse engineering are a dynamic cat-and-mouse game. Thanks to a highly decoupled underlying registry architecture, **the maintainers commit to periodically updating and rotating the built-in cryptographic primitives and steganographic pseudo-signatures**, continuously refreshing code expansion shapes and data patterns so that the latest version of `obfstr2` maintains dynamic polymorphic security and renders targeted extraction rules obsolete.

### 1. Cryptographic Primitive Library Resistant to Symbolic Execution & Decompilation
Features 17+ built-in polymorphic encryption/decryption primitives spanning stream ciphers, Substitution-Permutation Networks (SPN), dynamic box transforms, and block ciphers, quantitatively graded by runtime latency and security score:
- **Proprietary MSE / MSE2 Algorithm Family**: Combines CFB ciphertext feedback, dynamic S-Boxes / P-Boxes that mutate byte-by-byte based on historical state, 128-bit non-linear key derivation based on fixed-point approximations of transcendental numbers ( $\pi$ , $1/e$ ), and 64B-block 4-round ARX diffusion. This completely eliminates the possibility of attackers identifying standard cryptographic magic constants (e.g., AES / ChaCha20 constants) and invoking off-the-shelf libraries for bulk decryption.
- **Cross-Domain Floating-Point Algebraic Obfuscation**: Equivalently expands basic integer bitwise operations (rotations, XORs) into IEEE-754 floating-point divisions, truncations, and exponent constructions. While drastically inflating cognitive load in decompiler (F5) pseudocode, this directly triggers path explosion and solver timeouts in SMT solvers due to non-linear floating-point constraints.
- **Dynamic Box Inversion & Modular Multiplicative Inverse**: Encrypts at compile time using $\pmod{2^8}$ odd multiplicative inverses and decrypts at runtime with a single multiplication; dynamically shuffles and inverts S-Boxes / P-Boxes on the fly from keys at runtime, completely eradicating static lookup table signatures from the binary.
- **Global Positional Diffusion**: Introduces Fisher-Yates permutation driven by 128-bit unbiased multiplicative interval mapping to break byte-boundary alignment.

### 2. Reverse Key Ratchet & Full-Chain Avalanche Coupling
Refuses to expose independent per-layer keys in the binary, constructing a bidirectional state-coupling chain spanning "compile-time forward encryption" and "runtime reverse decryption":
- **Zero Extra Key Storage ( $\pmod{2^{64}}$ Multiplicative Inverse Ratchet)**: Each chunk stores only a single root key. At compile time, outer-layer keys are backward-derived using 64-bit odd multiplicative inverses combined with the trailing byte of the current layer; at runtime, the next layer's key is forward-recovered via the trailing byte:
  $$K _ {\text{next}} = (K _ {\text{cur}} + \text{ghost\\_state}) \times (\text{last\\_byte} \mid 1) \pmod{2^{64}}$$
- **Cross-Chunk Ghost State (`ghost_state`)**: After each chunk is decrypted and appended to the primary container, the trailing byte of the primary container is extracted to cumulatively update the global state. Any attempt to skip preceding chunks via a debugger, patch a single byte in memory, or statically rip out an individual sub-function for emulated execution will trigger a total avalanche across all subsequent chunk and layer keys.

### 3. Superlinear Chunking, High-Fidelity Junk Chunks & Number-Theoretic Opaque Predicates
- **Superlinear Chunking & Short-Block Padding**: Randomly slices payloads using an increasing step size of $\lceil i^{1.4} \rceil$ (fine-grained fragmentation at the head to scatter signatures, smooth growth at the tail to bound total chunk count); medium- and high-intensity tiers enforce prepended PKCS#7 padding on short chunks ( $<16\text{B}$ ) to eliminate length side-channels on short strings.
- **100% High-Fidelity Junk Chunks**: Probabilistically interleaves forged decoy chunks according to the selected tier. Decoy chunks undergo the exact same multi-layer encryption, ciphertext storage, and runtime decryption pipeline as real chunks, making them indistinguishable in decompiler views.
- **Number-Theoretic Opaque Predicates**: Injects integrity checks at the end of chunk decryption based on number-theoretic identities such as modulo-4 quadratic residues (e.g., $x^2 \not\equiv 2 \pmod 4$ holds unconditionally). Shielded against compile-time constant folding via `black_box`, decoy chunks deterministically trigger self-erasure at runtime, keeping the primary container and global state intact.

### 4. Call-Graph Shattering & LLVM Optimization Barriers
Purpose-built defenses against static cross-reference (Xrefs) tracking, batch function hooking, and aggressive compiler optimizations:
- **Adaptive Outlining & Vtable Trampolines**: As consecutive inlined layers accumulate, the probability of outlining into standalone functions increases exponentially. Outlined routines randomly emit (50/50 split) either a named `#[inline(never)]` function or a dynamically dispatched trampoline invoked indirectly through a `&mut dyn FnMut` fat pointer—converting direct calls into virtual table indirect calls and severing the static call graph.
- **O(log M) Bottom-Up Multi-Way Folding Tree**: When chunk counts are large, adjacent groups of 4–12 chunk nodes are recursively encapsulated into a multi-level call tree, preventing the root function from growing linearly with payload size.
- **Cold/Hot Section Splitting & Anti-Interprocedural Optimization (Anti-IPO)**: Probabilistically attaches the `#[cold]` attribute to evict decryption logic into the `.text.unlikely` section, disrupting spatial locality; casts function pointers to `usize` and feeds them into `black_box` to fake address escape, preventing LLVM from performing Interprocedural Constant Propagation (IPCP) and Dead Argument Elimination (DAE).

### 5. Polymorphic Steganographic Storage & Controlled Memory Lifecycle
- **Polymorphic Ciphertext & Key Camouflage**: Ciphertexts and per-chunk 8-byte initial keys are encoded via randomly selected storage strategies, disguised as raw byte streams, `u64`/`u128` integer arrays, MAC addresses, UUIDs, IPv6 addresses, PEM certificates, or HB2 whitespace steganography to evade high-entropy section scans. Notably, HB2 steganography embeds a realistic text corpus (code snippets, logs, scripts, etc.) at compile time with randomly generated whitespace symbol tables, ensuring neither the host text nor symbol patterns can be fingerprinted.
- **Full-Chain Volatile Erasure**: Not only does the final returned container perform a full-capacity volatile zeroing on `Drop`, but intermediate temporary containers across variable-length decryption layers are also immediately destroyed and scrubbed upon handoff, leaving zero plaintext or intermediate state behind in memory.
- **Fine-Grained Stack Budget (`STACK_BUDGET`)**: Supports capping pure-stack overhead via `budget-1k` through `budget-8m` feature flags. Peak expansion volume across multi-layer encryption/decryption is tracked at compile time to intelligently route between stack and heap containers, seamlessly adapting to `no_std` bare-metal and embedded firmware targets.

### 6. Blazing-Fast Compilation for Multi-MB Payloads (No `long_running_const_eval` Needed)
When handling multi-megabyte binary firmware or resource assets, expanding a massive payload into a single monolithic function causes the LLVM backend to hit $O(V^2)$ complexity explosion during register allocation. `obfstr2` evaluates natively inside procedural macros (requiring no `#![allow(long_running_const_eval)]` attributes), leveraging superlinear chunking, exponential inline backpressure, and an **O(log M) multi-way folding tree** to strictly bound child nodes per function body to a small constant ( $\le 12$ ). This enables the LLVM backend to compile multi-MB payloads in near-linear time.

### 7. Anti-Disassembly Rogue Instruction Injection
Targeting linear sweep and recursive traversal instruction recognizers in disassemblers, architecture-gated inline assembly sequences (rogue/junk instructions) with zero data dependency on real state are probabilistically injected at the prologues and epilogues of non-inlined wrapper functions, inlined decryption fragments, multi-level folding aggregation functions, and chunk scopes—deeply interleaving junk bytes with real decryption instructions in disassembly views.

---

## Design Philosophy & Scope

### Core Principles
1. **White-Box Threat Model**: Traditional cryptography is unbreakable in a black-box setting, but under a white-box model where the adversary has total control over the binary, once standard algorithm constants (such as ChaCha20 magic numbers) are identified, extracting the Key/IV allows instant recovery via scripts or deobfuscators. `obfstr2` optimizes for **maximizing reverse-engineering cognitive resistance and automated script recovery time**, forcing attackers to manually untangle non-standard polymorphic logic instance by instance.
2. **Complementary Probabilistic Security Model**: Encryption layer selection follows no fixed template; instead, primitives are randomly sampled and stacked from the pool within the tier's latency ceiling until the cumulative security score reaches the target threshold (`min_security`):
   $$\text{Security} _ {\text{combined}} = 1 - \prod _ {i=1}^{k} \left(1 - \frac{\text{Security} _ i}{100}\right) \ge \text{min\\_security}$$
3. **Decoupled Registry & Periodic Rotation**: Cryptographic primitives and steganographic storage strategies are isolated into standalone static registries, while the orchestrator solely manages the "Chunk → Encrypt → Store → Fold & Emit" pipeline. Backed by this pluggable architecture, the maintainers commit to regularly rotating and expanding underlying algorithms and pseudo-signatures to continuously counter evolving reverse-engineering signatures.
4. **Compile-Time Evaluation & Zero Redundant Checks**: The procedural macro completes all encryption, state simulation, and length/alignment invariant verification at compile time; runtime execution uses `unwrap_unchecked` directly to eliminate redundant branch-checking overhead.

### Trade-offs

| We Chose                            | Over                        | Rationale                                                                                                                                                     |
|:------------------------------------|:----------------------------|:--------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Anti-RE Strength First**          | Minimal Binary Size         | Cascaded decryption, high-fidelity decoy chunks, and polymorphic expansion inevitably increase code size—a necessary cost to defeat decompilers and emulators |
| **Non-Deterministic Polymorphism**  | Reproducible Builds         | Ensuring every build produces completely distinct binary signatures is the core defense against automated signature extraction and batch scripting            |
| **White-Box Engineering Hardening** | Theoretical Eternal Secrecy | The goal is to drastically inflate the attacker's time, human effort, and compute costs during static analysis, symbolic execution, and dynamic debugging     |

### Recommended Use Cases vs. Non-Goals

| ✅ Recommended Use Cases                                                                                                                                                                                                                                                                                                                                                                                                | ❌ Inapplicable Scenarios / Non-Goals                                                                                                                                                                                                                                                                                                                                                                   |
|:------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| • Protecting commercial software DRM, license verification constants, embedded public keys / API keys<br>• Hiding sensitive syscall / module names (e.g., `ntdll.dll`) in game anti-cheat and security SDKs<br>• Hiding hardcoded credentials and protocol magic numbers in `no_std` embedded firmware and bare-metal binaries<br>• Embedding and obfuscating multi-MB binary assets while maintaining fast build times | • Release pipelines requiring deterministic, bit-for-bit reproducible binary hashes<br>• Compliance auditing scenarios requiring human-readable plaintext in binaries<br>• **Does not replace** standard cryptographic libraries for network transit or persistent storage encryption<br>• **Does not** apply control-flow flattening to host business logic, nor bundle built-in anti-debugging checks |

---

## Macro Overview

All macros are provided in three preset tiers: `1` (Low Latency), `2` (Balanced), and `3` (High Intensity):

| Macro Family          | Example Literal Input            | Return Type (`Deref` Target)                                 | Features & Constraints                                                                                                                                                                                    |
|:----------------------|:---------------------------------|:-------------------------------------------------------------|:----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `s1!` ~ `s3!`         | `"hello"`                        | `StackStr<N>` / `HeapStr<N>`<br>(`Deref<Target = str>`)      | Automatically zeroed on `Drop`.                                                                                                                                                                           |
| `b1!` ~ `b3!`         | `b"abc"` or `[0x41, 66]`         | `StackBytes<N>` / `HeapBytes<N>`<br>(`Deref<Target = [u8]>`) | Array elements must be in `0..=255`; automatically zeroed on `Drop`.                                                                                                                                      |
| `cs1!` ~ `cs3!`       | `c"..."` / `"..."` / `b"..."`    | `StackCStr<N>` / `HeapCStr<N>`<br>(`Deref<Target = CStr>`)   | Automatically appends trailing `\0` before obfuscation; zero-copy recovery at runtime; use `b"..."` for non-UTF-8; **interior NULs are strictly forbidden** (prevents truncation); auto-zeroed on `Drop`. |
| `w1!` ~ `w3!`         | `"a\0b"`                         | `StackU16<N>` / `HeapU16<N>`<br>(`Deref<Target = [u16]>`)    | Encoded into UTF-16 code units at compile time; **no** trailing NUL appended, **interior NULs permitted**; ideal for length-prefixed APIs; auto-zeroed on `Drop`.                                         |
| `cw1!` ~ `cw3!`       | `"kernel32.dll"`                 | `StackU16<N>` / `HeapU16<N>`<br>(`Deref<Target = [u16]>`)    | Encoded into UTF-16 at compile time with an **automatic trailing `0u16`**; **interior NULs strictly forbidden**; `as_ptr()` passes directly to Win32 `PCWSTR`; auto-zeroed on `Drop`.                     |
| `f1!` ~ `f3!`         | `"assets/data.bin"`              | `StackBytes<N>` / `HeapBytes<N>`<br>(`Deref<Target = [u8]>`) | Path resolved relative to the compiled crate's `CARGO_MANIFEST_DIR`; automatically zeroed on `Drop`.                                                                                                      |
| `i1!` ~ `i3!`         | `42u32` / `-1` / `0xFFu16`       | Native integer primitive (`u8`~`i128` / `usize`)             | Unabbreviated suffix defaults to `i32`; `usize`/`isize` encoded with 64-bit semantics; no `Drop` zeroing.                                                                                                 |
| `fl1!` ~ `fl3!`       | `3.15f32` / `-1.0` / `1e10`      | Native float primitive (`f32` / `f64`)                       | Unabbreviated suffix defaults to `f64`; rejects `inf`/`NaN`; preserves `-0.0` sign bit (use `to_bits()` for assertions); no `Drop` zeroing.                                                               |
| `s_fmt1!` ~ `s_fmt3!` | `"hi, {}!", name`                | `alloc::string::String`                                      | Obfuscates only static literal segments within the format string, then invokes `format!`; requires the `alloc` feature.                                                                                   |
| `*_user!` (All)       | `{ max_latency: 50, ... }, load` | Same as corresponding preset macro                           | Custom tier (experimental): first argument is a config block overriding balanced tier parameters field-by-field (see [below](#custom-tier-_user)); `s_fmt_user!` requires `alloc`.                        |

> **Usage Tip (Stack/Heap Routing & Type Inference):**
> When `alloc` is enabled (default), payloads $\le 16\text{B}$ always use stack containers, payloads reaching `STACK_BUDGET` always use heap containers, and payloads in between are probabilistically routed per tier. Consequently, **the concrete container type returned (`Stack*` or `Heap*`) may change dynamically across compilations**. Always rely on type inference (`let val = s2!("...");`) and avoid hardcoding concrete container type annotations.

### Custom Tier `*_user!`

Every macro family provides a unified two-argument **`({ config }, payload)`** form: the first argument is a braced configuration block `{ key: val, ... }`, and the second argument matches the payload of the corresponding tiered macro. The configuration overrides parameters field-by-field **using the balanced tier (`2`) as the baseline**:

```rust
use obfstr2::{cs_user, s_user};
fn main() {

    let greeting = s_user!(
        {
            prefer_stack: true,      // Stack preference in heap/stack coin flip
            max_latency: 60,         // Per-layer latency ceiling (>= 2)
            min_security: 75,        // Combined security floor (<= 99, multiplicative accumulation)
            junk_pct: 12,            // Decoy junk chunk injection probability (0..=100)
            pad_short: true,         // Force PKCS#7 pre-padding on short chunks (< 16B)
        },
        "custom-tier",
    );
    assert_eq!(&*greeting, "custom-tier");

    // An empty config block {} is equivalent to the balanced tier s2!
    assert_eq!(&*s_user!({}, "hello"), "hello");
    // Static literal segments in s_fmt_user! are obfuscated using the custom tier
    assert_eq!(cs_user!({ max_latency: 40 }, c"/bin/sh").to_str().ok(), Some("/bin/sh"));
}
```

| Config Key                  | Value                 | Description                                                                                                                 |
|:----------------------------|:----------------------|:----------------------------------------------------------------------------------------------------------------------------|
| `prefer_stack`              | `bool`                | Stack preference probability during heap/stack routing coin flips                                                           |
| `max_latency`               | `u8` (≥ 2)            | Maximum allowed latency per encryption layer                                                                                |
| `min_security`              | `u8` (≤ 99)           | Minimum combined security target (accumulated multiplicatively; stops once reached)                                         |
| `security_factor`           | `f64` (> 0.0, finite) | Per-layer security scaling factor: `< 1.0` discounts per-layer contribution (more layers), `> 1.0` amplifies (fewer layers) |
| `wrap_pct`                  | `0..=100`             | Per-layer probability of outlining into a non-inlined wrapper function                                                      |
| `junk_pct`                  | `0..=100`             | Probability of injecting high-fidelity decoy junk chunks                                                                    |
| `cold_pct`                  | `0..=100`             | Probability of marking outlined decryption functions with `#[cold]`                                                         |
| `pad_short`                 | `bool`                | Enforce prepended PKCS#7 padding on short chunks (< 16B)                                                                    |
| `anti_disasm_pct`           | `0..=100`             | Anti-disassembly rogue instruction injection probability (requires `anti-disassembly` feature)                              |
| `anti_disasm_count_range`   | `lo..=hi` (`u8`)      | Random range for the number of anti-disassembly blocks                                                                      |
| `anti_disasm_garbage_range` | `lo..=hi` (`u8`)      | Random range for the baseline number of garbage bytes per anti-disassembly block                                            |

- Keys support both `key: val` and `key = val` syntax, separated by commas; trailing commas are permitted both inside the config block and after the payload.

---

## Feature Flags

| Feature                   | Default | Description                                                                                                                                                              |
|:--------------------------|:-------:|:-------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `default`                 |   ✅    | Enables `alloc` and `anti-disassembly` by default.                                                                                                                       |
| `alloc`                   |   ✅    | Enables heap container (`Heap*`) support and `s_fmt*!` macros. Compatible with `no_std + alloc` environments.                                                            |
| `anti-disassembly`        |   ✅    | Enables x86 / x86_64 anti-disassembly rogue inline assembly injection with Red-Zone safety protection. Can be disabled in size-critical scenarios.                       |
| `budget-1k` ~ `budget-8m` |   ❌    | Stack budget ceiling control: choose from `1k`, `4k` (default), `16k`, `64k`, `256k`, `512k`, `1m`, `4m`, `8m`. When multiple are enabled, the largest takes precedence. |

**Pure-Stack Bare-Metal Mode (no `alloc`, no rogue instruction injection):**
```toml
[dependencies]
obfstr2 = { version = "0.4", default-features = false }
```
*Note: When `alloc` is disabled, all tiers unconditionally use stack containers (`Stack*`); if the peak decryption volume of a single chunk exceeds `STACK_BUDGET / 2`, a compile-time error will prompt you to enable a larger `budget-*` feature.*

---

## Performance Benchmarks

For small payloads (such as the 1 KiB sample in the table below), `b1!` incurs near-zero runtime overhead; `b2!` and `b3!` trade increased runtime and compile-time costs for substantially stronger anti-analysis protection. Conversely, when handling **multi-MB large payloads**, thanks to native procedural macro evaluation and the multi-way folding tree architecture, `obfstr2` compiles significantly faster than traditional crates reliant on `const` evaluation. Choose the appropriate tier based on your workload's balance between runtime performance and anti-reverse-engineering strength.

Below is a single-run benchmark sample for a 1 KiB payload decrypted over 512 iterations (polymorphic builds fluctuate slightly across runs; intended for order-of-magnitude reference):

| Case               | Per-Deobfuscation Time | Total Runtime (512 loops, incl. startup) | Incremental Build Time | Cold Build Time      | Binary Size (unstripped) | Expanded Chars  |
|:-------------------|:-----------------------|:-----------------------------------------|:-----------------------|:---------------------|:-------------------------|:----------------|
| Plaintext Baseline | —                      | 10.30702ms                               | 171.832628ms           | 404.063369ms         | 445264                   | 1437            |
| CasualX obfbytes!  | 0.10µs                 | 10.360403ms (1%)                         | 202.252557ms (18%)     | 1.280738746s (217%)  | 448304 (1%)              | 7780 (441%)     |
| obfstr2 b1!        | 19.75µs                | 20.419212ms (98%)                        | 353.501118ms (106%)    | 4.838922938s (1098%) | 500664 (12%)             | 77153 (5269%)   |
| obfstr2 b2!        | 317.91µs               | 173.078567ms (1579%)                     | 888.272897ms (417%)    | 4.005122498s (891%)  | 612816 (38%)             | 224527 (15525%) |
| obfstr2 b3!        | 829.70µs               | 435.110996ms (4122%)                     | 2.436355796s (1318%)   | 5.346699777s (1223%) | 838360 (88%)             | 630688 (43789%) |

<details>
<summary>Click to view benchmark parameters & reproduction instructions</summary>

Data is generated by `tests/perf.rs` and can be reproduced via `cargo test --test perf -- --ignored --nocapture`.
- **Benchmark Parameters**: `PAYLOAD_LEN = 1024` (all `'a'`), `LOOPS = 512` (decryption + checksum accumulation, return value passed through `black_box` to prevent loop hoisting), `profile = release`, median of 3 runs per group.
- **Per-Deobfuscation Time**: `(current_row_runtime - baseline_runtime) / 512`, canceling out fixed process startup overhead.
- **Build Time**: Cold build compiles a fresh temporary Cargo project from scratch (including dependencies); incremental build rewrites only `main.rs` against pre-warmed shared dependencies.

</details>

---

## Security & Safety

> **Disclaimer**: This project is intended strictly for legitimate and compliant use cases such as commercial software intellectual property protection, anti-reverse-engineering research, and firmware security hardening. Any use in malware development or unauthorized penetration testing is strictly prohibited.

- **Platform Support**: Pure Rust implementation supporting mainstream desktop platforms and `no_std` bare-metal targets (verified on `x86_64-unknown-none`; Wasm is theoretically supported but not currently included in CI).
- **Unsafe Boundaries**: Generated code uses `unsafe` exclusively for `unwrap_unchecked` where length invariants have already been strictly verified at compile time (eliminating redundant runtime branches), and for volatile memory overwrites inside container `Drop` implementations.
- **Vulnerability Disclosure**: If you discover a security vulnerability, please report it privately via [GitHub Private Vulnerability Reporting](https://github.com/AlbusPhos/obfstr2/security/advisories/new) or email `owo.zzz@proton.me`. **Do not** open a public issue.

---

## Contributing & Development Verification

Issues and Pull Requests are welcome! Adding a new obfuscation primitive or storage strategy simply requires appending an entry to the static registry in `obfstr2-macros/src/crypto.rs` or `storage.rs`, without modifying the core orchestration pipeline.

<details>
<summary>Click to expand local test matrix & PR checklist</summary>

**Standard Test & Calibration Suite:**
```bash
# 1. Core functionality & macro correctness (millisecond-level)
cargo test --test smoke --test s_fmt
cd obfstr2-macros && cargo test --lib

# 2. Compile-time rejection tests for invalid inputs
cargo test --test compile_fail

# 3. Bare-metal no_std pipeline verification (requires x86_64-unknown-none target installed)
cargo test --test nostd

# 4. Performance benchmark report (requires network access to fetch baseline crates & cargo-expand installed)
cargo test --test perf -- --ignored --nocapture

# 5. Storage strategy & cryptographic primitive latency/security calibration (long-running; last two require a Python angr environment)
cd obfstr2-macros && cargo test --lib test_storages_latency -- --ignored --nocapture
cd obfstr2-macros && cargo test --lib tests::crypto::test_cryptos_latency -- --ignored --nocapture
ANGR_PYTHONPATH=/path/to/python-with-angr cargo cd obfstr2-macros && cargo test --lib tests::crypto::test_cryptos_security -- --ignored --nocapture
ANGR_PYTHONPATH=/path/to/python-with-angr cargo test --test angr -- main --ignored --nocapture
```

**Pre-Submission Formatting & Lint Checks:**
```bash
cargo fmt --all -- --check
cd obfstr2-macros && cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cd obfstr2-macros && cargo clippy --all-targets --all-features -- -D warnings
```
</details>

---

## License

This project is licensed under the [MPL-2.0](./LICENSE) License. See [CHANGELOG.md](CHANGELOG.md) for version history and release notes.