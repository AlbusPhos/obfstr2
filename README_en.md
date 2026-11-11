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
obfstr2 = "0.3"
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

## Design Philosophy

### Core Principles

1. **Extreme Polymorphism** — The exact same input produces completely different ciphertext forms on every build:
    - **Random Chunking**: Splits payloads at random increasing steps; chunk boundaries change on every compilation.
    - **Random Primitive Stacking**: Randomly selects and stacks multiple encryption/decryption layers according to the security level until the combined security threshold is met.
    - **Random Storage Disguises**: Ciphertext may appear disguised in various forms such as byte strings, `u64` arrays, `u128` arrays, MAC addresses, UUIDs, IPv6 addresses, or PEM.
    - **Key Steganography**: Keys for each layer are likewise hidden using random storage forms and reconstructed only at runtime; no plaintext key constants appear in the expanded code.
    - **Chaff Injection**: Mixes in junk code and meaningless pseudo-state variables to further disrupt static analysis and data-flow tracking.

   Together, these mechanisms ensure that batch deobfuscation scripts cannot rely on any fixed pattern.

2. **Abstracted Obfuscation Pipeline** — All algorithms converge into two registries: `Crypto { name / enc / dec / support / security / latency / metrics }` (polymorphic encryption/decryption) and `Storage { ast / support / security / latency }` (polymorphic ciphertext storage). `build_obfuscated_bytes` is solely responsible for orchestrating the *chunk → encrypt → store → emit* pipeline. Adding a new algorithm only requires appending a table entry without modifying the pipeline.
3. **Compile-Time Evaluation, Minimal Runtime Dependencies** — Procedural macros expand into self-contained TokenStreams. Obfuscation is completed at compile time, while runtime depends only on `types`, `crypto`, and `encoding` from `lib-unknown`. `no_std` compatible. Invariants verified at expansion time use `unwrap_unchecked`, leaving zero runtime validation overhead.
4. **Symbolic Execution Resistance** — `obfstr2` moves beyond traditional "simple XOR" by introducing **modern symmetric cryptography design principles (Confusion, Diffusion, SPN networks, Avalanche Effect)** into string obfuscation to counter automated symbolic execution tools like `angr` (see reproduction commands in the [Contributing](#contributing) section).

### Trade-offs

| We Chose                      | Over                                        | Reason                                                                                                                               |
|:------------------------------|:--------------------------------------------|:-------------------------------------------------------------------------------------------------------------------------------------|
| Security Strength             | Minimal Binary Size                         | High-strength cascaded decryption and polymorphic expansion inevitably cause code bloat—a necessary cost to block reverse analysis   |
| Non-Deterministic Builds      | Reproducible Builds                         | Ensuring polymorphism is the core defense against automated signature extraction                                                     |
| Practical Engineering Defense | Theoretical Cryptographic Indestructibility | The goal of this project is to drastically raise the cost of attack in reverse analysis, symbolic execution, and reverse engineering |

> While cryptographic security is not guaranteed, modern cryptographic design principles are actively incorporated.

### Non-Goals

- Does not perform control-flow obfuscation or anti-debugging.
- Does not guarantee reproducible builds, nor does it replace strong encryption for storing sensitive data at rest.

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

- Also reuse the byte-obfuscation core: the payload is appended with a trailing `\0` and obfuscated as a whole, then reconstructed at runtime via `try_from(&mut [u8])` (truncating at the first NUL + zeroing the decrypted source buffer).
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
- Scenarios requiring evasion from bulk static string scanning.

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
obfstr2 = { version = "0.3", default-features = false }
```

## Platform & Environment Support

- `no_std` compatible; `Heap*` types require the `default` feature.
- Supported OS: No platform-specific code (pure Rust, works on all major desktop OSes; bare-metal toolchain verified on `x86_64-unknown-none`, see [Contributing](#contributing)).
- WebAssembly: `wasm32-unknown-unknown` / `wasm32-wasi` are theoretically supported but untested.

## Benchmarks

`b1!` has near-zero runtime overhead (on par with the plaintext baseline); `b2!` incurs roughly tens of times more runtime overhead; `b3!` has higher overhead corresponding to stronger anti-analysis resistance. Binary size and build time also increase significantly with higher levels—choose your level based on whether your use case is "runtime-sensitive" or "reversing-cost-sensitive". Detailed metrics and reproduction steps are below.

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

| Case               | Time / Deobf | Total Runtime (512 loops, incl. startup) | Incremental Build   | Cold Build           | Binary Size (unstripped) | Expand Chars    |
|:-------------------|:-------------|:-----------------------------------------|:--------------------|:---------------------|:-------------------------|:----------------|
| Plaintext Baseline | —            | 10.365382ms                              | 171.992052ms        | 414.602076ms         | 445040                   | 1437            |
| CasualX obfbytes!  | -0.10µs      | 10.313586ms (-0%)                        | 211.984155ms (23%)  | 1.546629633s (273%)  | 448072 (1%)              | 7780 (441%)     |
| obfstr2 b1!        | 19.57µs      | 20.383619ms (97%)                        | 686.693935ms (299%) | 5.199780109s (1154%) | 519688 (17%)             | 104605 (7179%)  |
| obfstr2 b2!        | 295.99µs     | 161.912404ms (1462%)                     | 1.619466966s (842%) | 5.734792802s (1283%) | 640912 (44%)             | 284900 (19726%) |
| obfstr2 b3!        | 926.98µs     | 484.979339ms (4579%)                     | 4.180500072s (2331%)| 7.775605694s (1775%) | 846864 (90%)             | 637097 (44235%) |

## Security

> **Disclaimer:** This project is intended strictly for legitimate and compliant use cases such as software security protection, anti-reverse-engineering research, and firmware code protection. Using it for malware development or unauthorized penetration testing is strictly prohibited.

- **Unsafe Code**: Expanded code contains `unwrap_unchecked` (length invariants are verified at macro-expansion time, incurring zero runtime check overhead); container `Drop` implementations zero out plaintext memory via volatile writes.
- If you discover a security vulnerability, please open an Issue directly (this repository currently does not have a private reporting channel).

## Contributing

Issues and Pull Requests are welcome!

**Local Verification:**

- `cargo test --test smoke --test s_fmt`: Correctness tests for all macros, in-process assertions, finishes in milliseconds.
- `cargo run --example showcase`: Comprehensive showcase example.
- `cargo test --test compile_fail`: Compile-time rejection of invalid inputs (isolated `dyntest` projects per case).
- `cargo test --test nostd`: `x86_64-unknown-none` bare-metal toolchain check (`b1` pure stack without alloc + `b2` with heap; requires target installed).
- `cargo test --test perf -- --ignored --nocapture`: Performance comparison report (Plaintext baseline vs. CasualX vs. `b1/b2/b3`, measuring build time / runtime / expand chars / binary size; report-only without assertions, requires network access to fetch `obfstr` and `cargo-expand` installed).
- `cd obfstr2-macros && cargo test --lib`: Pure unit tests and encryption layer selection simulations (milliseconds, automatically skips `#[ignore]` cases).
- `cd obfstr2-macros && cargo test --lib test_storages -- --ignored --nocapture`: Round-trip `dyntest` verification for all storage strategies (builds temporary guest projects).
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