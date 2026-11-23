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

Comprehensive coverage across **strings, C strings, UTF-16 wide strings, byte arrays, embedded external files, integers, floating-point numbers, and format strings**. Supports direct dereferencing to raw primitives or storing into managed containers. All string and byte containers execute automatic volatile memory zeroization on `Drop`, ensuring sensitive plaintext never lingers in stack or heap allocations.

## Table of Contents
- [Quick Start](#quick-start)
- [Key Highlights & Adversarial Verification](#key-highlights--adversarial-verification)
- [Design Philosophy & Scope](#design-philosophy--scope)
- [Macro Overview](#macro-overview)
- [Feature Flags](#feature-flags)
- [Performance Benchmarks](#performance-benchmarks)
- [Security](#security)
- [Contributing & Development Verification](#contributing--development-verification)
- [License](#license)

## Quick Start

```toml
[dependencies]
obfstr2 = "0.5"
```

All obfuscation macros use numerical suffixes to denote security tiers: `1` (low latency), `2` (recommended balanced default), `3` (maximum resistance against reverse engineering);
Additionally, [`*_user!`](#custom-tier-_user) custom macros allow fine-grained field overrides over the balanced tier base.

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

## Key Highlights & Adversarial Verification

`obfstr2` is far beyond a basic "compile-time XOR" macro; it is a polymorphic cryptographic obfuscation pipeline engineered against **dedicated deobfuscation engines, white-box reverse engineering, decompilers (IDA / Ghidra F5), and SMT/symbolic execution engines (angr / Z3)**.

### Real-World Adversarial Verification: Full Immunity to Dedicated Deobfuscators (`rust-deobf`)

While traditional compile-time obfuscation crates bypass `strings` and generic tools like `FLOSS`, they crumble before [**`rust-deobf`**](https://github.com/VAHPEM/rust-deobf)—a tool purpose-built to break Rust binary idioms, inlined XOR patterns, static key layouts, and standard AEAD constructs.

Leveraging polymorphic primitive pipelines, cross-block ghost-state coupling, and call-graph shattering, `obfstr2` completely neutralizes `rust-deobf`'s static extraction and Unicorn emulation:

1. **Production Showcase ([`examples/showcase.rs`](examples/showcase.rs))**: Zero macro-protected payload strings were recovered (**0 decoded**). The deobfuscator only extracted standard library internal lookup-table noise.
2. **Automated Adversarial Test Suite ([`tests/rust_deobf.rs`](tests/rust_deobf.rs))**: Implanted with 5 realistic IOC categories matching `rust-deobf`'s benchmark suite (URL, IP:Port, Mutex, File Path, Registry Key). Baseline `obfstr` was cracked **5/5 within 0.1s**. In contrast, under `obfstr2::s2`, the function call graph fragmented (expanding from 14 to 452 functions), emulation time inflated by ~80x, and **0/5 strings were recovered (100% immune)**.

| Obfuscation Solution / Tool                    |    `strings`    |     `FLOSS`     | [`rust-deobf`](https://github.com/VAHPEM/rust-deobf) | Reverse Engineering & Deobfuscation Assessment                                      |
|:-----------------------------------------------|:---------------:|:---------------:|:----------------------------------------------------:|:------------------------------------------------------------------------------------|
| `litcrypt` / `litcrypt2`                       |      0 / 5      |      0 / 5      |                  **5 / 5 (Broken)**                  | ❌ Obvious static byte XOR patterns                                                 |
| `obfstr` (inlined XOR+ADD)                     |      0 / 5      |      0 / 5      |                  **5 / 5 (Broken)**                  | ❌ Fixed inline pattern; easily unwound via emulation                               |
| `cryptify` (default key)                       |      0 / 5      |      0 / 5      |                  **5 / 5 (Broken)**                  | ❌ Static routine signature and predictable key layout                              |
| `muddy` (ChaCha20-Poly1305)                    |      0 / 5      |      0 / 5      |                  **5 / 5 (Broken)**                  | ❌ Standard crypto constants and AEAD calls easily recognized                       |
| Manual compact stack strings / XOR / ADD       |      0 / 5      |      0 / 5      |                  **5 / 5 (Broken)**                  | ❌ Lacks control-flow barriers and cross-block state coupling                       |
| **`obfstr2` (`tests/rust_deobf.rs` verified)** | **0 recovered** | **0 recovered** |           **0 recovered (Full Immunity)**            | ✅ **Non-standard polymorphic primitives + avalanche ratchet + vtable trampolines** |

*Note: The first 5 baseline rows cite [`rust-deobf`'s official benchmark suite](https://github.com/VAHPEM/rust-deobf).*

> **Continuous Adversarial Evolution Commitment:**
> Reverse engineering is an active cat-and-mouse game. Thanks to our modular registry architecture, **the maintainers commit to regularly updating and rotating built-in cryptographic primitives and steganographic signatures**, keeping binary profiles fresh and invalidating automated heuristic extraction rules over time.

### 1. SMT- & Decompiler-Resistant Cryptographic Primitives
17+ built-in polymorphic primitives spanning stream ciphers, substitution-permutation networks (SPN), dynamic S/P-box transformations, and block ciphers, categorized by latency and security:
- **Proprietary MSE / MSE2 Cipher Families**: Integrates Cipher Feedback (CFB), state-dependent dynamic S-Boxes/P-Boxes mutating on every byte, 128-bit nonlinear key derivation using transcendental constant approximations ( $\pi$ , $1/e$ ), and 64-byte block 4-round ARX diffusion. Eliminates static cryptographic constants (e.g., AES/ChaCha20 magic numbers) that attackers exploit to automate decryption.
- **Floating-Point Cross-Domain Algebraic Obfuscation**: Translates integer bitwise operations (rotations, XORs) into equivalent IEEE-754 floating-point divisions, truncations, and exponent syntheses. While dramatically increasing decompiler (IDA F5) cognitive overhead, it triggers path explosions and solver timeouts in SMT engines (angr / Z3) grappling with nonlinear FP constraints.
- **Dynamic Box Inversion & Modular Multiplicative Inverses**: Encrypts using $\pmod{2^8}$ odd modular inverses at compile time and decrypts with a single runtime multiplication; dynamically shuffles and inverts S-Boxes/P-Boxes at runtime based on the key, eradicating static table footprints.
- **Global Permutation Diffusion**: Employs Fisher-Yates permutations driven by unbiased 128-bit multiplication interval mappings to dismantle byte-boundary alignments.

### 2. Inverse Key Ratchet & Cascade Avalanche Coupling
Eliminates independent per-layer keys in the binary, enforcing a bidirectional coupling chain between compile-time forward encryption and runtime reverse decryption:
- **Zero-Storage Key Ratchet ($\pmod{2^{64}}$ Modular Inverse)**: Each chunk stores only a single root key (masked at compile time by the entry `ghost_state`). Multi-layer encryption uses a 64-bit odd modular multiplicative inverse combined with the current layer's trailing byte to deduce the outer layer key; runtime decryption walks this forward:
  $$K _ {\text{next}} = (K _ {\text{cur}} + \text{ghost\\_state}) \times (\text{last\\_byte} \mid 1) \pmod{2^{64}}$$
- **Cross-Block Ghost State (`ghost_state`)**: After each chunk is decrypted and merged into the destination container, a sliding window of up to 16 bytes is hashed via `LightHasher` to mutate the global `ghost_state` (incorporating key hashes when dynamic keys are used). Any attempt to skip precursory chunks, patch bytes in memory, or simulate isolated functions in a debugger triggers an avalanche failure across all subsequent keys.

### 3. Superlinear Chunking, Realistic Chaff Blocks, & Opaque Predicates
- **Superlinear Chunking & Short Padding**: Splits payloads using an increasing step size of $\lceil i^{1.4} \rceil$ (fine-grained head to scatter signatures, expanding tail to bound total chunk count). Balanced and high-security tiers enforce pre-padded `pkcs7_16` on chunks < 16B to eliminate length side-channels.
- **100% Realistic Chaff (Junk) Blocks**: Injects fake chunks matching the exact multi-layer encryption, storage structures, and runtime decryption paths of real chunks, rendering them indistinguishable in static decompilation.
- **Polymorphic Algebraic & Number-Theoretic Opaque Predicates**: Decorates chunk epilogues with 4 classes of integrity verifications—including **nonlinear Mixed Boolean Arithmetic (MBA) identities dual to $\pmod{2^{64}}$ modular inverses**, **Hamming weight `popcnt` inclusion-exclusion principles with orthogonal bit decomposition (poison for SMT solvers)**, **2-adic ring Hensel odd power fixed-points ( $v^2 \equiv 1 \pmod 8$ , $v^4 \equiv 1 \pmod{16}$ )**, and **De Morgan symmetric difference MBAs**. Verification formulas bind runtime buffer addresses to `ghost_state`. Shielded by `black_box` against compiler folding, junk blocks or tampered states inevitably trigger silent `wipe_data()` zeroization.

### 4. Call Graph Shattering & LLVM Optimization Barriers
Engineered to defeat cross-reference (Xref) tracing, automated function hooking, and aggressive compiler optimizations:
- **Dual-Granularity Adaptive Outlining & Vtable Trampolines**: At both per-layer and per-chunk granularities, consecutive inlining exponentially increases the likelihood of function outlining. Outlined routines have a 50% chance of being emitted as standard `#[inline(never)]` functions or as indirect dynamic dispatch trampolines invoked via `&mut dyn FnMut` fat pointers, severing static call graphs.
- **$O(\log M)$ Bottom-Up Multi-Way Folding Trees**: Recursively packs clusters of 4–12 adjacent chunk nodes into multi-tier trees, keeping the root function signature lightweight.
- **Cold/Hot Splitting & Anti-Interprocedural Optimization (Anti-IPO)**: Randomly tags routines with `#[cold]` to exile decryption paths into `.text.unlikely` sections; casts function pointers to `usize` through `black_box` to fake address escapes, preventing LLVM from running interprocedural constant propagation (IPCP) and dead argument elimination (DAE).

### 5. Polymorphic Steganographic Storage & Controlled Lifecycle
- **Polymorphic Ciphertext & Key Camouflage**: Raw ciphertext and initial 8-byte chunk keys are encoded as random data representations: standard byte slices, `u64`/`u128` arrays, MAC addresses, UUIDs, IPv6 addresses, PEM certificates, or HB2 whitespace steganography. HB2 embeds encrypted data inside realistic source code, logs, or scripts using randomized invisible whitespace alphabets to defeat entropy scanners.
- **End-to-End Volatile Memory Wipe**: The returned containers wipe their underlying allocations on `Drop`, and intermediate dynamically-sized decryption buffers are immediately cleared upon handover, leaving zero plaintext or intermediate state in memory.
- **Fine-Grained Stack Budgeting (`STACK_BUDGET`)**: Controls pure stack ceilings via `budget-1k` through `budget-8m`. The macro engine tracks compile-time peak expansion across layers, intelligently routing between stack and heap containers to accommodate bare-metal and embedded `no_std` targets.

### 6. MB-Scale Large Payload Fast Compilation (No `long_running_const_eval`)
Large payloads expanded into a single massive function often choke LLVM's register allocator ($O(V^2)$ explosion). `obfstr2` uses procedural macro evaluation (bypassing `#![allow(long_running_const_eval)]`) combined with superlinear chunking, inline backpressure, and **$O(\log M)$ multi-way tree folding** to bound function node complexity to a constant factor ($\le 12$), allowing multi-megabyte payloads to compile smoothly in near-linear time.

### 7. Anti-Disassembly Junk Instruction Injection
To disrupt linear-sweep and recursive disassemblers, target-gated inline assembly (junk code) with zero true data dependencies is randomly injected across wrapper functions, inlined decryption segments, folding trees, and root expansion blocks.

---

## Design Philosophy & Scope

### Core Principles
1. **White-Box Threat Model**: Standard cryptography guarantees security under black-box assumptions. In white-box environments where the adversary controls binary execution, standard constants (e.g., ChaCha20 magic words) are quickly harvested by automated scripts. `obfstr2` aims to **maximize human cognitive friction and automated solver runtimes**, forcing attackers to reverse-engineer non-standard polymorphic logic.
2. **Complementary Probabilistic Security Metric**: Layer selection is dynamically determined by latency constraints and a target security threshold (`min_security`), sampling and stacking primitives (scaled via `security_factor`):
   $$\text{Security} _ {\text{combined}} = 1 - \prod _ {i=1}^{k} \left(1 - \frac{\text{Security} _ i}{100}\right) \ge \text{min\\_security}$$
3. **Decoupled Registries & Periodic Rotation**: Primitives and storage strategies are isolated into static registries. The orchestrator purely handles the "chunk $\to$ encrypt $\to$ store $\to$ fold $\to$ emit" pipeline. Maintainers rotate underlying algorithms and dummy signatures to stay ahead of automated heuristics.
4. **Compile-Time Verification with Zero-Cost Runtime Guarantees**: Procedural macros validate all states, alignments, and lengths during compilation; runtime code uses `unwrap_unchecked` to eliminate unnecessary branch checks.

### Trade-offs
| We Chose                           | Rather Than                          | Rationale                                                                                                      |
|:-----------------------------------|:-------------------------------------|:---------------------------------------------------------------------------------------------------------------|
| **Maximized Reverse Resistance**   | Minimal Binary Bloat                 | Cascaded decryption, chaff blocks, and outlined branches increase binary size to dismantle automated emulators |
| **Non-Deterministic Builds**       | Reproducible Builds                  | Generating distinct binary signatures across compilations prevents bulk signature matching                     |
| **White-Box Engineering Friction** | Theoretical Cryptographic Permanence | The objective is to make dynamic analysis, symbolic execution, and reverse engineering prohibitively expensive |

### Recommended Use Cases vs Non-Goals
| ✅ Recommended Use Cases                                                                                                                                                                                                                                                                                                       | ❌ Non-Goals / Outside Scope                                                                                                                                                                                     |
|:-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| • Commercial DRM, license verification constants, embedded API keys<br>• Anti-cheat and security SDKs hiding sensitive syscalls (e.g., `ntdll.dll`)<br>• Embedded and bare-metal `no_std` firmware concealing credentials and magic numbers<br>• Embedding and obfuscating megabyte-scale binary assets with rapid compilation | • Deterministic builds requiring stable binary hashes<br>• Scenarios mandating open auditing or human-readable binaries<br>• Does not replace standard cryptographic libraries for network or storage encryption |

---

## Macro Overview

All macros offer `1` (low latency), `2` (balanced), and `3` (high security) tiers:

| Macro Family                 | Literal Input Example                                       | Return Type (Deref Target)                                     | Key Properties & Constraints                                                                                                            |
|:-----------------------------|:------------------------------------------------------------|:---------------------------------------------------------------|:----------------------------------------------------------------------------------------------------------------------------------------|
| `s1!` ~ `s3!`                | `"hello"`                                                   | `StackStr<N>` / `HeapStr<N>`<br>（`Deref<Target = str>`）      | Automatically zeroized on `Drop`.                                                                                                       |
| `b1!` ~ `b3!`                | `b"abc"` or `[0x41, 66]`                                    | `StackBytes<N>` / `HeapBytes<N>`<br>（`Deref<Target = [u8]>`） | Byte arrays support `0..=255`; zeroized on `Drop`.                                                                                      |
| `cs1!` ~ `cs3!`              | `c"..."` / `"..."` / `b"..."`                               | `StackCStr<N>` / `HeapCStr<N>`<br>（`Deref<Target = CStr>`）   | Appends trailing `\0`; zero-copy runtime restoration. Non-UTF-8 uses `b"..."`. **Must not contain internal NUL**; zeroized on `Drop`.   |
| `w1!` ~ `w3!`                | `"a\0b"`                                                    | `StackU16<N>` / `HeapU16<N>`<br>（`Deref<Target = [u16]>`）    | Encoded to UTF-16; **no trailing NUL appended**, **allows internal NUL**; suitable for length-prefixed APIs; zeroized on `Drop`.        |
| `cw1!` ~ `cw3!`              | `"kernel32.dll"`                                            | `StackU16<N>` / `HeapU16<N>`<br>（`Deref<Target = [u16]>`）    | Encoded to UTF-16 with **trailing `0u16` appended**; **no internal NUL permitted**; `as_ptr()` fits Win32 `PCWSTR`; zeroized on `Drop`. |
| `f1!` ~ `f3!`                | `"assets/data.bin"`                                         | `StackBytes<N>` / `HeapBytes<N>`<br>（`Deref<Target = [u8]>`） | Relative to crate's `CARGO_MANIFEST_DIR`; zeroized on `Drop`.                                                                           |
| `i1!` ~ `i3!`                | `42u32` / `-1` / `0xFFu16`                                  | Native primitive scalar (`u8`~`i128` / `usize`)                | Default suffix resolves to `i32`; `usize`/`isize` encoded as 64-bit; no `Drop` wipe.                                                    |
| `fl1!` ~ `fl3!`              | `3.15f32` / `-1.0` / `1e10`                                 | Native primitive float (`f32` / `f64`)                         | Default suffix resolves to `f64`; rejects `inf`/`NaN`; preserves `-0.0` sign (assert with `to_bits()`); no `Drop` wipe.                 |
| `s_fmt1!` ~ `s_fmt3!`        | `"hi, {}!", name`                                           | `alloc::string::String`                                        | Obfuscates literal fragments before invoking `format!`; requires `alloc`.                                                               |
| `*_user!` (All families)     | `{ max_latency: 50, ... }, payload`                         | Matches corresponding macro family                             | Custom tier: First argument is a configuration block overriding balanced-tier defaults.                                                 |
| `*_key!` (All families)      | `{ build_key, run_key }, payload`                           | Matches corresponding macro family                             | Dynamic keying: Encrypts with compile-time `build_key`, decrypts with runtime `run_key`; rejects literals for `run_key`.                |
| `*_key_user!` (All families) | `{ build_key, run_key }, { max_latency: 50, ... }, payload` | Matches corresponding macro family                             | Combines dynamic keys with custom obfuscation configurations.                                                                           |

> **Usage Note (Stack/Heap Routing & Type Inference):**
> When `alloc` is enabled (default), payloads $\le 16\text{B}$ are guaranteed to use stack containers. Above budget limits (`STACK_BUDGET` for the primary container, `STACK_BUDGET / 2` for intermediate chunks), heap containers are enforced; between these thresholds, routing is probabilistically determined by tier preferences. Cargo features `force_heap` / `force_stack` (mutually exclusive) or the `force_heap` field in `*_user!` can override the **primary container** routing. Intermediate chunks remain heuristically routed based on budget and preference. `force_stack` still honors `STACK_BUDGET` and triggers compile-time errors if exceeded. **Because the concrete container type (`Stack*` vs `Heap*`) can shift across compilations**, always rely on type inference (e.g., `let val = s2!("...");`) and avoid hardcoding explicit container type annotations.

### Custom Tier `*_user!`

All macro variants follow a unified **`({ config }, payload)`** syntax: the first argument is a `{ key: val, ... }` block that overrides fields on top of the **balanced baseline (Tier 2)**:

```rust
use obfstr2::{cs_user, s_user};

fn main() {
    let greeting = s_user!(
        {
            prefer_stack: true,      // Biases probabilistic stack allocation
            max_latency: 60,         // Upper bound on per-layer latency (>= 2)
            min_security: 75,        // Target composite security threshold (<= 99)
            junk_pct: 12,            // Chaff block injection rate (0..=100)
            pad_short: true,         // Enforces PKCS7 padding on short chunks (< 16B)
        },
        "custom-tier",
    );
    assert_eq!(&*greeting, "custom-tier");

    // An empty block {} is equivalent to s2!
    assert_eq!(&*s_user!({}, "hello"), "hello");

    // Literal fragments in s_fmt_user! use custom obfuscation settings
    assert_eq!(cs_user!({ max_latency: 40 }, c"/bin/sh").to_str().ok(), Some("/bin/sh"));
}
```

| Config Key                  | Value Types                    | Default Values (`1` / **`2` Base** / `3`) | Description                                                                                      |
|:----------------------------|:-------------------------------|:-----------------------------------------:|:-------------------------------------------------------------------------------------------------|
| `prefer_stack`              | `bool`                         |      `true` / **`false`** / `false`       | Biases stack allocation (76% stack when `true`, 40% when `false`)                                |
| `force_heap`                | `bool` / `Some(bool)` / `None` |       `None` / **`None`** / `None`        | Overrides primary container: `true` forces heap, `false` forces stack, `None` follows heuristics |
| `max_latency`               | `u8` (≥ 2)                     |          `30` / **`80`** / `100`          | Latency upper bound per encryption layer and storage strategy                                    |
| `min_security`              | `u8` (≤ 99)                    |           `7` / **`63`** / `93`           | Minimum composite security threshold                                                             |
| `security_factor`           | `f64` (finite > 0.0)           |         `1.0` / **`1.0`** / `1.0`         | Security contribution scalar per layer: `< 1.0` increases layers, `> 1.0` reduces layers         |
| `wrap_pct`                  | `0..=100`                      |           `8` / **`50`** / `85`           | Outlining probability for layers and chunks (escalates with consecutive inlines)                 |
| `junk_pct`                  | `0..=100`                      |           `2` / **`18`** / `35`           | Realistic chaff junk block injection rate                                                        |
| `cold_pct`                  | `0..=100`                      |          `10` / **`20`** / `50`           | Probability of tagging outlined functions with `#[cold]`                                         |
| `pad_short`                 | `bool`                         |       `false` / **`true`** / `true`       | Forces `pkcs7_16` pre-padding on chunks < 16B                                                    |
| `anti_disasm_pct`           | `0..=100`                      |           `5` / **`40`** / `80`           | Anti-disassembly junk code injection rate (requires `anti-disassembly` feature)                  |
| `anti_disasm_count_range`   | `lo..=hi` (`u8`)               |      `1..=1` / **`1..=2`** / `2..=3`      | Randomized range of junk blocks injected per site                                                |
| `anti_disasm_garbage_range` | `lo..=hi` (`u8`)               |     `1..=5` / **`4..=10`** / `8..=15`     | Baseline junk byte range injected per site                                                       |

- Keys accept either `key: val` or `key = val` syntax, separated by commas. Trailing commas are permitted.
- `force_heap` accepts `true` / `false` / `Some(true)` / `Some(false)` / `None`; `false` explicitly forces stack allocation. Precedence: `*_user!` configuration > Cargo features (`force_heap` / `force_stack`) > default heuristics. Without `alloc`, setting `true` triggers a compile error.

### Dynamic Key Tier `*_key!`

Decouples payload encryption from decryption by requiring distinct build-time and runtime keys:

```rust
use obfstr2::s_key;

// Fetch runtime key from a remote server; incorrect keys result in garbage or intentional UB.
fn get_server_key() -> &'static [u8] {
    "obfstr2-demo-key".as_bytes()
}

fn main() {
    // build_key is used solely during compilation and is never embedded in plaintext
    let s = s_key!(
        { build_key: "obfstr2-demo-key", run_key: get_server_key() },
        "hello",
    );
    assert_eq!(&*s, "hello");
}
```

- `build_key` accepts `"..."`, `b"..."`, `[0x12, 0x34, ...]`, or `env!("VAR")` (read at compile time, registering Cargo environment rebuild hooks).
- `run_key` must be a dynamic runtime expression (`&[u8]`). Literals, `.as_bytes()`, or `env!` expressions are rejected.
- `run_key` must match `build_key` byte-for-byte; mismatches result in undefined outputs or trigger opaque-predicate zeroization (**an intentional design** to poison symbolic and emulation memory).

### Lightweight Helper Primitives

The root crate re-exports lightweight constants from `lib-unknown`, designed for **non-sensitive** constants and hot paths:

| Primitive          | Description                                                                                                    |
|:-------------------|:---------------------------------------------------------------------------------------------------------------|
| `const_random!`    | Compile-time pseudo-random integers; supports `()`, `(Type)`, and `(Type, "salt")`; usable in `const`/`static` |
| `hide_array!`      | XOR mask array hiding, usable in `const fn`, restored upon evaluation                                          |
| `hide_array_lazy!` | Wraps `hide_array!` inside `LazyLock` for first-access lazy restoration                                        |
| `lazy`             | `no_std`-compatible `LazyLock` / `OnceLock`                                                                    |

```rust
use std::hint::black_box;
use obfstr2::{const_random, hide_array, hide_array_lazy, s_user};
use obfstr2::types::str::StackStr;
use obfstr2::lazy::LazyLock;

const SALT: u64 = const_random!(u64, "demo");
static TABLE: LazyLock<[u32; 4]> = hide_array_lazy!(u32, [1, 2, 3, 4]);
// Fix container to StackStr using force_heap: false so it fits into LazyLock
static STR: LazyLock<StackStr<70>> = LazyLock::new(|| s_user!({ force_heap: false }, "hello_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"));

fn hot() {
    for _ in 0..black_box(100) {
        println!("{}", &*STR)
    }
}

fn main() {
    hot();
    let [a, b, c] = hide_array!(u8, [1, 2, 3]);
    assert_eq!((a, b, c), (1, 2, 3));
    assert_eq!(TABLE[0], 1);
    let _ = SALT;
}
```

> **Security Notice:** The helper primitives above perform basic masking and `black_box` wrapping, providing substantially weaker protection than `obfstr2`'s core polymorphic encryption macros. Always use `s1!` ~ `s3!` for critical constants like API keys, URLs, and syscall names.

---

## Feature Flags

| Feature                   | Default | Description                                                                                                                                                                         |
|:--------------------------|:-------:|:------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `default`                 |   ✅    | Enables `alloc` and `anti-disassembly` by default.                                                                                                                                  |
| `alloc`                   |   ✅    | Enables heap containers (`Heap*`) and `s_fmt*!` macros. Supports `no_std + alloc` environments.                                                                                     |
| `anti-disassembly`        |   ✅    | Enables x86 / x86_64 anti-disassembly junk instruction injection with Red-Zone protection. Can be disabled for extreme binary size constraints.                                     |
| `force_heap`              |   ❌    | Forces primary containers to allocate on the heap (implicitly enables `alloc`); mutually exclusive with `force_stack`.                                                              |
| `force_stack`             |   ❌    | Forces primary containers to allocate on the stack (intermediate chunks may still use heap); mutually exclusive with `force_heap`. Fails compilation if `STACK_BUDGET` is exceeded. |
| `budget-1k` ~ `budget-8m` |   ❌    | Stack allocation budget limit: options include `1k`, `4k` (default), `16k`, `64k`, `256k`, `512k`, `1m`, `4m`, `8m`. The largest active flag takes precedence.                      |

> `force_heap` and `force_stack` are mutually exclusive. Passing `--all-features` will trigger a compile error; use explicit feature combinations (see [Contributing & Development Verification](#contributing--development-verification)).

**Pure Stack Bare-Metal Mode (No `alloc`, No Junk Code):**
```toml
[dependencies]
obfstr2 = { version = "0.5", default-features = false }
```

*Note: Disabling `alloc` forces all tiers to allocate on the stack (`Stack*`). If `alloc` is kept but you want primary containers on the stack, enable `force_stack`. If total primary size reaches `STACK_BUDGET` or peak chunk expansion reaches `STACK_BUDGET / 2`, compilation will abort prompting an increase in the `budget-*` level.*

---

## Performance Benchmarks

For small payloads (such as the 1 KiB sample below), `b1!` incurs negligible runtime overhead; `b2!` and `b3!` scale up latency and compile times in exchange for higher analytical resistance. Conversely, on **multi-megabyte binary assets**, `obfstr2`'s procedural macro engine and multi-way tree folding compile significantly faster than traditional `const eval` solutions.

Sample measurements for a 1 KiB payload across 512 decryption loops (polymorphic builds vary slightly between runs):

| Case               | Latency per Deobfuscation | Total Runtime (512 loops, incl. startup) | Incremental Build Time | Clean Build Time     | Binary Size (unstripped) | Expanded AST Chars |
|:-------------------|:--------------------------|:-----------------------------------------|:-----------------------|:---------------------|:-------------------------|:-------------------|
| Plaintext Baseline | —                         | 10.30702ms                               | 171.832628ms           | 404.063369ms         | 445264                   | 1437               |
| CasualX obfbytes!  | 0.10µs                    | 10.360403ms (1%)                         | 202.252557ms (18%)     | 1.280738746s (217%)  | 448304 (1%)              | 7780 (441%)        |
| obfstr2 b1!        | 19.75µs                   | 20.419212ms (98%)                        | 353.501118ms (106%)    | 4.838922938s (1098%) | 500664 (12%)             | 77153 (5269%)      |
| obfstr2 b2!        | 317.91µs                  | 173.078567ms (1579%)                     | 888.272897ms (417%)    | 4.005122498s (891%)  | 612816 (38%)             | 224527 (15525%)    |
| obfstr2 b3!        | 829.70µs                  | 435.110996ms (4122%)                     | 2.436355796s (1318%)   | 5.346699777s (1223%) | 838360 (88%)             | 630688 (43789%)    |

<details>
<summary>Click to view benchmark parameters and reproduction commands</summary>

Benchmark suite located in `tests/perf.rs`. Reproduce with `cargo test --test perf -- --ignored --nocapture`.
- **Parameters**: `PAYLOAD_LEN = 1024` (repeated `'a'`), `LOOPS = 512` (decryption + checksum aggregation, wrapped in `black_box`), `profile = release`, median of 3 runs.
- **Latency per Deobfuscation**: `(current_runtime - baseline_runtime) / 512`, eliminating process initialization overhead.
- **Build Times**: Clean builds compile from scratch in a fresh temporary directory; incremental builds touch only `main.rs` with pre-warmed dependencies.
</details>

---

## Security

> **Disclaimer**: This project is intended solely for software IP protection, reverse-engineering research, and firmware security hardening. It must not be used for malicious software development or unauthorized exploitation.

- **Platform Support**: Pure Rust implementation, verified across major desktop OSes and bare-metal `no_std` environments (`x86_64-unknown-none`).
- **Unsafe Boundaries**: Generated code restricts `unsafe` usage to `unwrap_unchecked` where slice lengths are mathematically verified at compile time, and volatile memory writes inside container `Drop` implementations.
- **Vulnerability Disclosure**: Report security vulnerabilities privately via [GitHub Private Vulnerability Reporting](https://github.com/AlbusPhos/obfstr2/security/advisories/new) or by emailing `owo.zzz@proton.me`. **Do not** create public issues.

---

## Contributing & Development Verification

Issues and pull requests are welcome! New cryptographic primitives or storage strategies only require appending entries into the registries in `obfstr2-macros/src/crypto.rs` or `storage.rs`, without altering the core pipeline.

<details>
<summary>Click to expand local test matrix and PR verification checklist</summary>

**Standard Test & Calibration Suite:**
```bash
# 1. Basic functionality and macro sanity tests
cargo test --test smoke --test s_fmt
cd obfstr2-macros && cargo test --lib

# 2. Compile-fail test suite
cargo test --test compile_fail

# 3. Bare-metal no_std pipeline verification (requires x86_64-unknown-none target)
cargo test --test nostd

# 4. Performance benchmark report
cargo test --test perf -- --ignored --nocapture

# 5. Storage strategy and cryptographic latency calibration
cd obfstr2-macros && cargo test --lib test_storages_latency -- --ignored --nocapture
cd obfstr2-macros && cargo test --lib tests::crypto::test_cryptos_latency -- --ignored --nocapture

# 6. Automated reverse-engineering adversarial regressions (angr & rust-deobf)
# Requires a Python virtual environment containing angr and rust-deobf. Activate it or supply VIRTUAL_ENV:
source /path/to/.venv/bin/activate
cargo test --manifest-path obfstr2-macros/Cargo.toml --lib tests::crypto::test_cryptos_security -- --ignored --nocapture
cargo test --test angr -- main --ignored --nocapture
cargo test --test rust_deobf -- main --ignored --nocapture
# Or explicitly pass the environment:
# VIRTUAL_ENV=/path/to/.venv cargo test --test rust_deobf -- main --ignored --nocapture
```

**Pre-Commit Style & Quality Checks:**
```bash
cargo fmt --all -- --check
cd obfstr2-macros && cargo fmt --all -- --check
cargo clippy --all-targets --features alloc,anti-disassembly -- -D warnings
cargo clippy --all-targets --features force_heap,anti-disassembly -- -D warnings
cargo clippy --all-targets --features alloc,force_stack,anti-disassembly -- -D warnings
cd obfstr2-macros && cargo clippy --all-targets --features alloc,anti-disassembly -- -D warnings

# Optional: Run feature-powerset smoke tests using cargo-hack
cargo hack test --feature-powerset \
  --include-features alloc,force_heap,force_stack,budget-4m \
  --mutually-exclusive-features force_heap,force_stack \
  --test smoke
```
</details>

---

## License

This project is licensed under the [MPL-2.0](./LICENSE) License. See [CHANGELOG.md](CHANGELOG.md) for release history and version notes.