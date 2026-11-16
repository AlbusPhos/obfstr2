<!-- i18n-sync-anchor:(sha256:8bbf3ec730d598ce3ca26de6310cddb8ec11d90d614c230c4892cea60de9cb80 reviewed:true source:README.md) -->
# obfstr2

> **Polymorphic compile-time string/wide-str/wide-cstr/bytes/int/float/cstr/file obfuscation**

[![Crates.io](https://img.shields.io/crates/v/obfstr2?logo=rust)](https://crates.io/crates/obfstr2)
[![Docs.rs](https://img.shields.io/docsrs/obfstr2?logo=docs.rs)](https://docs.rs/obfstr2)
[![CI](https://img.shields.io/github/actions/workflow/status/AlbusPhos/obfstr2/ci.yml?logo=github&label=CI)](https://github.com/AlbusPhos/obfstr2/actions)
[![MSRV](https://img.shields.io/crates/msrv/obfstr2?logo=rust&label=MSRV)](https://crates.io/crates/obfstr2)
[![Downloads](https://img.shields.io/crates/d/obfstr2?color=blue)](https://crates.io/crates/obfstr2)
[![License](https://img.shields.io/crates/l/obfstr2)](https://mozilla.org/MPL/2.0/)

**Language:** English | [简体中文](README.md)

A drop-in, single-macro polymorphic compile-time obfuscation framework for Rust. Literal in, secure expression out—all encryption and control-flow transformations occur entirely at compile time. Every build generates distinct binary signatures and ciphertext morphologies, thoroughly defeating static signature scanners, Mandiant `FLOSS`, and Rust-specific deobfuscators such as [`rust-deobf`](https://github.com/VAHPEM/rust-deobf).

Comprehensive support for **native strings, C strings, UTF-16 wide strings, byte arrays, external files, integers, floats, and formatted strings**. Dereference directly to raw values or bind to managed containers; all string and byte containers automatically perform volatile memory zeroization on `Drop`, preventing sensitive plaintext from lingering on the stack or heap.

## Table of Contents

- [Quick Start](#quick-start)
- [Core Highlights & Adversarial Verification](#core-highlights--adversarial-verification)
- [Design Philosophy & Boundaries](#design-philosophy--boundaries)
- [Macro Overview](#macro-overview)
- [Feature Flags](#feature-flags)
- [Benchmarks](#benchmarks)
- [Security](#security)
- [Contributing & Verification](#contributing--verification)
- [License](#license)

## Quick Start

```toml
[dependencies]
obfstr2 = "0.4"
```

All obfuscation macros use a numeric suffix to denote security tiers: `1` (low latency), `2` (balanced / recommended), and `3` (maximum anti-reversing strength).

```rust
use obfstr2::{b2, cs2, cw2, f2, fl2, i2, s2, s_fmt2, w1};

fn main() {
    // String: returns an owned secure container; bindable, passable, and zeroized on Drop
    let hello = s2!("hello");
    print!("{hello}");

    // C string: dereferences to &CStr; as_ptr() can be passed directly to FFI syscalls
    let sh = cs2!(c"/bin/sh");
    assert_eq!(&*sh, c"/bin/sh");

    // Wide C string: UTF-16 code units + trailing 0u16; as_ptr() is ready for Win32 PCWSTR
    let w = cw2!("kernel32.dll");
    assert_eq!(w[w.len() - 1], 0);

    // Wide string: raw UTF-16 code units; no trailing NUL appended, internal NULs allowed
    let ws = w1!("a\0b");
    assert_eq!(&*ws, &[0x61u16, 0, 0x62][..]);

    // Byte string & byte array: dereferences to &[u8]
    let b = b2!(b"abc");
    let c = b2!([0x61, 98, 99]);
    assert_eq!(&*b, &*c);

    // Scalar integers & floats: evaluates directly to raw primitive values
    let x = i2!(42u32);
    assert_eq!(x + 1, 43u32);
    let y = fl2!(3.15);
    assert_eq!(y.to_bits(), 3.15f64.to_bits());

    // External file: path relative to the crate's CARGO_MANIFEST_DIR, embedded at compile time
    let asset = f2!("assets/fixture.bin");

    // Formatted string: obfuscates static literal segments individually before invoking format!
    let name = "world";
    let greeting = s_fmt2!("hello, {}!", name);
    print!("{greeting}");
}
```

For a comprehensive real-world example, see [`examples/showcase.rs`](examples/showcase.rs) (run via `cargo run --example showcase`).

---

## Core Highlights & Adversarial Verification

`obfstr2` is far more than a simple "compile-time XOR" macro. It is a polymorphic cryptographic obfuscation pipeline purpose-built to counter **specialized deobfuscators, white-box reverse engineering, decompilers (IDA / Ghidra F5), and symbolic execution engines (angr / Z3)**.

### Adversarial Verification: Complete Immunity to `rust-deobf`

While traditional compile-time obfuscation crates bypass `strings` and general-purpose tools like `FLOSS`, they completely fail against [**`rust-deobf`**](https://github.com/VAHPEM/rust-deobf)—a specialized deobfuscator tailored to Rust binary idioms and control-flow patterns (due to their reliance on fixed inline XOR patterns, static key layouts, or standard AEAD constants).

Powered by a polymorphic primitive pipeline, cross-chunk ghost state coupling, and call-graph shattering, **`obfstr2` achieved 0 extracted strings** when tested against [`examples/showcase.rs`](examples/showcase.rs):

| Obfuscator / Technique               | `strings` | `FLOSS` | [`rust-deobf`](https://github.com/VAHPEM/rust-deobf) | Anti-Analysis & Deobfuscation Assessment                                            |
|:-------------------------------------|:---------:|:-------:|:----------------------------------------------------:|:------------------------------------------------------------------------------------|
| `litcrypt` / `litcrypt2`             |   0 / 5   |  0 / 5  |                  **5 / 5 (Broken)**                  | ❌ Obvious static byte-XOR signatures                                               |
| `obfstr` (inlined XOR+ADD)           |   0 / 5   |  0 / 5  |                  **5 / 5 (Broken)**                  | ❌ Fixed inline expansion patterns; easily recovered via emulation                  |
| `cryptify` (default key)             |   0 / 5   |  0 / 5  |                  **5 / 5 (Broken)**                  | ❌ Predictable decryption routine and key layout                                    |
| `muddy` (ChaCha20-Poly1305)          |   0 / 5   |  0 / 5  |                  **5 / 5 (Broken)**                  | ❌ Standard cryptographic constants and AEAD structures are easily located          |
| Tight stack strings / XOR / ADD      |   0 / 5   |  0 / 5  |                  **5 / 5 (Broken)**                  | ❌ Lacks control-flow barriers and cross-chunk state coupling                       |
| **`obfstr2` (`showcase` benchmark)** |   **0**   |  **0**  |                 **0 (100% Immune)**                  | ✅ **Non-standard polymorphic primitives + avalanche ratchet + vtable trampolines** |

*Note: The first 6 baseline entries are cited directly from the [official `rust-deobf` evaluation benchmark](https://github.com/VAHPEM/rust-deobf).*

> **Commitment to Continuous Evolution:**
> Reverse and anti-reverse engineering is an ongoing cat-and-mouse game. Backed by our highly decoupled registry architecture, **the maintainers commit to periodically rotating and expanding built-in cryptographic primitives and steganographic pseudo-signatures**. This continuously refreshes code emission patterns and data signatures across releases, ensuring targeted extraction heuristics rapidly become obsolete.

### 1. Anti-Symbolic-Execution & Anti-Decompiler Cryptographic Primitives
Includes 17+ polymorphic encryption/decryption primitives spanning stream ciphers, Substitution-Permutation Networks (SPN), dynamic box transformations, and block ciphers—quantitatively graded by runtime latency and security score:
- **Proprietary MSE / MSE2 Algorithm Family**: Combines CFB ciphertext feedback, dynamic S-Boxes / P-Boxes that mutate byte-by-byte based on historical state, 128-bit non-linear key derivation based on fixed-point approximations of transcendental numbers ( $\pi$ , $1/e$ ), and 64-byte block 4-round ARX diffusion. This completely eliminates magic constants (such as AES or ChaCha20 signatures) that attackers rely on for off-the-shelf decryption scripts.
- **Cross-Domain Floating-Point Algebraic Obfuscation**: Expands bitwise integer operations (rotations, XORs) into mathematically equivalent IEEE-754 floating-point divisions, truncations, and exponent manipulations. This drastically inflates cognitive load in decompiler (F5) pseudocode while triggering path explosion and solver timeouts in SMT solvers (Z3) under non-linear floating-point constraints.
- **Dynamic Box Inversion & Modular Multiplicative Inverse**: Encrypts at compile time using odd multiplicative inverses $\pmod{2^8}$ and decrypts at runtime via a single multiplication; generates and inverts key-shuffled S-Boxes / P-Boxes on the fly at runtime, eradicating static lookup tables from the binary.
- **Global Positional Diffusion**: Incorporates Fisher-Yates permutation driven by 128-bit unbiased Lemire multiplicative range mapping, breaking byte-boundary alignment.

### 2. Reverse Key Ratchet & Full-Chain Avalanche Coupling
Refuses to expose independent per-layer keys in the binary, constructing a bidirectional state-coupled chain across compile-time forward encryption and runtime reverse decryption:
- **Zero Extra Key Storage ( $\pmod{2^{64}}$ Multiplicative Inverse Ratchet)**: Each chunk stores only a single root key. At compile time, outer layer keys are derived backward using 64-bit odd multiplicative inverses combined with the current layer's trailing byte; at runtime, the next layer's key is recovered forward:
  $$K _ {\text{next}} = (K _ {\text{cur}} + \text{ghost\\_state}) \times (\text{last\\_byte} \mid 1) \pmod{2^{64}}$$
- **Cross-Chunk Ghost State (`ghost_state`)**: After each chunk is decrypted and appended to the main container, the container's trailing byte is accumulated into the global `ghost_state`. Any attempt to skip preceding chunks in a debugger, patch a single byte in memory, or rip an individual sub-function for isolated emulation will trigger a total key avalanche across all subsequent chunks and layers.

### 3. Superlinear Chunking, High-Fidelity Junk Chunks & Number-Theoretic Opaque Predicates
- **Superlinear Chunking & Short-Block Padding**: Randomly slices payloads with an increasing step size of $\lceil i^{1.4} \rceil$ (finely fragmented at the head to scatter signatures, smoothly growing at the tail to cap total chunk count). Medium and high tiers enforce prepended PKCS#7 padding on short blocks ($<16\text{B}$) to eliminate length side-channels.
- **100% High-Fidelity Junk Chunks**: Probabilistically injects bogus data chunks that undergo the exact same multi-layer encryption, steganographic storage, and runtime decryption pipeline as real chunks—making them indistinguishable in decompiler views.
- **Number-Theoretic Opaque Predicates**: Injects integrity checks at the end of chunk decryption based on elementary number-theoretic identities (e.g., quadratic residues modulo 4 where $x^2 \not\equiv 2 \pmod 4$ always holds). Shielded by `black_box` against compile-time folding, junk chunks deterministically trigger self-wiping at runtime, keeping the main container and `ghost_state` strictly synchronized.

### 4. Call-Graph Shattering & LLVM Optimization Barriers
Purpose-built defenses against static cross-reference (Xrefs) tracking, batch function hooking, and aggressive compiler optimizations:
- **Adaptive Outlining & Vtable Trampolines**: As consecutive inlined layers accumulate, the probability of outlining into a standalone function rises exponentially. Outlined routines randomly emit either named `#[inline(never)]` functions (50%) or **dynamic-dispatch vtable trampolines** invoked indirectly via `&mut dyn FnMut` fat pointers (50%)—converting direct calls into indirect virtual calls and severing the static call graph.
- **O(log M) Bottom-Up Multi-Way Folding Tree**: When chunk counts grow large, adjacent groups of 4–12 chunk nodes are recursively encapsulated into a multi-level call tree, preventing the root function from growing linearly with payload size.
- **Cold-Section Eviction & Anti-IPO**: Probabilistically attaches `#[cold]` attributes to evict decryption logic into the `.text.unlikely` section, disrupting spatial locality; casts function pointers to `usize` and feeds them through `black_box` to fake address escape, preventing LLVM from performing Interprocedural Constant Propagation (IPCP) and Dead Argument Elimination (DAE).

### 5. Polymorphic Steganographic Storage & Controlled Memory Lifecycle
- **Polymorphic Ciphertext & Key Disguise**: Both ciphertexts and per-chunk 8-byte initial keys are encoded via randomized storage strategies—disguised as raw byte streams, `u64`/`u128` arrays, MAC addresses, UUIDs, IPv6 addresses, PEM certificates, or HB2 whitespace steganography to evade high-entropy section scans. HB2 steganography embeds a realistic text corpus (code snippets, logs, scripts) at compile time with randomized whitespace symbol tables, preventing both host text and symbol fingerprinting.
- **Full-Chain Volatile Zeroization**: Not only does the final returned container perform a full-capacity volatile wipe on `Drop`, but intermediate temporary buffers in variable-length decryption layers are also immediately zeroized upon handoff, leaving zero plaintext or intermediate residue in stack or heap memory.
- **Fine-Grained Stack Budget (`STACK_BUDGET`)**: Configurable via `budget-1k` through `budget-8m` features. The compiler tracks peak buffer expansion across multi-layer decryption and intelligently routes between stack (`Stack*`) and heap (`Heap*`) containers, seamlessly adapting to `no_std` bare-metal and embedded firmware.

### 6. Blazing-Fast Compilation for Multi-MB Payloads (No `long_running_const_eval`)
When embedding multi-megabyte binary firmware or asset files, expanding a massive payload into a single monolithic function causes LLVM's backend to hit $O(V^2)$ complexity explosion during register allocation. By leveraging native proc-macro evaluation (requiring no `#![allow(long_running_const_eval)]`), superlinear chunking, exponential inline backpressure, and an **O(log M) multi-way folding tree**, `obfstr2` strictly bounds each function body to a constant number of child nodes ( $\le 12$ ), enabling LLVM to compile multi-MB payloads in near-linear time.

### 7. Anti-Disassembly Junk Instruction Injection
To counter linear sweep and instruction pattern matching in disassemblers, `obfstr2` probabilistically injects architecture-gated inline assembly (junk instructions with zero data dependency on real state) at the entry and exit of non-inline wrapper functions, inlined decryption blocks, multi-level folding functions, and chunk scopes—deeply interleaving chaff with real decryption instructions.

---

## Design Philosophy & Boundaries

### Core Principles
1. **White-Box Threat Model**: Traditional ciphers (AES/ChaCha20) are unbreakable in black-box scenarios, but in a white-box model where the analyst has full control over the binary, standard magic constants allow attackers to locate keys/IVs and decrypt payloads in seconds. `obfstr2` optimizes for **maximizing reverse-engineering cognitive load and script-recovery cost**, forcing analysts to manually untangle non-standard polymorphic logic instance by instance.
2. **Complementary Probabilistic Security Model**: Encryption layers are not fixed templates. Instead, primitives are randomly sampled and stacked within the tier's latency budget until the combined security score meets the target threshold (`min_security`):
   $$\text{Security} _ {\text{combined}} = 1 - \prod _ {i=1}^{k} \left(1 - \frac{\text{Security} _ i}{100}\right) \ge \text{min\\_security}$$
3. **Decoupled Registry & Periodic Rotation**: Cryptographic primitives and steganographic storage strategies are isolated in static registries, leaving the orchestrator to handle the "chunk → encrypt → store → fold & emit" pipeline. Building on this pluggable architecture, the maintainers periodically rotate and expand underlying algorithms and pseudo-signatures to stay ahead of evolving deobfuscation rules.
4. **Compile-Time Evaluation & Zero Redundant Checks**: The proc-macro completes all encryption, state simulation, and length/alignment invariant verification at compile time; runtime code uses `unwrap_unchecked` to eliminate redundant branching overhead.

### Trade-offs

| We Prioritize                      | Over                             | Rationale                                                                                                                                                     |
|:-----------------------------------|:---------------------------------|:--------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Anti-Reversing Strength**        | Minimal Binary Size              | Cascaded decryption, junk chunks, and polymorphic expansion inevitably increase code size—a necessary cost to defeat static analysis and automated emulators. |
| **Non-Deterministic Polymorphism** | Reproducible Builds              | Generating completely distinct binary signatures on every compile is the core defense against automated signature extraction and batch scripts.               |
| **Practical White-Box Resistance** | Theoretical Everlasting Security | The goal is to drastically inflate the time, compute, and human effort required during reverse engineering, symbolic execution, and dynamic debugging.        |

### Use Cases vs. Non-Goals

| ✅ Recommended Use Cases                                                                                                                                                                                                                                                                                                                                                           | ❌ Non-Goals & Unsuitable Scenarios                                                                                                                                                                                                                                                                                                                                              |
|:-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| • Commercial software DRM, license check constants, embedded public keys / API keys<br>• Hiding sensitive syscall / module names (e.g., `ntdll.dll`) in game anti-cheat and security SDKs<br>• Protecting hardcoded credentials and proprietary protocol magic numbers in `no_std` embedded firmware<br>• Embedding and obfuscating multi-MB binary assets with fast compile times | • Release pipelines requiring deterministic binary hashes (Reproducible Builds)<br>• Compliance auditing scenarios requiring human-readable plaintext in binaries<br>• **Not a replacement** for standard cryptographic libraries in network transport or persistent storage<br>• **Does not** flatten host business logic control flow or include built-in anti-debugger checks |

---

## Macro Overview

All macros provide three security tiers: `1` (low latency), `2` (balanced), and `3` (high strength):

| Macro Family          | Input Literal Examples        | Return Type (`Deref` Target)                                 | Behavior & Constraints                                                                                                                                                            |
|:----------------------|:------------------------------|:-------------------------------------------------------------|:----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `s1!` ~ `s3!`         | `"hello"`                     | `StackStr<N>` / `HeapStr<N>`<br>(`Deref<Target = str>`)      | Automatically zeroized on `Drop`.                                                                                                                                                 |
| `b1!` ~ `b3!`         | `b"abc"` or `[0x41, 66]`      | `StackBytes<N>` / `HeapBytes<N>`<br>(`Deref<Target = [u8]>`) | Array elements must be in `0..=255`; zeroized on `Drop`.                                                                                                                          |
| `cs1!` ~ `cs3!`       | `c"..."` / `"..."` / `b"..."` | `StackCStr<N>` / `HeapCStr<N>`<br>(`Deref<Target = CStr>`)   | Appends trailing `\0` before obfuscation and restores zero-copy at runtime; use `b"..."` for non-UTF-8 bytes; **internal NULs are rejected** at compile time; zeroized on `Drop`. |
| `w1!` ~ `w3!`         | `"a\0b"`                      | `StackU16<N>` / `HeapU16<N>`<br>(`Deref<Target = [u16]>`)    | Encodes to UTF-16 at compile time; **no trailing NUL appended**, **internal NULs allowed**; ideal for length-prefixed APIs; zeroized on `Drop`.                                   |
| `cw1!` ~ `cw3!`       | `"kernel32.dll"`              | `StackU16<N>` / `HeapU16<N>`<br>(`Deref<Target = [u16]>`)    | Encodes to UTF-16 and **appends trailing `0u16`**; **internal NULs are rejected**; `as_ptr()` can be passed directly to Win32 `PCWSTR`; zeroized on `Drop`.                       |
| `f1!` ~ `f3!`         | `"assets/data.bin"`           | `StackBytes<N>` / `HeapBytes<N>`<br>(`Deref<Target = [u8]>`) | Path is resolved relative to the target crate's `CARGO_MANIFEST_DIR`; zeroized on `Drop`.                                                                                         |
| `i1!` ~ `i3!`         | `42u32` / `-1` / `0xFFu16`    | Raw primitive integer (`u8`~`i128` / `usize`)                | Unkempt suffix defaults to `i32`; `usize`/`isize` are encoded with 64-bit semantics; no `Drop` zeroization.                                                                       |
| `fl1!` ~ `fl3!`       | `3.15f32` / `-1.0` / `1e10`   | Raw primitive float (`f32` / `f64`)                          | Unkempt suffix defaults to `f64`; `inf`/`NaN` are rejected; preserves `-0.0` sign bit (use `to_bits()` for assertions); no `Drop` zeroization.                                    |
| `s_fmt1!` ~ `s_fmt3!` | `"hi, {}!", name`             | `alloc::string::String`                                      | Obfuscates static literal segments individually before passing to `format!`; requires `alloc` feature.                                                                            |

> **Usage Tip (Stack/Heap Routing & Type Inference):**
> When `alloc` is enabled (default), payloads $\le 16\text{B}$ always use stack containers, payloads reaching `STACK_BUDGET` always use heap containers, and intermediate sizes are routed probabilistically by tier. Because **the concrete return type (`Stack*` vs. `Heap*`) may vary across builds**, always rely on type inference (`let val = s2!("...");`) rather than hardcoding container type annotations.

---

## Feature Flags

| Feature                   | Default | Description                                                                                                                                                             |
|:--------------------------|:-------:|:------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `default`                 |   ✅    | Enables `alloc` and `anti-disassembly` by default.                                                                                                                      |
| `alloc`                   |   ✅    | Enables heap-backed containers (`Heap*`) and `s_fmt*!` macros. Compatible with `no_std + alloc` environments.                                                           |
| `anti-disassembly`        |   ✅    | Enables x86 / x86_64 anti-disassembly junk instruction inline assembly injection with red-zone safety protection. Can be disabled for size- or latency-critical builds. |
| `budget-1k` ~ `budget-8m` |   ❌    | Configures `STACK_BUDGET` ceiling: `1k`, `4k` (default), `16k`, `64k`, `256k`, `512k`, `1m`, `4m`, `8m`. If multiple flags are enabled, the largest takes precedence.   |

**Stack-Only Bare-Metal Mode (no `alloc`, no anti-disassembly injection):**
```toml
[dependencies]
obfstr2 = { version = "0.4", default-features = false }
```
*Note: When `alloc` is disabled, all tiers strictly use stack containers (`Stack*`). If a single chunk's peak decryption buffer exceeds `STACK_BUDGET / 2` (2 KiB by default), a compile-time error will prompt you to enable a larger `budget-*` feature.*

---

## Benchmarks

For small payloads (such as the 1 KiB sample below), `b1!` incurs near-zero runtime overhead; `b2!` and `b3!` trade higher runtime and compile-time costs for exponentially stronger anti-analysis protection. Conversely, for **multi-MB payloads**, `obfstr2` compiles significantly faster than `const`-eval alternatives thanks to native proc-macro execution and multi-way folding trees. Choose your tier based on whether the code path is latency-sensitive or security-critical.

Below is a single-run measurement for a 1 KiB payload over 512 decryption loops (numbers fluctuate slightly across polymorphic builds and are for order-of-magnitude reference only):

| Case                    | Per-Deobfuscation Latency | Total Time (512 Loops) | Incremental Build | Cold Build      | Binary Size (unstripped) | Macro Expansion (`cargo expand`) |
|:------------------------|:--------------------------|:-----------------------|:------------------|:----------------|:-------------------------|:---------------------------------|
| **Plaintext Baseline**  | —                         | 10.35 ms               | 293 ms            | 0.72 s          | 435 KB (1.0x)            | 1.4 KB (1x)                      |
| **CasualX `obfbytes!`** | ~0 µs                     | 10.29 ms (-1%)         | 353 ms (+21%)     | 2.14 s (+199%)  | 438 KB (+1%)             | 7.6 KB (5x)                      |
| **`obfstr2::b1!`**      | **19.66 µs**              | 20.42 ms (+97%)        | 695 ms (+138%)    | 6.97 s (+872%)  | 483 KB (+11%)            | 74.0 KB (53x)                    |
| **`obfstr2::b2!`**      | **569.86 µs**             | 302.12 ms (29x)        | 1.54 s (5.3x)     | 12.59 s (17.5x) | 610 KB (+40%)            | 238.7 KB (170x)                  |
| **`obfstr2::b3!`**      | **1319.54 µs**            | 685.95 ms (66x)        | 2.72 s (9.3x)     | 9.76 s (13.6x)  | 838 KB (+93%)            | 566.1 KB (403x)                  |

<details>
<summary>Click to view benchmark parameters and reproduction steps</summary>

Data is generated by `tests/perf.rs` and can be reproduced via `cargo test --test perf -- --ignored --nocapture`.
- **Parameters**: `PAYLOAD_LEN = 1024` (all `'a'`), `LOOPS = 512` (decrypt + checksum accumulation, wrapped in `black_box` to prevent loop-invariant code motion), `profile = release`, median of 3 runs per case.
- **Per-Deobfuscation Latency**: `(Row Time - Baseline Time) / 512`, canceling out fixed process startup overhead.
- **Build Times**: Cold builds compile an isolated temporary project from scratch (including dependencies); incremental builds share pre-warmed dependencies and only rewrite `main.rs`.

</details>

---

## Security

> **Disclaimer**: This project is intended strictly for legitimate use cases such as commercial software IP protection, anti-reverse-engineering research, and firmware hardening. Use in malware development or unauthorized penetration testing is strictly prohibited.

- **Platform Support**: Pure Rust implementation supporting major desktop OSes and `no_std` bare-metal targets (verified on `x86_64-unknown-none`; WebAssembly is theoretically supported but not tested in CI).
- **Unsafe Boundaries**: Generated code uses `unwrap_unchecked` exclusively where length invariants are strictly proven at compile time, alongside volatile memory writes during container `Drop`.
- **Vulnerability Disclosure**: If you discover a security vulnerability, please report it privately via [GitHub Private Vulnerability Reporting](https://github.com/AlbusPhos/obfstr2/security/advisories/new) or email `owo.zzz@proton.me`. **Do not** open a public issue.

---

## Contributing & Verification

Issues and Pull Requests are welcome! Adding new cryptographic primitives or storage strategies only requires appending entries to the static registries in `obfstr2-macros/src/crypto.rs` or `storage.rs` without modifying the core orchestration pipeline.

<details>
<summary>Click to expand local test matrix and PR checklist</summary>

**Test & Calibration Suite:**
```bash
# 1. Core macro correctness & unit tests (milliseconds)
cargo test --test smoke --test s_fmt
cd obfstr2-macros && cargo test --lib

# 2. Compile-fail tests for invalid inputs
cargo test --test compile_fail

# 3. Bare-metal no_std verification (requires x86_64-unknown-none target)
cargo test --test nostd

# 4. Performance benchmark report (requires network access & cargo-expand)
cargo test --test perf -- --ignored --nocapture

# 5. Storage & crypto latency/security calibration (long-running; last two require Python angr)
cd obfstr2-macros && cargo test --lib test_storages_latency -- --ignored --nocapture
cd obfstr2-macros && cargo test --lib tests::crypto::test_cryptos_latency -- --ignored --nocapture
cd obfstr2-macros && cargo test --lib tests::crypto::test_cryptos_security -- --ignored --nocapture
ANGR_PYTHONPATH=/path/to/python-with-angr cargo test --test angr -- main --ignored --show-output
```

**Pre-Submission Formatting & Linting:**
```bash
cargo fmt --all -- --check
cd obfstr2-macros && cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cd obfstr2-macros && cargo clippy --all-targets --all-features -- -D warnings
```
</details>

---

## License

This project is licensed under the [MPL-2.0](./LICENSE) License. See [CHANGELOG.md](CHANGELOG.md) for release history.