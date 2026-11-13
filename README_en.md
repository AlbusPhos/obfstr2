<!-- i18n-sync-anchor:(sha256:f12972bab62efbe0617322f59dc0069556a42fcdced7d5f0afc8ddc7a45d3900 reviewed:false source:README.md) -->
# obfstr2

> **Polymorphic compile-time string/wide-str/wide-cstr/bytes/int/float/cstr/file obfuscation**

[![Crates.io](https://img.shields.io/crates/v/obfstr2?logo=rust)](https://crates.io/crates/obfstr2)
[![Docs.rs](https://img.shields.io/docsrs/obfstr2?logo=docs.rs)](https://docs.rs/obfstr2)
[![CI](https://img.shields.io/github/actions/workflow/status/AlbusPhos/obfstr2/ci.yml?logo=github&label=CI)](https://github.com/AlbusPhos/obfstr2/actions)
[![MSRV](https://img.shields.io/crates/msrv/obfstr2?logo=rust&label=MSRV)](https://crates.io/crates/obfstr2)
[![Downloads](https://img.shields.io/crates/d/obfstr2?color=blue)](https://crates.io/crates/obfstr2)
[![License](https://img.shields.io/crates/l/obfstr2)](https://mozilla.org/MPL/2.0/)

**Language:** English | [简体中文](README.md)

A drop-in, one-line macro framework for compile-time security obfuscation in Rust: literal in, secure expression out, with all encryption and data transformation completed at compile time. Every build generates completely different binary signatures and ciphertext forms, thoroughly defeating signature-based static scanning and batch reverse-engineering scripts.

Comprehensive coverage across nine payload types: native strings, UTF-16 wide strings, UTF-16 wide C strings, byte arrays, external static files, scalar integers, floating-point numbers, C-style strings (`CStr`), and format strings. Supports dereferencing directly to raw values or storing in controlled secure containers; string and byte containers perform volatile memory zeroing upon going out of scope (`Drop`) to prevent sensitive plaintext from lingering in stack or heap memory.

## Table of Contents

- [Quick Start](#quick-start)
- [Core Highlights](#core-highlights)
- [Design Philosophy](#design-philosophy)
- [Macro Overview](#macro-overview)
- [When to Use vs. When Not to Use](#when-to-use-vs-when-not-to-use)
- [Feature Flags](#feature-flags)
- [Platform & Environment Support](#platform--environment-support)
- [Benchmarks](#benchmarks)
- [Security](#security)
- [Contributing](#contributing)
- [Changelog](#changelog)
- [License](#license)

## Quick Start

```toml
[dependencies]
obfstr2 = "0.4"
```

```rust
use obfstr2::{b2, cs2, cw2, f2, fl2, i2, s2, s_fmt2, w1};

fn main() {
    // String: evaluates to plaintext, returns an owned container that can be bound, passed, and reused
    let hello = s2!("hello");
    print!("{hello}");
    // C-string: evaluates to plaintext, dereferences to `CStr`, `as_ptr()` is ready for syscalls/FFI
    let sh = cs2!(c"/bin/sh");
    assert_eq!(&*sh, c"/bin/sh");
    // Wide C-string: UTF-16 code units + automatic trailing 0u16; as_ptr() is ready for Win32 PCWSTR
    let w = cw2!("kernel32.dll");
    assert_eq!(w[w.len() - 1], 0);
    // Wide string: UTF-16 code units, no trailing NUL, internal NUL allowed
    let ws = w1!("a\0b");
    assert_eq!(&*ws, &[0x61u16, 0, 0x62][..]);
    // Byte string / byte array: dereferences to plaintext `[u8]`
    let b = b2!(b"abc");
    let c = b2!([0x61, 98, 99]);
    assert_eq!(&*b, &*c);
    // Integer: evaluates to raw value, ready for arithmetic and comparison (unsuffixed defaults to `i32`)
    let x = i2!(42u32);
    assert_eq!(x + 1, 43u32);
    // Float: evaluates to raw value (unsuffixed defaults to `f64`, `inf` / `NaN` rejected)
    let y = fl2!(3.15);
    assert_eq!(y.to_bits(), 3.15f64.to_bits());
    // File: path relative to the compiled crate's CARGO_MANIFEST_DIR, read at compile time
    let d = f2!("assets/fixture.bin");
    // Format string: literal segments are obfuscated individually then passed to `format!`, placeholders work as usual, returns `String`
    let name = "world";
    let greeting = s_fmt2!("hello, {}!", name);
    print!("{greeting}");
}
```

For more scenarios, see [`examples/showcase.rs`](examples/showcase.rs) (real-world application bootstrap configuration: `cargo run --example showcase`).

> **Security Levels:** All obfuscation macros offer three security levels: 1 (low overhead), 2 (recommended/balanced), and 3 (high-intensity anti-reverse engineering).

## Core Highlights

`obfstr2` is not a simple "compile-time XOR" macro, but a polymorphic cryptographic obfuscation pipeline purpose-built against **white-box reverse analysis, decompilers (IDA/Ghidra F5), and symbolic execution engines (angr/Z3)**:

### 1. Cryptographic Primitive Library Resistant to Symbolic Execution and Decompilation
Ships 17+ polymorphic encryption/decryption primitives spanning stream ciphers, substitution-permutation networks (SPN), dynamic box transforms, and block ciphers, quantified and graded by runtime latency and security:
- **In-house MSE / MSE2 algorithm family**: combines CFB ciphertext feedback, dynamic S-Box / P-Box that mutate byte-by-byte with historical state, 128-bit nonlinear key derivation based on fixed-point approximations of transcendental numbers ($\pi$, $1/e$), and 4-round ARX diffusion over 64B blocks — eliminating any chance of identifying standard algorithm constants (such as ChaCha20 signatures) and batch-decrypting with off-the-shelf tools.
- **Cross-domain floating-point algebraic obfuscation (`rot_xor_f` / `xor_diff_f`)**: expands basic integer bit operations (rotation, XOR) into equivalent IEEE-754 floating-point division, truncation, and exponent construction. While sharply raising the cognitive load of decompiler F5 pseudo-code, it directly triggers path explosion and solver timeouts in SMT solvers under nonlinear floating-point constraints.
- **Dynamic box inversion and modular multiplicative inverses**: compile-time encryption with odd multiplicative inverses modulo $2^8$, runtime single-multiplication decryption (`inv_mul`); runtime key-driven shuffling generates S-Box / P-Box and their inverses (`gen_s` / `inv_s`), completely removing static lookup-table signatures from the binary.
- **Global positional diffusion**: introduces a `fisher_yates` permutation based on a 128-bit unbiased multiplicative interval mapping, breaking byte-boundary alignment.

### 2. Reverse-Engineering Key Ratchet and End-to-End Avalanche Coupling
Refuses to expose per-layer keys in the binary; instead builds a bidirectional state-coupling chain spanning "compile-time forward encryption" and "runtime reverse-order decryption":
- **Zero extra key storage ($\pmod{2^{64}}$ multiplicative-inverse ratchet)**: each chunk stores only a single root key. At compile time, the 64-bit odd multiplicative inverse `inv_mul64` combined with the current layer's last byte derives the outer-layer key in reverse; at runtime, decryption restores the next layer's key forward from the last byte:
  $$K_{\text{next}} = (K_{\text{cur}} + \text{ghost\\_state}) \times (\text{last\\_byte} \mid 1) \pmod{2^{64}}$$
- **Cross-chunk ghost state (`ghost_state`)**: after each chunk is decrypted and appended to the main container, the main container's trailing bytes are extracted and accumulated into the global `ghost_state` (mirrored precisely at compile time by the `sim_ghost` simulator). Any attempt to skip preceding chunks in a debugger, patch a single memory byte, or statically strip a single sub-function causes a complete avalanche across all subsequent chunks and layer keys.

### 3. Superlinear Chunking, Realistic Junk Chunks, and Number-Theoretic Opaque Predicates
- **Superlinear chunking with short-chunk padding**: splits payloads at random increasing steps of $\lceil i^{1.4} \rceil$ (the head is extremely fragmented to break signatures, the tail grows smoothly to control chunk count); mid/high tiers force `pkcs7_16` padding in front of short blocks ($<16\text{B}$), eliminating short-string length side channels.
- **100% realistic junk chunks**: forged data chunks are randomly injected according to tier-dependent probabilities and go through exactly the same multi-layer encryption, ciphertext storage, and runtime decryption as real chunks, making them indistinguishable in decompiler views.
- **Elementary number-theoretic opaque predicates**: integrity checks based on number-theoretic identities such as quadratic residues modulo 4 (e.g., $x^2 \not\equiv 2 \pmod 4$ always holds) are injected at the end of chunk decryption. After `black_box` blocks compile-time folding, junk chunks necessarily trigger `wipe_data()` at runtime to wipe themselves, keeping the main container and `ghost_state` perfectly consistent.

### 4. Call-Graph Smashing and LLVM Optimization Barriers
Defends specifically against static cross-reference (Xrefs) tracking, bulk function hooking, and aggressive compiler optimization:
- **Adaptive wrapping and vtable trampolines**: the more consecutive inline layers, the exponentially higher the probability of outsourcing to a standalone function. Outsourced forms are randomly chosen at 50% probability between a named `#[inline(never)]` function and a **dynamically dispatched trampoline** invoked indirectly through a `&mut dyn FnMut` fat pointer, converting direct calls into vtable-indirect calls and cutting the static call graph.
- **O(log M) bottom-up multiway fold tree**: when there are many chunks, adjacent 4–12 chunk nodes are recursively wrapped into a multi-level call tree (`__fold_{id}_{depth}_{idx}`), preventing the root function from growing linearly with payload size.
- **Cold/hot section segregation and anti-IPO**: randomly attaches `#[cold]` with some probability to evict decryption logic into `.text.unlikely`, destroying spatial locality; casts function pointers to `usize` and feeds them into `black_box` to fake address escape, preventing LLVM from performing cross-procedural constant propagation and dead-argument elimination.

### 5. Polymorphic Steganographic Storage and Controlled Memory Lifetime
- **Polymorphic ciphertext and key disguise**: ciphertext and each chunk's 8-byte initial key are encoded through a random `Storage` strategy, disguised as ordinary byte streams, `u64`/`u128` integer arrays, MAC addresses, UUIDs, IPv6 addresses, PEM certificates, or HB2 whitespace steganography, evading high-entropy data-section scanning.
- **End-to-end volatile wiping**: not only do the final returned string/byte containers perform volatile zeroing over their entire capacity when leaving scope (`Drop`), but intermediate temporary containers of variable-length decryption layers (such as `mse2`, `pkcs7_16`) are immediately `drop`-wiped after handover, preventing plaintext or intermediate states from lingering in stack/heap memory.
- **Fine-grained stack budget (`STACK_BUDGET`)**: `budget-1k` through `budget-8m` features control the pure-stack ceiling; compile time automatically tracks the peak inflated size across multi-layer encryption/decryption and routes intelligently between `Stack*` and `Heap*` containers, fitting `no_std` bare-metal and embedded firmware.

### 6. Blazing-Fast Compilation of MB-Scale Payloads (No `long_running_const_eval`)
When handling MB-scale or larger payloads (e.g., embedding large binary firmware or resource files via `f1!` / `f2!`), `obfstr2` compiles significantly faster than the traditional `obfstr`, and **requires no `#![allow(long_running_const_eval)]` in your code**:
- **Inline backpressure and multiway fold trees**: expanding a large payload into a single giant function would plunge the LLVM backend into $O(V^2)$ complexity explosion during register allocation and instruction scheduling. `obfstr2` uses $\lceil i^{1.4} \rceil$ superlinear chunking to control total chunk count, combined with exponential inline backpressure (`effective_wrap_pct`) and an **O(log M) multiway fold tree (`fold_chunk_nodes`)**, strictly converging each function body's child count to a constant level ($\le 12$), letting the LLVM backend compile multi-MB payloads in roughly linear time.

---

## Design Philosophy

### Core Principles

1. **White-Box Threat Model** — Traditional cryptographic algorithms (such as standard ChaCha20/AES) are unbreakable in a black-box setting, but under a **white-box model** where reverse engineers have full control over the binary, once algorithm signatures or constants are identified, an attacker only needs to extract the Key/IV to batch-decrypt with a script in seconds. `obfstr2` takes **maximizing reverse-engineering cognitive resistance and script recovery time** as its core metric, forcing attackers to analyze each case individually through non-standard in-house primitives and polymorphic orchestration.
2. **Complementary Probabilistic Security Model** — Layer selection is not a fixed recipe. Based on the tier's latency ceiling (`max_latency`) and target security (`min_security`), primitives are randomly sampled and stacked from the pool until the combined security satisfies:
   $$\text{Security}_{\text{combined}} = 1 - \prod_{i=1}^{k} \left(1 - \frac{\text{Security}_i}{100}\right) \ge \text{min\_security}$$
3. **Registry-Decoupled Architecture** — All algorithms converge into two static registries: `Crypto { name / enc / dec / support / security / latency / metrics }` (polymorphic encryption/decryption) and `Storage { ast / support / latency }` (polymorphic ciphertext storage). The orchestrator `build_obfuscated_bytes` is solely responsible for the *chunk → encrypt → store → fold-emit* pipeline. Adding a new primitive only requires appending a table entry without modifying the orchestration pipeline.
4. **Compile-Time Evaluation with Zero Redundant Checks** — Procedural macros complete all encryption and state simulation at compile time and expand into self-contained TokenStreams; runtime depends only on `types`, `crypto`, and `encoding` from `lib-unknown` (`no_std` compatible). Length/alignment invariants already strictly verified at compile time are used directly at runtime via `unwrap_unchecked`, leaving no branch-validation overhead.

### Trade-offs

| We Chose                       | Over                                        | Reason                                                                                                                                                 |
|:-------------------------------|:--------------------------------------------|:-------------------------------------------------------------------------------------------------------------------------------------------------------|
| Security Strength              | Minimal Binary Size                         | High-strength cascaded decryption, junk chunks, and polymorphic expansion inevitably cause code bloat—a necessary cost to block reverse analysis       |
| Non-Deterministic Builds       | Reproducible Builds                         | Guaranteeing that every build has completely different artifact signatures is the core defense against automated signature extraction and bulk scripts |
| White-Box Adversarial Strength | Theoretical Cryptographic Indestructibility | The goal is to drastically raise the attacker's time and compute cost in white-box reverse analysis, symbolic execution, and dynamic debugging         |

> While permanent cryptographic-theoretical security is not guaranteed, modern cryptographic design principles (Confusion, Diffusion, SPN networks, Avalanche Effect) are deeply incorporated in the engineering implementation.

### Non-Goals

- Does not perform control-flow flattening on host business code (only call-graph and control-flow obfuscation on the internal decryption chain of macro expansions), nor does it include anti-debugger detection.
- Does not guarantee reproducible builds (Reproducible Builds), and does not replace standard cryptographic libraries for network transmission or persistent encrypted storage.

## Macro Overview

| Macro                             | Input                                                                                        | Security Level                                                                                                                                                                                |
|-----------------------------------|----------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `s1!` / `s2!` / `s3!`             | `"..."` string literal                                                                       | Low latency / Balanced / High strength                                                                                                                                                        |
| `b1!` / `b2!` / `b3!`             | `b"..."` or `[0x41, 66, ...]` (elements `0..=255`)                                           | Low latency / Balanced / High strength                                                                                                                                                        |
| `i1!` / `i2!` / `i3!`             | `42u8` / `-1` / `0xFFu16` etc. integer literals (unsuffixed defaults to `i32`)               | Low latency / Balanced / High strength                                                                                                                                                        |
| `fl1!` / `fl2!` / `fl3!`          | `3.15f32` / `-1.0` / `1e10` etc. float literals (unsuffixed defaults to `f64`)               | Low latency / Balanced / High strength                                                                                                                                                        |
| `cs1!` / `cs2!` / `cs3!`          | `"..."` / `c"..."` / `b"..."` (`b"..."` may contain non-UTF-8 bytes; internal NUL forbidden) | Low latency / Balanced / High strength                                                                                                                                                        |
| `w1!` / `w2!` / `w3!`             | `"..."` string literal (UTF-16, no trailing NUL, internal NUL allowed)                       | Low latency / Balanced / High strength                                                                                                                                                        |
| `cw1!` / `cw2!` / `cw3!`          | `"..."` string literal (UTF-16, appends a trailing `0u16`, internal NUL forbidden)           | Low latency / Balanced / High strength                                                                                                                                                        |
| `f1!` / `f2!` / `f3!`             | `"path/to/file"` file path literal                                                           | Low latency / Balanced / High strength                                                                                                                                                        |
| `s_fmt1!` / `s_fmt2!` / `s_fmt3!` | `"...{}..."` format string + args                                                            | Obfuscates literal segments by level then passes to `format!`, returning `String` (requires `alloc`, `no_std` compatible; level `2`/`3` segments may use `Heap*`, requiring default features) |

**Container Types**

- String macros → `StackStr<N>` / `HeapStr<N>`
- Byte / File macros → `StackBytes<N>` / `HeapBytes<N>`
- C-string macros → `StackCStr<N>` / `HeapCStr<N>` (`N` includes trailing `\0`, `N >= 1`; dereferences to `core::ffi::CStr`, auto-zeroed on `Drop`)
- Wide string macros → `StackU16<N>` / `HeapU16<N>` (`N` is the number of UTF-16 code units, excluding the terminator; internal NUL allowed; `Deref<Target = [u16]>`, auto-zeroed on `Drop`)
- Wide C-string macros → `StackU16<N>` / `HeapU16<N>` (`N` is the number of UTF-16 code units, including the trailing `0u16`; `Deref<Target = [u16]>`, `as_ptr()` is ready for `PCWSTR`, auto-zeroed on `Drop`)
- Integer macros expand to raw integer values (`u8`/`i8`/`u16`/`i16`/`u32`/`i32`/`u64`/`i64`/`u128`/`i128`/`usize`/`isize`, determined by the literal suffix)
- Float macros expand to raw floating-point values (`f32` / `f64`, determined by the literal suffix)

> The concrete return type may vary across compilations. Please rely on type inference (e.g., `let val = s2!("...");`) rather than hardcoding type annotations.

**Integer / Float Macros**

- Reuse the same byte-obfuscation core (integers are encoded in little-endian, floats via `to_bits` in little-endian, followed by *chunk → encrypt → store → emit*).
- No `Drop` auto-zeroing.
- `usize` / `isize` are encoded with 64-bit semantics (bridged via `u64` / `i64` then cast with `as`).
- Floats only accept finite normal/subnormal values (`inf` / `NaN` are strictly rejected); `-0.0` preserves its sign bit bitwise—use `to_bits()` rather than `==` for assertions.

**C-String Macros**

- Also reuse the byte-obfuscation core: the payload is appended with a trailing `\0` and obfuscated as a whole, then reconstructed at runtime via `from_inner_with_nul` (zero-copy: truncating at the first NUL and reusing the decrypted container directly as a CStr).
- All three literal forms are semantically equivalent; use `b"..."` for non-UTF-8 payloads.
- Any internal NUL byte in the payload triggers a compile-time error.

**Wide String Macros (No NUL)**

- Accept only `"..."` string literals; the payload is encoded to UTF-16 code units at compile time. **No trailing `0u16` is appended**, and **internal NUL is allowed** (`U+0000` is preserved as a regular code unit).
- Return `StackU16<N>` / `HeapU16<N>`, where `N` is the number of code units (an empty string yields `N = 0`). `Deref<Target = [u16]>`; auto-zeroed on `Drop`.
- The result is not guaranteed to be NUL-terminated; it fits length-delimited / length-prefixed APIs. Use `cwN!` when a NUL-terminated Win32 `PCWSTR` is required.
- `w1!` is always pure-stack; `w2!` / `w3!` choose between `StackU16` / `HeapU16` based on the security level and `alloc`.

**Wide C-String Macros**

- Accept only `"..."` string literals; the payload is encoded to UTF-16 code units at compile time, with exactly one trailing `0u16` appended. Any `U+0000` in the payload triggers a compile-time error (prevents truncation).
- Return `StackU16<N>` / `HeapU16<N>`, where `N` includes the trailing `0u16` (an empty string yields `N = 1`). `Deref<Target = [u16]>` and `as_ptr()` are ready for Win32 `PCWSTR` parameters such as `LoadLibraryW` / `GetModuleHandleW`.
- Non-BMP characters occupy two code units as a UTF-16 surrogate pair; `String::from_utf16(&w[..w.len() - 1])` reconstructs the text (exclude the trailing `0u16`).
- Use `wN!` for wide strings without a trailing NUL or with internal NUL data.
- `cw1!` is always pure-stack; `cw2!` / `cw3!` choose between `StackU16` / `HeapU16` based on the security level and `alloc`.

## When to Use vs. When Not to Use

**Good fit for:**

- Hiding strings and constant bytes in `no_std` firmware or bare-metal programs.
- Scenarios requiring evasion from bulk static string scanning, decompiler pseudo-code analysis, and symbolic execution.
- Embedding and obfuscating MB-scale large binary files (without triggering the `long_running_const_eval` compilation limit).

**Not suitable for:**

- Scenarios requiring compliance auditing or human-readable plaintext in binaries.
- Release pipelines requiring deterministic artifact hashes (reproducible builds).

## Feature Flags

| Feature       | Default | Description                                                                                                                                         |
|:--------------|:--------|:----------------------------------------------------------------------------------------------------------------------------------------------------|
| `default`     | ✅      | Enables heap memory support: activates `alloc` for downstream `lib-unknown` and `obfstr2-macros`, allowing expanded code to use `Heap*` containers. |
| `budget-1k`   |         | Stack budget control: sets the pure-stack limit `STACK_BUDGET` to 1 KiB.                                                                            |
| `budget-4k`   |         | Stack budget control: sets `STACK_BUDGET` to 4 KiB (the default).                                                                                   |
| `budget-16k`  |         | Stack budget control: sets `STACK_BUDGET` to 16 KiB.                                                                                                |
| `budget-64k`  |         | Stack budget control: sets `STACK_BUDGET` to 64 KiB.                                                                                                |
| `budget-256k` |         | Stack budget control: sets `STACK_BUDGET` to 256 KiB.                                                                                               |
| `budget-512k` |         | Stack budget control: sets `STACK_BUDGET` to 512 KiB.                                                                                               |
| `budget-1m`   |         | Stack budget control: sets `STACK_BUDGET` to 1 MiB.                                                                                                 |
| `budget-4m`   |         | Stack budget control: sets `STACK_BUDGET` to 4 MiB.                                                                                                 |
| `budget-8m`   |         | Stack budget control: sets `STACK_BUDGET` to 8 MiB.                                                                                                 |

Notes:

- Disabling default features switches to pure-stack mode: Level 1 macros (`s1` / `b1` / `i1` / `fl1` / `cs1` / `w1` / `cw1` / `f1`) are always pure-stack and unaffected. If the peak intermediate chunk container exceeds `STACK_BUDGET`/2 (2048B under the default 4 KiB) without heap support enabled, a compile-time error will be emitted.
- `budget-*` features combine freely: when multiple are enabled, the largest takes effect; if none is specified, `STACK_BUDGET` defaults to 4 KiB.
- `s_fmt1!` / `s_fmt2!` / `s_fmt3!` expand to `alloc::format!` and return `String`, requiring `alloc` downstream (`no_std` compatible), regardless of pure-stack settings.
- Runtime fixedly depends on `crypto` / `types` / `encoding` from `lib-unknown`; `encoding` is used for Hex / Base64 decoding in expanded code and cannot be opted out.

To disable default features (pure stack, no heap):

```toml
[dependencies]
obfstr2 = { version = "0.4", default-features = false }
```

## Platform & Environment Support

- `no_std` compatible; `Heap*` types require the `default` feature.
- Supported OS: No platform-specific code (pure Rust, works on all major desktop OSes; bare-metal toolchain verified on `x86_64-unknown-none`, see [Contributing](#contributing)).
- WebAssembly: `wasm32-unknown-unknown` / `wasm32-wasi` are theoretically supported but untested.

## Benchmarks

`b1!` has near-zero runtime overhead (on par with the plaintext baseline); `b2!` incurs roughly tens of times more runtime overhead; `b3!` has higher overhead corresponding to stronger anti-analysis resistance. For short payloads (such as the 1 KiB sample below), build time rises with the tier because polymorphic call trees and inline barriers are generated; **but for MB-scale and larger payloads, thanks to native procedural-macro execution, inline backpressure, and the $O(\log M)$ multiway fold tree, `obfstr2` actually compiles faster than the `obfstr` that relies on `const` evaluation, with no need for `allow(long_running_const_eval)`**. Choose your level based on whether your use case is "runtime-sensitive" or "reversing-cost-sensitive". Detailed metrics and reproduction steps are below.

The sole source for the data below is `tests/perf.rs` (re-runnable via `cargo test --test perf -- --ignored --nocapture`; report-only, numbers are displayed without assertions). Due to polymorphism, results vary across runs; the table below shows a single measured run for order-of-magnitude reference only.

<details>
<summary>Benchmark Configuration</summary>

| Parameter      | Value    | Description                                                                                                                                  |
|:---------------|:---------|:---------------------------------------------------------------------------------------------------------------------------------------------|
| `PAYLOAD_LEN`  | 1024     | Payload size in bytes, filled with `'a'`                                                                                                     |
| `LOOPS`        | 512      | Number of "decrypt + checksum" iterations per guest run                                                                                      |
| `REPEATS`      | 3        | Repetitions per test case, taking the median                                                                                                 |
| `TIMEOUT_SECS` | 300      | Timeout per single build / run (seconds)                                                                                                     |
| profile        | release  | Cold build compiles an isolated temp project from scratch (including 3rd-party deps); incremental build rewrites only `main.rs` after warmup |
| `EXPECTED_SUM` | 50855936 | Checksum assertion value (`1024 × 97 × 512`) as a correctness gate                                                                           |

Parameter Notes:

- **Time per Deobfuscation**: `(Row Runtime − Baseline Runtime) / LOOPS`, in µs; assumes identical process startup overhead across rows that cancels out via subtraction; baseline row is marked `—`; rows indistinguishable from baseline yield ≈0 or slightly negative values due to noise and are shown as-is.
- **Total Runtime**: `LOOPS` decryption iterations (including process startup overhead and container `Drop` zeroing); loop bodies are wrapped in `black_box` (applied symmetrically to the whole return value for both `CasualX` and `obfstr2`) to prevent the compiler from hoisting loop-invariant decryption.
- **Incremental Build Time**: Uses a shared pre-warmed runner (pre-compiling merged `obfstr` + `obfstr2` dependencies once) and measures the incremental compile time triggered by rewriting only `main.rs` (median of 3 runs); measures `build` only, skipping runtime / binary size / expand.
- **Cold Build Time**: Creates a fresh temporary project compiled from scratch each time (including 3rd-party dependencies, median of 3 runs); the difference between rows represents the marginal cost of macro expansion, while percentages are diluted by fixed compilation costs (for order-of-magnitude reference only).
- **Total Runtime / Binary Size / Expand Chars**: Sampled from the cold build phase (independent of build path, not re-measured during warm builds to save time).
- **Expand Chars**: Character count of `cargo expand` output (requires `cargo-expand` installed on the host, otherwise marked `n/a`; includes ~0.5k chars of boilerplate).
- **Binary Size**: Unstripped executable size via `fs::metadata` on `DnyRun::bin_path()`.
- **Parentheses `(%)`**: Percentage increase relative to the Plaintext Baseline (baseline is `0%`); `-0%` is the result of rounding to the nearest integer, indicating a value slightly below baseline.

</details>

| Case               | Time / Deobf | Total Runtime (512 loops, incl. startup) | Incremental Build     | Cold Build            | Binary Size (unstripped) | Expand Chars     |
|:-------------------|:-------------|:-----------------------------------------|:----------------------|:----------------------|:-------------------------|:-----------------|
| Plaintext Baseline | —            | 10.364751ms                              | 171.966449ms          | 454.140508ms          | 445040                   | 1437             |
| CasualX obfbytes!  | 0.01µs       | 10.371281ms（0%）                        | 202.474127ms（18%）   | 1.909735432s（321%）  | 448072（1%）             | 7780（441%）     |
| obfstr2 b1!        | 19.46µs      | 20.327803ms（96%）                       | 475.87713ms（177%）   | 5.003601624s（1002%） | 512192（15%）            | 71476（4874%）   |
| obfstr2 b2!        | 314.51µs     | 171.393708ms（1554%）                    | 1.232847096s（617%）  | 5.00176234s（1001%）  | 613456（38%）            | 190793（13177%） |
| obfstr2 b3!        | 768.22µs     | 403.694785ms（3795%）                    | 2.860009413s（1563%） | 6.236919564s（1273%） | 812000（82%）            | 536333（37223%） |

## Security

> **Disclaimer:** This project is intended strictly for legitimate and compliant use cases such as software security protection, anti-reverse-engineering research, and firmware code protection. Using it for malware development or unauthorized penetration testing is strictly prohibited.

- **Unsafe Code**: Expanded code contains `unwrap_unchecked` (length invariants are verified at macro-expansion time, incurring zero runtime check overhead); container `Drop` implementations zero out plaintext memory via volatile writes.
- **Vulnerability Disclosure**: If you discover a security issue, please report it through
  [GitHub Private Vulnerability Reporting](https://github.com/AlbusPhos/obfstr2/security/advisories/new),
  or send an email to `owo.zzz@proton.me`. **Please do not** create a public Issue or Pull Request for an undisclosed security vulnerability.

## Contributing

Issues and Pull Requests are welcome!

**Local Verification:**

- `cargo test --test smoke --test s_fmt`: Correctness tests for all macros, in-process assertions, finishes in milliseconds.
- `cargo run --example showcase`: Comprehensive showcase example.
- `cargo test --test compile_fail`: Compile-time rejection of invalid inputs (isolated `dyntest` projects per case).
- `cargo test --test nostd`: `x86_64-unknown-none` bare-metal toolchain check (`b1` pure stack without alloc + `b2` with heap; requires target installed).
- `cargo test --test perf -- --ignored --nocapture`: Performance comparison report (Plaintext baseline vs. CasualX vs. `b1/b2/b3`, measuring build time / runtime / expand chars / binary size; report-only without assertions, requires network access to fetch `obfstr` and `cargo-expand` installed).
- `cd obfstr2-macros && cargo test --lib`: Pure unit tests and encryption layer selection simulations (milliseconds, automatically skips `#[ignore]` cases).
- `cd obfstr2-macros && cargo test --lib test_storages_latency -- --ignored --nocapture`: Round-trip `dyntest` verification and latency calibration for all storage strategies (multi-length, multi-round sampling; builds temporary guest projects and takes a while).
- `cd obfstr2-macros && cargo test --lib tests::crypto::test_cryptos_latency -- --ignored --nocapture`: Comprehensive L-score latency calibration for primitives (multi-sample, multi-round, report-only, takes several minutes).
- `cd obfstr2-macros && cargo test --lib tests::crypto::test_cryptos_security -- --ignored --nocapture`: Primitive security score and `angr` resistance calibration (requires `angr` installed).
- `ANGR_PYTHONPATH=/path/to/python-with-angr cargo test --test angr -- main --ignored --show-output`: Symbolic execution resistance verification (requires `angr` installed; specify the Python interpreter path via `ANGR_PYTHONPATH`).

**Pre-Submission Checks:**

```bash
cargo fmt --all -- --check
cd obfstr2-macros && cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cd obfstr2-macros && cargo clippy --all-targets --all-features -- -D warnings
```

Before submitting a PR, please read the [Design Philosophy](#design-philosophy): new obfuscation primitives should be integrated as table entries in `Crypto` or `Storage` (located in `obfstr2-macros/src/crypto.rs` and `storage.rs`, respectively); new macros require a thin `macro_rules` wrapper + user documentation in `obfstr2/src/lib.rs`, plus implementation docs + `# Warning` section on the `obfstr2-macros` side.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for detailed version history.

## License

This project is open-sourced under the [MPL-2.0](./LICENSE) license. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project shall be licensed under the same terms.
