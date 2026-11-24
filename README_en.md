# obfstr2

> **Polymorphic compile-time string/wide-str/wide-cstr/bytes/int/float/cstr/file obfuscation**

[![Crates.io](https://img.shields.io/crates/v/obfstr2?logo=rust)](https://crates.io/crates/obfstr2)
[![Docs.rs](https://img.shields.io/docsrs/obfstr2?logo=docs.rs)](https://docs.rs/obfstr2)
[![CI](https://img.shields.io/github/actions/workflow/status/AlbusPhos/obfstr2/ci.yml?logo=github&label=CI)](https://github.com/AlbusPhos/obfstr2/actions)
[![MSRV](https://img.shields.io/crates/msrv/obfstr2?logo=rust&label=MSRV)](https://crates.io/crates/obfstr2)
[![Downloads](https://img.shields.io/crates/d/obfstr2?color=blue)](https://crates.io/crates/obfstr2)
[![License](https://img.shields.io/crates/l/obfstr2)](https://mozilla.org/MPL/2.0/)

**Language:** English | [简体中文](README.md)

A drop-in, one-line macro framework for compile-time polymorphic security obfuscation in Rust. Literal in, secure expression out—all encryption and control-flow mutations are performed at compile time. Every build generates completely distinct binary signatures and ciphertext forms, thoroughly defeating static signature scanning, Mandiant `FLOSS`, and specialized Rust deobfuscators (such as [`rust-deobf`](https://github.com/VAHPEM/rust-deobf)).

Comprehensive coverage for **strings, C strings, UTF-16 wide strings, byte arrays, external files, integers, floats, and formatted strings**. Supports direct dereferencing to raw values or storing in managed containers; all containers automatically execute volatile memory zeroing upon leaving scope (`Drop`) to prevent sensitive plaintext from lingering on the stack or heap.

## Table of Contents

- [Quick Start](#quick-start)
- [Core Highlights & Adversarial Validation](#core-highlights--adversarial-validation)
- [Use Cases & Design Boundaries](#use-cases--design-boundaries)
- [Macro Overview](#macro-overview)
- [Feature Flags](#feature-flags)
- [Benchmarks](#benchmarks)
- [Security](#security)
- [Contributing & Development Verification](#contributing--development-verification)
- [License](#license)

## Quick Start

```toml
[dependencies]
obfstr2 = "0.5"
```

All obfuscation macros use numeric suffixes to distinguish strength tiers: `1` (low latency), `2` (recommended balanced), and `3` (high-strength anti-RE); custom [`*_user!`](#custom-strength-tier-_user) and dynamic-key [`*_key!`](#dynamic-key-tier-_key) variants are also available.

```rust
use obfstr2::{b2, cs2, cw2, f2, fl2, i2, s2, s_fmt2, w1};

fn main() {
    // String: returns an owned secure container, automatically zeroed on Drop
    let hello = s2!("hello");
    print!("{hello}");

    // C string: derefs to &CStr, as_ptr() can be passed directly to FFI syscalls
    let sh = cs2!(c"/bin/sh");
    assert_eq!(&*sh, c"/bin/sh");

    // Wide C string: UTF-16 code units + trailing 0u16, as_ptr() ready for Win32 PCWSTR
    let w = cw2!("kernel32.dll");
    assert_eq!(w[w.len() - 1], 0);

    // Wide string: raw UTF-16 code units without trailing NUL, interior NULs allowed
    let ws = w1!("a\0b");
    assert_eq!(&*ws, &[0x61u16, 0, 0x62][..]);

    // Byte strings & byte arrays: derefs to &[u8]
    let b = b2!(b"abc");
    let c = b2!([0x61, 98, 99]);
    assert_eq!(&*b, &*c);

    // Scalar integers & floats: evaluates directly to raw primitive values
    let x = i2!(42u32);
    assert_eq!(x + 1, 43u32);
    let y = fl2!(3.15);
    assert_eq!(y.to_bits(), 3.15f64.to_bits());

    // External file: path relative to the current crate's CARGO_MANIFEST_DIR, read & obfuscated at compile time
    let asset = f2!("assets/fixture.bin");

    // Formatted string: obfuscates each literal segment individually before calling format!, returns String
    let name = "world";
    let greeting = s_fmt2!("hello, {}!", name);
    print!("{greeting}");
}
```

For more practical examples, see [`examples/showcase.rs`](examples/showcase.rs) (run `cargo run --example showcase`).

---

## Core Highlights & Adversarial Validation

Under a **white-box threat model** where the attacker has full control over the binary, once standard cryptographic constants (e.g., AES / ChaCha20 magic numbers) or fixed inline patterns are identified, ciphertexts can be batch-recovered in seconds via scripts or emulators. `obfstr2` is a polymorphic cryptographic obfuscation pipeline deeply tailored to counter **specialized deobfuscators, decompilers (IDA / Ghidra F5), and symbolic execution engines (angr / Z3)**.

### Adversarial Validation: 100% Immune to Specialized Deobfuscators (`rust-deobf`)

Powered by non-standard polymorphic primitive pipelines, cross-chunk ghost-state coupling, and call-graph shattering, `obfstr2` completely blocks both static extraction and Unicorn emulation analysis by the Rust-specialized deobfuscator [**`rust-deobf`**](https://github.com/VAHPEM/rust-deobf):

| Obfuscation Scheme / Tool                                              |  `strings`   |    `FLOSS`    | [`rust-deobf`](https://github.com/VAHPEM/rust-deobf) | Anti-Analysis & Deobfuscation Assessment                                                        |
|:-----------------------------------------------------------------------|:------------:|:-------------:|:----------------------------------------------------:|:------------------------------------------------------------------------------------------------|
| `litcrypt` / `litcrypt2`                                               |    0 / 5     |     0 / 5     |                **5 / 5 (All broken)**                | ❌ Obvious static byte-XOR signatures                                                           |
| `obfstr` (Inline XOR+ADD)                                              |    0 / 5     |     0 / 5     |                **5 / 5 (All broken)**                | ❌ Fixed inline unrolling patterns, easily recovered via instruction emulation                  |
| `cryptify` (Default key)                                               |    0 / 5     |     0 / 5     |                **5 / 5 (All broken)**                | ❌ Fixed decryption routine and key layout                                                      |
| `muddy` (ChaCha20-Poly1305)                                            |    0 / 5     |     0 / 5     |                **5 / 5 (All broken)**                | ❌ Standard cryptographic constants and AEAD call structures are accurately fingerprinted       |
| Handwritten compact stack strings / XOR / ADD                          |    0 / 5     |     0 / 5     |                **5 / 5 (All broken)**                | ❌ Lacks control-flow barriers and cross-block state coupling                                   |
| **`obfstr2` (Tested in [`tests/rust_deobf.rs`](tests/rust_deobf.rs))** | **0decoded** | **0 decoded** |             **0 decoded (100% Immune)**              | ✅ **Non-standard polymorphic primitives + cross-chunk avalanche ratchet + vtable trampolines** |

*Note: Baseline data for the first 5 entries is cited from the [official `rust-deobf` evaluation benchmark](https://github.com/VAHPEM/rust-deobf).*

> **Commitment to Continuous Adversarial Evolution:**
> Thanks to a static registry architecture that cleanly decouples encryption/decryption primitives from steganographic storage strategies, **the maintainers commit to regularly updating and rotating built-in cryptographic algorithms and steganographic pseudo-signatures**, continuously refreshing code expansion forms and data signatures to render targeted extraction rules obsolete.

### 1. Cryptographic Primitives Resistant to Symbolic Execution & Decompilation

Includes 16 polymorphic encryption/decryption primitives (spanning stream ciphers, SPNs, dynamic box transforms, and block ciphers), randomly sampled and stacked according to latency caps and target security levels:

- **Proprietary MSE / MSE2 Algorithm Family**: Combines CFB ciphertext feedback, dynamic S-Boxes / P-Boxes that mutate byte-by-byte with historical state, 128-bit non-linear key derivation based on fixed-point approximations of transcendental numbers ( $\pi$ , $1/e$ ), and 64B-block 4-round ARX diffusion—eliminating all standard cryptographic magic numbers.
- **Cross-Domain Floating-Point Algebraic Obfuscation**: Equivalently expands integer rotations and XORs into IEEE-754 floating-point division, truncation, and exponent construction, drastically increasing cognitive load in F5 pseudocode and inducing path explosion and timeouts in SMT solvers under non-linear floating-point constraints.
- **Dynamic Box Inversion & Modular Multiplicative Inverses**: Encrypts at compile time using odd multiplicative inverses $\pmod{2^8}$ and decrypts at runtime via a single multiplication; shuffles and inverts S-Boxes / P-Boxes on the fly at runtime from the key, eliminating static lookup tables in the binary.
- **Global Positional Diffusion**: Introduces Fisher-Yates permutation driven by unbiased 128-bit multiplicative interval mapping, breaking byte-boundary alignment.

### 2. Reverse Key Ratchet & Full-Chain Avalanche Coupling

Refuses to expose independent per-layer keys in the binary, building a bidirectional state-coupling chain spanning compile-time forward encryption and runtime reverse decryption:

- **Zero Extra Key Storage ( $\pmod{2^{64}}$ Multiplicative Inverse Ratchet)**: Each chunk stores only a single root key (masked by the entry `ghost_state`). At compile time, 64-bit odd multiplicative inverses combined with the current layer's trailing byte backward-derive the outer key; at runtime, the trailing byte forward-recovers the next layer's key:
  $$K _ {\text{next}} = (K _ {\text{cur}} + \text{ghost\\_state}) \times (\text{last\\_byte} \mid 1) \pmod{2^{64}}$$
- **Cross-Chunk Ghost State (`ghost_state`)**: After a chunk is decrypted and appended to the main container, a `LightHasher` digest is computed over a sliding window of up to the last 16B of the main container and XORed into the global `ghost_state` (mixing in the key hash in dynamic-key mode). Skipping preceding chunks, patching a single byte in memory, or ripping a single function for isolated emulation triggers a total avalanche across the subsequent key chain.

### 3. Superlinear Chunking, High-Fidelity Junk Chunks & Polymorphic Algebraic Opaque Predicates

- **Superlinear Chunking & Short-Chunk Padding**: Randomly slices payloads with base $\text{base} = \lceil i^{2.4} \times 0.3 \rceil$ and range $\text{base} \ldots \text{base} \times \max(1,\ (\text{remaining} \bmod 3 + 1) + \lfloor i/5 \rfloor)$ (fine-grained head chunks scatter signatures; fast-growing tail chunks amortize overhead); medium and high tiers enforce prepended `pkcs7_16` padding on short chunks (`< 16B`) to eliminate length side-channels on short strings.
- **100% High-Fidelity Junk Chunks**: Probabilistically interleaves decoy data chunks that undergo the exact same multi-layer encryption, storage, and runtime decryption pipeline—indistinguishable from real chunks in decompiler views.
- **Polymorphic Algebraic & Number-Theoretic Opaque Predicates**: Randomly injects 4 classes of integrity checks at the end of chunk decryption—**Non-linear MBA identities with $\pmod{2^{64}}$ inverse duality**, **Hamming-weight `popcnt` inclusion-exclusion with orthogonal bit splitting**, **2-adic ring Hensel odd-power fixed points ( $v^2 \equiv 1 \pmod 8$, $v^4 \equiv 1 \pmod{16}$ )**, and **De Morgan symmetric-difference bilateral MBA**. Expressions are bound to the buffer address and `ghost_state`, ensuring junk chunks and tampered chunks deterministically trigger `wipe_data()` self-scrubbing.

### 4. Call-Graph Shattering, Anti-Disassembly & Compiler Barriers

- **Adaptive Outlining & Vtable Trampolines**: At both single-layer and per-chunk granularities, outlining probability grows exponentially with consecutive inline steps; emits either `#[inline(never)]` functions or `&mut dyn FnMut` dynamic-dispatch trampolines at a 50/50 split, converting direct calls into indirect vtable calls and severing static call graphs (Xrefs).
- **$O(\log M)$ Multi-Way Folding Tree & Blazing-Fast MB-Scale Compilation**: Recursively encapsulates adjacent groups of 2–6 chunk nodes into a multi-level call tree. Combined with native proc-macro evaluation (no `#![allow(long_running_const_eval)]` needed), this avoids $O(V^2)$ LLVM register-allocation blowup on massive functions, compiling MB-scale payloads in near-linear time.
- **Hot/Cold Section Splitting & Anti-IPO**: Probabilistically attaches `#[cold]` to evict decryption logic into the `.text.unlikely` section; casts function pointers to `usize` through `black_box` to fake address escape, preventing LLVM interprocedural constant propagation.
- **Anti-Disassembly Junk Instruction Injection**: Randomly injects zero-data-dependency x86/x86_64 architecture-gated inline assembly (rogue bytes / desynchronizing jumps) at function and decryption-block boundaries to disrupt linear disassembly and instruction pattern matching.

### 5. Polymorphic Steganographic Storage & Managed Memory Lifecycle

- **Polymorphic Ciphertext & Key Camouflage**: Ciphertexts and initial keys are randomly encoded as raw byte streams, `u64`/`u128` arrays, MAC addresses, UUIDs, IPv6 addresses, PEM certificates, or HB2 whitespace steganography (embedded within randomized realistic code/log corpora using dynamic whitespace alphabets) to evade high-entropy scanners.
- **Full-Chain Volatile Scrubbing**: Both the final container on `Drop` and intermediate temporary containers in variable-length decryption layers immediately execute full-capacity volatile zeroing upon handoff, leaving no plaintext or intermediate state behind.

---

## Use Cases & Design Boundaries

| ✅ Recommended Use Cases                                                                                                                                                                                                                                                                                                                                                                                                                                           | ❌ Non-Goals & Design Trade-offs                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
|:-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| • **Core Asset Protection**: Commercial software DRM, license verification constants, embedded public keys / API keys<br>• **Anti-Cheat & Security SDKs**: Hiding sensitive syscalls and module names (e.g., `ntdll.dll`)<br>• **Bare-Metal & Embedded Firmware**: Hiding hardcoded credentials and protocol magic numbers in `no_std` environments<br>• **Large Asset Embedding**: Obfuscating MB-scale binary firmware/assets while requiring fast compile times | • **Reproducible Builds**: The polymorphic engine generates distinct binary signatures on every build; artifact hashes are non-deterministic<br>• **Extremely Size-Constrained & High-Frequency Hot Paths**: Cascaded decryption and junk chunks introduce necessary code bloat and runtime overhead (pair with `LazyLock` or use tier `1` on hot paths)<br>• **Data-in-Transit / At-Rest Encryption**: White-box obfuscation raises reverse-engineering costs; it does not replace standard cryptographic libraries for network transport or disk storage |

---

## Macro Overview

All macros provide three preset tiers: `1` (low latency), `2` (balanced), and `3` (high strength). **All container types (`Stack*` / `Heap*`) automatically perform volatile zeroing on `Drop`; scalar types (`i*` / `fl*`) evaluate directly to raw primitive values:**

| Macro Family          | Input Literal Example                                       | Return Type (`Deref` Target)                              | Features & Constraints                                                                                               |
|:----------------------|:------------------------------------------------------------|:----------------------------------------------------------|:---------------------------------------------------------------------------------------------------------------------|
| `s1!` ~ `s3!`         | `"hello"`                                                   | `StackStr<N>` / `HeapStr<N>` (`Deref<Target = str>`)      | UTF-8 string container.                                                                                              |
| `b1!` ~ `b3!`         | `b"abc"` or `[0x41, 66]`                                    | `StackBytes<N>` / `HeapBytes<N>` (`Deref<Target = [u8]>`) | Array elements must be in `0..=255`.                                                                                 |
| `cs1!` ~ `cs3!`       | `c"..."` / `"..."` / `b"..."`                               | `StackCStr<N>` / `HeapCStr<N>` (`Deref<Target = CStr>`)   | Automatically appends trailing `\0`, zero-copy runtime recovery; use `b"..."` for non-UTF-8; **no interior NULs**.   |
| `w1!` ~ `w3!`         | `"a\0b"`                                                    | `StackU16<N>` / `HeapU16<N>` (`Deref<Target = [u16]>`)    | Compile-time UTF-16 conversion; **no** trailing NUL appended, interior NULs **allowed** (for length-prefixed APIs).  |
| `cw1!` ~ `cw3!`       | `"kernel32.dll"`                                            | `StackU16<N>` / `HeapU16<N>` (`Deref<Target = [u16]>`)    | Compile-time UTF-16 conversion with **automatic trailing `0u16`**; **no interior NULs**; `as_ptr()` fits `PCWSTR`.   |
| `f1!` ~ `f3!`         | `"assets/data.bin"`                                         | `StackBytes<N>` / `HeapBytes<N>` (`Deref<Target = [u8]>`) | Path is relative to the compiled crate's `CARGO_MANIFEST_DIR`.                                                       |
| `i1!` ~ `i3!`         | `42u32` / `-1` / `0xFFu16`                                  | Raw primitive integer (`u8`~`i128` / `usize`)             | Unabbreviated suffix defaults to `i32`; `usize`/`isize` encoded with 64-bit semantics.                               |
| `fl1!` ~ `fl3!`       | `3.15f32` / `-1.0` / `1e10`                                 | Raw primitive float (`f32` / `f64`)                       | Unabbreviated suffix defaults to `f64`; rejects `inf`/`NaN`; preserves `-0.0` sign bit (use `to_bits()` in asserts). |
| `s_fmt1!` ~ `s_fmt3!` | `"hi, {}!", name`                                           | `alloc::string::String`                                   | Obfuscates only static literal segments in the format string before invoking `format!`; requires `alloc` feature.    |
| `*_user!` (All)       | `{ max_latency: 50, ... }, payload`                         | Same as corresponding family macro                        | Custom tier: overrides parameters field-by-field on top of the balanced tier (`2`) baseline.                         |
| `*_key!` (All)        | `{ build_key, run_key }, payload`                           | Same as corresponding family macro                        | Dynamic key tier: encrypts with `build_key` at compile time, decrypts with `run_key` at runtime.                     |
| `*_key_user!` (All)   | `{ build_key, run_key }, { max_latency: 50, ... }, payload` | Same as corresponding family macro                        | Configures both dynamic keys and custom tier parameters simultaneously.                                              |

> **Stack/Heap Routing & Type Inference Note:**
> When `alloc` is enabled (default), payloads $\le 16\text{B}$ always use the stack, payloads reaching the budget limit (main container `STACK_BUDGET` / intermediate chunk container `STACK_BUDGET / 2`) always use the heap, and sizes in between are routed probabilistically according to the tier's stack preference. You can force **main container** routing via the `force_heap` / `force_stack` Cargo features or the `force_heap` config key (precedence: `*_user!` config > Cargo feature > default heuristic; intermediate chunk containers always follow budget and heuristics). If the main container or single-chunk peak exceeds the stack budget and heap allocation is unavailable, a compile-time error is emitted. Because **the concrete return type (`Stack*` vs. `Heap*`) may vary dynamically across builds**, always rely on type inference (`let val = s2!("...");`) rather than hardcoding type annotations.

### Custom Strength Tier `*_user!`

The first argument is a configuration block `{ key: val, ... }` (supporting `:` or `=`), overriding fields individually on top of the **balanced tier (`2`) baseline**. Layer selection follows a complementary probability accumulation model, randomly sampling and stacking primitives until the combined security reaches the target threshold:

$$\text{Security} _ {\text{combined}} = 1 - \prod _ {i=1}^{k} \left(1 - \frac{\text{Security} _ i}{100}\right) \ge \text{min\\_security}$$

```rust
use obfstr2::{cs_user, s_user};

fn main() {
    let greeting = s_user!(
        {
            prefer_stack: true,      // Stack bias for heap/stack coin flip
            max_latency: 60,         // Per-layer latency cap (>= 2)
            min_security: 75,        // Combined security floor (<= 99)
            junk_pct: 12,            // Junk chunk injection probability (0..=100)
            pad_short: true,         // Enforce PKCS7 pre-padding on short chunks (< 16B)
        },
        "custom-tier",
    );
    assert_eq!(&*greeting, "custom-tier");
    assert_eq!(&*s_user!({}, "hello"), "hello"); // Empty config block is equivalent to s2!
    assert_eq!(cs_user!({ max_latency: 40 }, c"/bin/sh").to_str().ok(), Some("/bin/sh"));
}
```

| Config Key                  | Value                          | Preset Defaults (`1` / **`2` Base** / `3`) | Description                                                                                                          |
|:----------------------------|:-------------------------------|:------------------------------------------:|:---------------------------------------------------------------------------------------------------------------------|
| `prefer_stack`              | `bool`                         |       `true` / **`false`** / `false`       | Stack preference for heap/stack coin flip (`true`: 76% stack, `false`: 40% stack)                                    |
| `force_heap`                | `bool` / `Some(bool)` / `None` |        `None` / **`None`** / `None`        | Main container forced routing: `true` forces heap (requires `alloc`), `false` forces stack, `None` inherits defaults |
| `max_latency`               | `u8` (≥ 2)                     |          `36` / **`80`** / `100`           | Per-layer latency cap for cryptographic primitives and storage strategies                                            |
| `min_security`              | `u8` (≤ 99)                    |           `30` / **`63`** / `93`           | Combined security floor (multiplicative accumulation; stops once reached)                                            |
| `security_factor`           | `f64` (> 0.0, finite)          |         `1.0` / **`1.0`** / `1.0`          | Per-layer security scaling factor: `< 1.0` discounts per-layer contribution (more layers), `> 1.0` amplifies (fewer) |
| `wrap_pct`                  | `0..=100`                      |           `30` / **`60`** / `85`           | Base probability of non-inline outlining per layer and per chunk (grows exponentially with consecutive inlines)      |
| `junk_pct`                  | `0..=100`                      |           `8` / **`18`** / `35`            | Probability of interleaving high-fidelity junk chunks                                                                |
| `cold_pct`                  | `0..=100`                      |           `10` / **`20`** / `50`           | Probability of attaching `#[cold]` to outlined functions (hot/cold section splitting)                                |
| `pad_short`                 | `bool`                         |       `false` / **`true`** / `true`        | Enforces prepended `pkcs7_16` pre-padding on short chunks (`< 16B`, excluded from layer count & security budget)     |
| `anti_disasm_pct`           | `0..=100`                      |           `45` / **`60`** / `80`           | Probability of injecting anti-disassembly junk instructions (requires `anti-disassembly` feature)                    |
| `anti_disasm_count_range`   | `lo..=hi` (`u8`)               |      `1..=1` / **`1..=2`** / `2..=3`       | Random range for the number of anti-disassembly blocks injected per site                                             |
| `anti_disasm_garbage_range` | `lo..=hi` (`u8`)               |     `1..=5` / **`4..=10`** / `8..=15`      | Base random range for anti-disassembly rogue garbage bytes                                                           |

### Dynamic Key Tier `*_key!`

Decouples the compile-time encryption key from the runtime decryption key so the complete key is never stored in the binary artifact:

```rust
use obfstr2::s_key;

fn get_server_key() -> &'static [u8] { "obfstr2-demo-key".as_bytes() }

fn main() {
    let s = s_key!(
        { build_key: "obfstr2-demo-key", run_key: get_server_key() },
        "hello",
    );
    assert_eq!(&*s, "hello");
}
```

- `build_key` accepts `"..."`, `b"..."`, `[0x12, ...]`, and `env!("VAR")` (automatically registering Cargo environment variable dependencies) and is never written into the binary;
- `run_key` must be a runtime `&[u8]` expression (constant forms such as literals, `as_bytes()`, or `env!` are rejected at compile time); if the runtime key does not match `build_key`, decryption produces garbled output or triggers integrity self-scrubbing (poisoning emulation and symbolic execution states).

### Lightweight Helper Primitives

Lightweight utilities re-exported from `lib-unknown` (performing simple XOR masking and `black_box` recovery, suitable for **non-sensitive constants** or caching hot paths with `LazyLock`):

| Primitive                          | Description                                                                                                               |
|:-----------------------------------|:--------------------------------------------------------------------------------------------------------------------------|
| `const_random!`                    | Compile-time pseudo-random integer; supports `()` (`u64`), `(Type)`, `(Type, "salt")`; usable in `const`/`static` items   |
| `hide_array!` / `hide_array_lazy!` | Compile-time XOR-masked array hiding (recovered on evaluation / recovered on first access via `LazyLock`)                 |
| `lazy::LazyLock` / `OnceLock`      | `no_std`-compatible lazy initialization containers                                                                        |

```rust
use obfstr2::{const_random, hide_array, hide_array_lazy, s_user, lazy::LazyLock, types::str::StackStr};

const SALT: u64 = const_random!(u64, "demo");
static TABLE: LazyLock<[u32; 4]> = hide_array_lazy!(u32, [1, 2, 3, 4]);
// Hot-path caching example: pin to StackStr via force_heap: false to store in a static LazyLock
static HOT_STR: LazyLock<StackStr<16>> = LazyLock::new(|| s_user!({ force_heap: false }, "hot_path_secret!"));

fn main() {
    assert_eq!(&**HOT_STR, "hot_path_secret!");
    assert_eq!(hide_array!(u8, [1, 2, 3]), [1, 2, 3]);
    assert_eq!(TABLE[0], 1);
    let _ = SALT;
}
```

---

## Feature Flags

| Feature                   | Default | Description                                                                                                                                         |
|:--------------------------|:-------:|:----------------------------------------------------------------------------------------------------------------------------------------------------|
| `default`                 |   ✅    | Enables `alloc` and `anti-disassembly` by default.                                                                                                  |
| `alloc`                   |   ✅    | Enables heap containers (`Heap*`) and `s_fmt*!` macros (supports `no_std + alloc`). When disabled, all tiers use `Stack*`.                          |
| `anti-disassembly`        |   ✅    | Enables x86 / x86_64 anti-disassembly inline assembly injection and Red-Zone protection.                                                            |
| `force_heap`              |   ❌    | Forces the main container of all container macros onto the heap (implicitly enables `alloc`); mutually exclusive with `force_stack`.                |
| `force_stack`             |   ❌    | Forces the main container of all container macros onto the stack (compile error if exceeding `STACK_BUDGET`); mutually exclusive with `force_heap`. |
| `budget-1k` ~ `budget-8m` |   ❌    | Stack budget cap: choose from `1k`, `4k` (default), `16k`, `64k`, `256k`, `512k`, `1m`, `4m`, `8m` (highest wins if multiple).                      |

> **Pure-stack bare-metal mode (no `alloc`, no anti-disassembly):** `obfstr2 = { version = "0.5", default-features = false }`  
> *Note: `force_heap` and `force_stack` are mutually exclusive; do not pass `--all-features` directly when running tests (see [Contributing & Development Verification](#contributing--development-verification)).*

---

## Benchmarks

- **Small Payloads (8 B ~ 1 KB)**: `b1!` incurs near-zero runtime overhead at 8B and roughly a hundred microseconds at 1 KB, making it suitable for hot paths together with `LazyLock`; `b2!` and `b3!` scale up runtime cost and compile time proportionally with anti-analysis strength.
- **Large Payloads (256 KB ~ 10 MB)**: Control-flow overhead is rapidly amortized by superlinear chunking as payload size grows, allowing `obfstr2` to **surpass traditional `const`-based schemes across compile speed, binary size, and macro expansion volume**—at 10 MB, `b1!` / `b2!` / `b3!` compile in just **1/33, 1/16, and 1/7** the time of `CasualX obfbytes!` (230.25s), while binary size overhead converges to **9% ~ 41%** (well below the latter's 96%).

|      Payload | Case               | Deobf Time / Call | Cached Build Time | Binary Size (unstripped) |   Expanded Chars |
|-------------:|:-------------------|------------------:|------------------:|-------------------------:|-----------------:|
|      **8 B** | Plaintext baseline |                 — |         245.04 ms |                 475.8 KB |              698 |
|           〃 | CasualX obfbytes!  |           0.08 µs |   232.13 ms (-5%) |            476.4 KB (0%) |      3192 (357%) |
|           〃 | obfstr2 b1!        |           0.00 µs |   343.12 ms (40%) |           523.9 KB (10%) |    20800 (2880%) |
|           〃 | obfstr2 b2!        |          39.38 µs |   383.59 ms (57%) |           548.8 KB (15%) |    49105 (6935%) |
|           〃 | obfstr2 b3!        |         157.72 µs |  977.21 ms (299%) |           635.9 KB (34%) |   82500 (11719%) |
|     **32 B** | Plaintext baseline |                 — |         212.28 ms |                 475.8 KB |              723 |
|           〃 | CasualX obfbytes!  |           0.03 µs |   242.04 ms (14%) |            476.7 KB (0%) |      3524 (387%) |
|           〃 | obfstr2 b1!        |           0.10 µs |   413.66 ms (95%) |           534.4 KB (12%) |    57262 (7820%) |
|           〃 | obfstr2 b2!        |         137.82 µs |  787.11 ms (271%) |           602.1 KB (27%) |  156729 (21578%) |
|           〃 | obfstr2 b3!        |         216.11 µs |     1.13 s (432%) |           676.4 KB (42%) |  227905 (31422%) |
|    **256 B** | Plaintext baseline |                 — |         212.25 ms |                 475.8 KB |              948 |
|           〃 | CasualX obfbytes!  |           0.01 µs |   252.63 ms (19%) |            477.9 KB (0%) |      4644 (390%) |
|           〃 | obfstr2 b1!        |          19.82 µs |  515.39 ms (143%) |           571.0 KB (20%) |    68721 (7149%) |
|           〃 | obfstr2 b2!        |          99.06 µs |  878.01 ms (314%) |           622.4 KB (31%) |  213288 (22399%) |
|           〃 | obfstr2 b3!        |         453.77 µs |     1.52 s (618%) |           761.2 KB (60%) |  458211 (48234%) |
|   **1.0 KB** | Plaintext baseline |                 — |         221.99 ms |                 475.8 KB |             1717 |
|           〃 | CasualX obfbytes!  |           0.00 µs |   262.32 ms (18%) |            478.7 KB (1%) |      8484 (394%) |
|           〃 | obfstr2 b1!        |         118.14 µs |  605.60 ms (173%) |           591.6 KB (24%) |   161442 (9303%) |
|           〃 | obfstr2 b2!        |         374.51 µs |     1.28 s (478%) |           682.2 KB (43%) |  312530 (18102%) |
|           〃 | obfstr2 b3!        |         826.93 µs |     1.90 s (754%) |           810.1 KB (70%) |  657826 (38213%) |
| **256.0 KB** | Plaintext baseline |                 — |         214.49 ms |                 731.9 KB |           262838 |
|           〃 | CasualX obfbytes!  |           0.00 µs |    5.08 s (2267%) |           988.8 KB (35%) |   1314083 (400%) |
|           〃 | obfstr2 b1!        |          28.42 ms |     1.91 s (790%) |            1.09 MB (53%) |   1373076 (422%) |
|           〃 | obfstr2 b2!        |          44.56 ms |    3.84 s (1689%) |            1.35 MB (90%) |   1810578 (589%) |
|           〃 | obfstr2 b3!        |         121.89 ms |    8.19 s (3718%) |           2.08 MB (191%) |  3988707 (1418%) |
|  **1.00 MB** | Plaintext baseline |                 — |         242.33 ms |                  1.46 MB |          1049271 |
|           〃 | CasualX obfbytes!  |           7.00 µs |   19.94 s (8130%) |            2.47 MB (68%) |   5246243 (400%) |
|           〃 | obfstr2 b1!        |         116.08 ms |    3.05 s (1157%) |            1.98 MB (35%) |   3987224 (280%) |
|           〃 | obfstr2 b2!        |         211.41 ms |    5.48 s (2162%) |            2.31 MB (58%) |   4596155 (338%) |
|           〃 | obfstr2 b3!        |         501.65 ms |   11.12 s (4487%) |           3.42 MB (134%) |   7137882 (580%) |
| **10.00 MB** | Plaintext baseline |                 — |         504.14 ms |                 10.46 MB |         10486455 |
|           〃 | CasualX obfbytes!  |           5.07 ms | 230.25 s (45573%) |           20.47 MB (96%) |  52432162 (400%) |
|           〃 | obfstr2 b1!        |            1.21 s |    6.84 s (1256%) |            11.39 MB (9%) |  31275657 (198%) |
|           〃 | obfstr2 b2!        |            1.77 s |   14.33 s (2742%) |           12.33 MB (18%) |  36207718 (245%) |
|           〃 | obfstr2 b3!        |            4.60 s |   31.55 s (6158%) |           14.75 MB (41%) |  45779509 (337%) |

<details>
<summary>Click to view benchmark parameters and reproduction instructions</summary>

Data is generated from `tests/perf.rs` and can be reproduced via `cargo test --test perf -- --ignored --nocapture` (single polymorphic builds fluctuate slightly; values are for order-of-magnitude reference).
- **Test Parameters**: Covers 7 payload gradients from `8 B` to `10.00 MB` under `profile = release`; decrypted return values and checksums are passed through `black_box` to prevent loop-invariant code motion and dead-code elimination, taking the median across repeated runs.
- **Deobf Time / Call**: `(total_row_time - plaintext_baseline_time) / iterations`, canceling out process startup and outer-loop fixed overhead.
- **Cached Build Time**: Incremental compile time triggered by rewriting only the target source file with a shared, pre-warmed dependency cache.
</details>

---

## Security

> **Disclaimer**: This project is intended strictly for legitimate and compliant use cases such as commercial software IP protection, anti-reverse-engineering research, and firmware security hardening. Use in malware development or unauthorized penetration testing is strictly prohibited.

- **Platform Support**: Pure Rust implementation supporting mainstream desktop OSes and `no_std` bare-metal targets (verified on `x86_64-unknown-none`; Wasm is theoretically supported but not included in CI).
- **Unsafe Boundaries**: The procedural macro validates all length and alignment invariants at compile time; runtime `unsafe` is limited to `unwrap_unchecked` at verified sites to eliminate redundant branches, plus volatile memory overwrites inside container `Drop` implementations.
- **Vulnerability Disclosure**: If you discover a security issue, please report it privately via [GitHub Private Vulnerability Reporting](https://github.com/AlbusPhos/obfstr2/security/advisories/new) or email `owo.zzz@proton.me`. **Please do not** open a public issue.

---

## Contributing & Development Verification

Issues and Pull Requests are welcome! Adding a new obfuscation primitive or storage strategy only requires appending an entry to the static registries in `obfstr2-macros/src/crypto.rs` or `storage.rs`, without touching the core orchestration pipeline.

<details>
<summary>Click to expand local test matrix and PR checklist</summary>

**Standard Test & Calibration Suite:**
```bash
# Core functionality, compile-time rejection, and bare-metal no_std verification (requires x86_64-unknown-none target)
cargo test --test smoke --test s_fmt --test compile_fail --test nostd
cd obfstr2-macros && cargo test --lib

# Performance benchmarks and primitive latency calibration
cargo test --test perf -- --ignored --nocapture
cd obfstr2-macros && cargo test --lib test_storages_latency -- --ignored --nocapture
cd obfstr2-macros && cargo test --lib tests::crypto::test_cryptos_latency -- --ignored --nocapture

# Automated adversarial RE regression tests (requires a Python virtualenv with angr and rust-deobf)
source /path/to/.venv/bin/activate
cargo test --manifest-path obfstr2-macros/Cargo.toml --lib tests::crypto::test_cryptos_security -- --ignored --nocapture
cargo test --test angr -- main --ignored --nocapture
cargo test --test rust_deobf -- main --ignored --nocapture
```

**Pre-Submission Linting & Formatting Checks:**
```bash
cargo fmt --all -- --check
cd obfstr2-macros && cargo fmt --all -- --check
cargo clippy --all-targets --features alloc,anti-disassembly -- -D warnings
cargo clippy --all-targets --features force_heap,anti-disassembly -- -D warnings
cargo clippy --all-targets --features alloc,force_stack,anti-disassembly -- -D warnings
cd obfstr2-macros && cargo clippy --all-targets --features alloc,anti-disassembly -- -D warnings

# Optional: run feature powerset smoke tests (requires cargo-hack)
cargo hack test --feature-powerset \
  --include-features alloc,force_heap,force_stack,budget-4m \
  --mutually-exclusive-features force_heap,force_stack \
  --test smoke
```
</details>

---

## License

This project is licensed under the [MPL-2.0](./LICENSE) License. See [CHANGELOG.md](CHANGELOG.md) for release history.