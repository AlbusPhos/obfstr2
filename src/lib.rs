// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::needless_doctest_main)]
#![no_std]
/// 重导出 `lib_unknown::crypto`，提供运行时解密与密码学原语。
pub use lib_unknown::crypto;
/// 重导出 `lib_unknown::encoding`，供展开代码解码 Hex / Base64 载荷。
pub use lib_unknown::encoding;
/// 重导出统一错误类型，便于下游处理容器与解码错误。
pub use lib_unknown::errors;
/// 重导出 `lib_unknown::hash`，供动态密钥宏在展开代码中派生 `ghost_state` 绑定。
pub use lib_unknown::hash;
/// 重导出 `lib_unknown::types`，提供 `StackStr` / `HeapBytes` 等运行时容器类型。
pub use lib_unknown::types;

#[cfg(feature = "anti-disassembly")]
pub use anti_disassembly::anti_disassembly;

/// 重导出 `lib_unknown::const_random`，在编译期生成由构建种子、调用位置与环境共同派生的伪随机整数。
///
/// 支持 `const_random!()`（默认 `u64`）、`const_random!(Type)` 与
/// `const_random!(Type, "salt")` 三种形式，结果可用于 `const` / `static` 初始化。
pub use lib_unknown::const_random;
/// 重导出 `lib_unknown::hide_array`，提供 `const fn` 可用的快速常量隐藏。
///
/// 混淆强度低，开销低。敏感常量请使用 obfstr2 原生宏。
pub use lib_unknown::hide_array;
/// 重导出 `lib_unknown::hide_array_lazy`，提供 `LazyLock` 包裹的快速常量隐藏。
///
/// 混淆强度低，开销低。敏感常量请使用 obfstr2 原生宏。
pub use lib_unknown::hide_array_lazy;
/// 重导出 `lib_unknown::lazy`，提供 `no_std` 可用的 `LazyLock` / `OnceLock`，方便视情况减少热路径开销。
pub use lib_unknown::lazy;

/// 内部过程宏实现库的重导出。
#[doc(hidden)]
pub extern crate obfstr2_macros;

/// 字符串混淆宏（低延迟档）。
///
/// 只接受字符串字面量 `"..."`，展开为求值即得原文的表达式（类型为
/// `StackStr<N>`，请使用类型推断，不要写死类型标注）。
///
/// # Examples
///
/// ```rust
/// use obfstr2::s1;
///
/// let s = s1!("hello");
/// assert_eq!(&*s, "hello");
/// ```
#[macro_export]
macro_rules! s1 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::s1!($($tt)*)
    };
}

/// 字符串混淆宏（均衡档）。
///
/// 只接受字符串字面量 `"..."`。返回的具体字符串类型（`StackStr` /
/// `HeapStr`）同一宏名下可能随编译变化，请使用类型推断。
///
/// # Examples
///
/// ```rust
/// use obfstr2::s2;
///
/// let s = s2!("hello");
/// assert_eq!(&*s, "hello");
/// ```
#[macro_export]
macro_rules! s2 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::s2!($($tt)*)
    };
}

/// 字符串混淆宏（高强度档）。
///
/// 只接受字符串字面量 `"..."`，展开类型规则同 [`s2`](s2!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::s3;
///
/// let s = s3!("hello");
/// assert_eq!(&*s, "hello");
/// ```
#[macro_export]
macro_rules! s3 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::s3!($($tt)*)
    };
}

/// 字节串混淆宏（低延迟档）。
///
/// 接受字节串字面量 `b"..."` 或字节数组 `[0x41, 66, ...]`（元素须为 0..=255 的整数字面量），展开为求值即得原文的字节容器表达式
/// （`StackBytes<N>` / `HeapBytes<N>`，可解引用为 `[u8]`）。
///
/// # Examples
///
/// ```rust
/// use obfstr2::b1;
///
/// let a = b1!(b"abc");
/// let b = b1!([0x61, 98, 99]);
/// assert_eq!(&*a, &*b);
/// assert_eq!(&*a, b"abc");
/// ```
#[macro_export]
macro_rules! b1 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::b1!($($tt)*)
    };
}

/// 字节串混淆宏（均衡档）。
///
/// 接受 `b"..."` 或 `[0x41, 66, ...]`（元素须为 0..=255 的整数字面量）。
///
/// # Examples
///
/// ```rust
/// use obfstr2::b2;
///
/// let a = b2!(b"abc");
/// let b = b2!([0x61, 98, 99]);
/// assert_eq!(&*a, &*b);
/// assert_eq!(&*a, b"abc");
/// ```
#[macro_export]
macro_rules! b2 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::b2!($($tt)*)
    };
}

/// 字节串混淆宏（高强度档）。
///
/// 接受 `b"..."` 或 `[0x41, 66, ...]`（元素须为 0..=255 的整数字面量）。
///
/// # Examples
///
/// ```rust
/// use obfstr2::b3;
///
/// let a = b3!(b"abc");
/// let b = b3!([0x61, 98, 99]);
/// assert_eq!(&*a, &*b);
/// assert_eq!(&*a, b"abc");
/// ```
#[macro_export]
macro_rules! b3 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::b3!($($tt)*)
    };
}

/// `u32` 数组混淆宏（低延迟档）。
///
/// 只接受 `u32` 整数字面量数组（如 `[0xDEAD_BEEF, 1u32]`），展开为求值即得原文的
/// `StackU32<N>` / `HeapU32<N>` 容器表达式（可解引用为 `[u32]`，`N` 为元素个数）。
/// 具体容器类型每次编译可能变化，请使用类型推断。
///
/// # Examples
///
/// ```rust
/// use obfstr2::d1;
///
/// let words = d1!([0xDEAD_BEEF, 1u32]);
/// assert_eq!(&*words, &[0xDEAD_BEEF, 1u32][..]);
/// ```
#[macro_export]
macro_rules! d1 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::d1!($($tt)*)
    };
}

/// `u32` 数组混淆宏（均衡档）。
///
/// 输入与展开规则同 [`d1`](d1!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::d2;
///
/// let words = d2!([0, 1, 0xFFFF_FFFFu32]);
/// assert_eq!(&*words, &[0, 1, 0xFFFF_FFFFu32][..]);
/// ```
#[macro_export]
macro_rules! d2 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::d2!($($tt)*)
    };
}

/// `u32` 数组混淆宏（高强度档）。
///
/// 输入与展开规则同 [`d1`](d1!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::d3;
///
/// let words = d3!([1u32, 2, 3]);
/// assert_eq!(&*words, &[1u32, 2, 3][..]);
/// ```
#[macro_export]
macro_rules! d3 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::d3!($($tt)*)
    };
}

/// `u64` 数组混淆宏（低延迟档）。
///
/// 只接受 `u64` 整数字面量数组（如 `[0x0123_4567_89AB_CDEF, 1u64]`），展开为求值即得原文的
/// `StackU64<N>` / `HeapU64<N>` 容器表达式（可解引用为 `[u64]`，`N` 为元素个数）。
/// 具体容器类型每次编译可能变化，请使用类型推断。
///
/// # Examples
///
/// ```rust
/// use obfstr2::q1;
///
/// let words = q1!([0x0123_4567_89AB_CDEFu64, 1u64]);
/// assert_eq!(&*words, &[0x0123_4567_89AB_CDEFu64, 1u64][..]);
/// ```
#[macro_export]
macro_rules! q1 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::q1!($($tt)*)
    };
}

/// `u64` 数组混淆宏（均衡档）。
///
/// 输入与展开规则同 [`q1`](q1!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::q2;
///
/// let words = q2!([0, 1, 0xFFFF_FFFF_FFFF_FFFFu64]);
/// assert_eq!(&*words, &[0, 1, 0xFFFF_FFFF_FFFF_FFFFu64][..]);
/// ```
#[macro_export]
macro_rules! q2 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::q2!($($tt)*)
    };
}

/// `u64` 数组混淆宏（高强度档）。
///
/// 输入与展开规则同 [`q1`](q1!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::q3;
///
/// let words = q3!([1u64, 2, 3]);
/// assert_eq!(&*words, &[1u64, 2, 3][..]);
/// ```
#[macro_export]
macro_rules! q3 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::q3!($($tt)*)
    };
}

/// 文件混淆宏（低延迟档）。
///
/// 接受文件路径字面量（如 `"assets/fixture.bin"`，相对于被编译 crate 的
/// `CARGO_MANIFEST_DIR` 解析），编译期读入文件内容并混淆，展开为求值即得
/// 文件原文的字节容器表达式（`StackBytes<N>` / `HeapBytes<N>`）。
/// 需自行保证文件存在；文件缺失或不可读时报编译错误。
///
/// # Examples
///
/// ```rust
/// use obfstr2::f1;
///
/// let b = f1!("assets/fixture.bin");
/// assert_eq!(&*b, b"secret");
/// ```
#[macro_export]
macro_rules! f1 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::f1!($($tt)*)
    };
}

/// 文件混淆宏（均衡档）。
///
/// 输入与展开规则同 [`f1`](f1!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::f2;
///
/// let b = f2!("assets/fixture.bin");
/// assert_eq!(&*b, b"secret");
/// ```
#[macro_export]
macro_rules! f2 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::f2!($($tt)*)
    };
}

/// 文件混淆宏（高强度档）。
///
/// 输入与展开规则同 [`f1`](f1!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::f3;
///
/// let b = f3!("assets/fixture.bin");
/// assert_eq!(&*b, b"secret");
/// ```
#[macro_export]
macro_rules! f3 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::f3!($($tt)*)
    };
}

/// 格式化字符串混淆宏（低延迟档）。
///
/// 首参须为字符串字面量：其中的字面量片段逐个混淆后注入 `format!` 调用，
/// 占位符与后续参数原样保留。返回 `String`，需要调用方有 `alloc`（`no_std` 可用）。
///
/// # Examples
///
/// ```rust
/// use obfstr2::s_fmt1;
///
/// let name = "world";
/// let s = s_fmt1!("hello, {}!", name);
/// assert_eq!(s, "hello, world!");
/// let t = s_fmt1!("hi, {name}!", name = name);
/// assert_eq!(t, "hi, world!");
/// ```
#[macro_export]
macro_rules! s_fmt1 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::s_fmt1!($($tt)*)
    };
}

/// 格式化字符串混淆宏（均衡档）。
///
/// 输入与展开规则同 [`s_fmt1!`](s_fmt1)，字面量片段走 `s2` 档混淆。
///
/// # Examples
///
/// ```rust
/// use obfstr2::s_fmt2;
///
/// let name = "world";
/// let s = s_fmt2!("hello, {}!", name);
/// assert_eq!(s, "hello, world!");
/// ```
#[macro_export]
macro_rules! s_fmt2 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::s_fmt2!($($tt)*)
    };
}

/// 格式化字符串混淆宏（高强度档）。
///
/// 输入与展开规则同 [`s_fmt1!`](s_fmt1)，字面量片段走 `s3` 档混淆。
///
/// # Examples
///
/// ```rust
/// use obfstr2::s_fmt3;
///
/// let name = "world";
/// let s = s_fmt3!("hello, {}!", name);
/// assert_eq!(s, "hello, world!");
/// ```
#[macro_export]
macro_rules! s_fmt3 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::s_fmt3!($($tt)*)
    };
}

/// 整数混淆宏（低延迟档）。
///
/// 只接受整数字面量（如 `42u8`、`-1`、`0xFFu16`，空后缀视为 `i32`），
/// 展开为求值即得原文的裸整数表达式，可直接算术、比较、`let` 绑定传递。
/// 注意：返回裸值，无 `Drop` 自动清零（与 `StackStr` 不同）。
///
/// # Examples
///
/// ```rust
/// use obfstr2::i1;
///
/// assert_eq!(i1!(42u8), 42u8);
/// assert_eq!(i1!(-1), -1i32);
/// assert_eq!(i1!(0xFFu16), 0xFFu16);
/// ```
#[macro_export]
macro_rules! i1 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::i1!($($tt)*)
    };
}

/// 整数混淆宏（均衡档）。
///
/// 输入与展开规则同 [`i1`](i1!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::i2;
///
/// assert_eq!(i2!(-1), -1i32);
/// assert_eq!(i2!(1_000u32), 1_000u32);
/// assert_eq!(i2!(123usize), 123usize);
/// ```
#[macro_export]
macro_rules! i2 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::i2!($($tt)*)
    };
}

/// 整数混淆宏（高强度档）。
///
/// 输入与展开规则同 [`i1`](i1!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::i3;
///
/// assert_eq!(i3!(0xFFu16), 0xFFu16);
/// assert_eq!(i3!(255u8), 255u8);
/// assert_eq!(i3!(-7i16), -7i16);
/// ```
#[macro_export]
macro_rules! i3 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::i3!($($tt)*)
    };
}

/// 浮点混淆宏（低延迟档）。
///
/// 只接受浮点字面量（如 `3.14f32`、`-1.0`、`1e10`，空后缀视为 `f64`），
/// 展开为求值即得原文的裸浮点表达式，可直接算术、比较、`let` 绑定传递。
/// 仅接受有限常规值：`inf` / `NaN` 一律拒绝；`-0.0` 按位保留符号位。
/// 注意：返回裸值，无 `Drop` 自动清零（与 `StackStr` 不同）。
///
/// # Examples
///
/// ```rust
/// use obfstr2::fl1;
///
/// assert_eq!(fl1!(1.5f32).to_bits(), 1.5f32.to_bits());
/// assert_eq!(fl1!(-0.0f32).to_bits(), (-0.0f32).to_bits());
/// ```
#[macro_export]
macro_rules! fl1 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::fl1!($($tt)*)
    };
}

/// 浮点混淆宏（均衡档）。
///
/// 输入与展开规则同 [`fl1`](fl1!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::fl2;
///
/// assert_eq!(fl2!(3.15).to_bits(), 3.15f64.to_bits());
/// assert_eq!(fl2!(1e10).to_bits(), 1e10f64.to_bits());
/// assert_eq!(fl2!(5f32).to_bits(), 5f32.to_bits());
/// ```
#[macro_export]
macro_rules! fl2 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::fl2!($($tt)*)
    };
}

/// 浮点混淆宏（高强度档）。
///
/// 输入与展开规则同 [`fl1`](fl1!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::fl3;
///
/// assert_eq!(fl3!(-0.0).to_bits(), (-0.0f64).to_bits());
/// assert_eq!(fl3!(2.5f64).to_bits(), 2.5f64.to_bits());
/// ```
#[macro_export]
macro_rules! fl3 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::fl3!($($tt)*)
    };
}

/// C 字符串混淆宏（低延迟档）。
///
/// 接受字符串字面量 `"..."`、C 字符串字面量 `c"..."` 或字节串字面量
/// `b"..."`（后者可表达非 UTF-8 载荷；三者语义等价，`b"..."` 不校验 UTF-8）。
/// 载荷不能包含内部 NUL（宏会追加唯一的结尾 `\0`）。
/// 展开为求值即得原文的 C 字符串容器表达式（`StackCStr<N>`，`N` 含结尾 `\0`，
/// 请使用类型推断），可解引用为 `core::ffi::CStr`、`as_ptr()` 直投系统调用，
/// `Drop` 时自动清零。
///
/// # Examples
///
/// ```rust
/// use obfstr2::cs1;
///
/// let a = cs1!("/bin/sh");
/// let b = cs1!(c"/bin/sh");
/// let c = cs1!(b"/bin/sh");
/// assert_eq!(&*a, c"/bin/sh");
/// assert_eq!(&*b, c"/bin/sh");
/// assert_eq!(&*c, c"/bin/sh");
/// ```
#[macro_export]
macro_rules! cs1 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::cs1!($($tt)*)
    };
}

/// C 字符串混淆宏（均衡档）。
///
/// 输入与展开规则同 [`cs1`](cs1!())；返回的具体容器类型（`StackCStr` /
/// `HeapCStr`）同一宏名下可能随编译变化，请使用类型推断。
///
/// # Examples
///
/// ```rust
/// use obfstr2::cs2;
///
/// let a = cs2!("/bin/sh");
/// let b = cs2!(c"/bin/sh");
/// let c = cs2!(b"/bin/sh");
/// assert_eq!(&*a, c"/bin/sh");
/// assert_eq!(&*b, c"/bin/sh");
/// assert_eq!(&*c, c"/bin/sh");
/// ```
#[macro_export]
macro_rules! cs2 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::cs2!($($tt)*)
    };
}

/// C 字符串混淆宏（高强度档）。
///
/// 输入与展开规则同 [`cs1`](cs1!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::cs3;
///
/// let a = cs3!("/bin/sh");
/// let b = cs3!(c"/bin/sh");
/// let c = cs3!(b"/bin/sh");
/// assert_eq!(&*a, c"/bin/sh");
/// assert_eq!(&*b, c"/bin/sh");
/// assert_eq!(&*c, c"/bin/sh");
/// ```
#[macro_export]
macro_rules! cs3 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::cs3!($($tt)*)
    };
}

/// 宽 C 字符串混淆宏（低延迟档）。
///
/// 只接受字符串字面量 `"..."`，编译期经 `encode_utf16` 展开为 UTF-16 码元，
/// 自动追加唯一结尾 `0u16`；载荷不能包含内部 NUL（`U+0000`）。
/// 展开为求值即得原文的宽字符容器表达式（`StackU16<N>`，`N` 为码元数且含结尾
/// `0u16`，请使用类型推断），`Deref<Target = [u16]>`，`as_ptr()` 可直接传给
/// `LoadLibraryW` / `GetModuleHandleW` 等 Win32 `PCWSTR` 参数，`Drop` 时自动清零。
///
/// # Examples
///
/// ```rust
/// use obfstr2::cw1;
///
/// let w = cw1!("hello");
/// assert_eq!(&w[..w.len() - 1], "hello".encode_utf16().collect::<Vec<_>>().as_slice());
/// assert_eq!(w[w.len() - 1], 0);
/// ```
#[macro_export]
macro_rules! cw1 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::cw1!($($tt)*)
    };
}

/// 宽 C 字符串混淆宏（均衡档）。
///
/// 输入与展开规则同 [`cw1`](cw1!())；返回的具体容器类型（`StackU16` /
/// `HeapU16`）同一宏名下可能随编译变化，请使用类型推断。
///
/// # Examples
///
/// ```rust
/// use obfstr2::cw2;
///
/// let w = cw2!("你好🦀");
/// let text = String::from_utf16(&w[..w.len() - 1]).unwrap();
/// assert_eq!(text, "你好🦀");
/// assert_eq!(w[w.len() - 1], 0);
/// ```
#[macro_export]
macro_rules! cw2 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::cw2!($($tt)*)
    };
}

/// 宽 C 字符串混淆宏（高强度档）。
///
/// 输入与展开规则同 [`cw1`](cw1!())；启用 `alloc` 时主容器走 `HeapU16`。
///
/// # Examples
///
/// ```rust
/// use obfstr2::cw3;
///
/// let w = cw3!("kernel32.dll");
/// assert_eq!(&w[..w.len() - 1], "kernel32.dll".encode_utf16().collect::<Vec<_>>().as_slice());
/// assert_eq!(w[w.len() - 1], 0);
/// ```
#[macro_export]
macro_rules! cw3 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::cw3!($($tt)*)
    };
}

/// 宽字符串混淆宏（低延迟档）。
///
/// 只接受字符串字面量 `"..."`，编译期经 `encode_utf16` 展开为 UTF-16 码元，
/// **不追加**结尾 `0u16`，且 **不限制内部 NUL**（`U+0000` 作为普通码元保留）。
/// `N` 即码元数，空串为 `N = 0`。
/// 展开为求值即得原文的宽字符容器表达式（`StackU16<N>`，请使用类型推断），
/// `Deref<Target = [u16]>`，`Drop` 时自动清零。
///
/// 返回值不保证以 NUL 结尾；需要 NUL 结尾的 Win32 `PCWSTR` 请使用
/// [`cw1`](cw1!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::w1;
///
/// let w = w1!("hello");
/// assert_eq!(&*w, "hello".encode_utf16().collect::<Vec<_>>().as_slice());
/// assert_eq!(w.len(), 5);
/// assert!(w1!("").is_empty());
/// ```
#[macro_export]
macro_rules! w1 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::w1!($($tt)*)
    };
}

/// 宽字符串混淆宏（均衡档）。
///
/// 输入与展开规则同 [`w1`](w1!())；返回的具体容器类型（`StackU16` /
/// `HeapU16`）同一宏名下可能随编译变化，请使用类型推断。
///
/// # Examples
///
/// ```rust
/// use obfstr2::w2;
///
/// let w = w2!("a\0b");
/// assert_eq!(&*w, &[0x61u16, 0, 0x62][..]);
/// ```
#[macro_export]
macro_rules! w2 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::w2!($($tt)*)
    };
}

/// 宽字符串混淆宏（高强度档）。
///
/// 输入与展开规则同 [`w1`](w1!())；启用 `alloc` 时主容器走 `HeapU16`。
///
/// # Examples
///
/// ```rust
/// use obfstr2::w3;
///
/// let w = w3!("kernel32.dll");
/// assert_eq!(&*w, "kernel32.dll".encode_utf16().collect::<Vec<_>>().as_slice());
/// ```
#[macro_export]
macro_rules! w3 {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::w3!($($tt)*)
    };
}

// ── `*_user` 自定义档：`({ 配置 }, 载荷)` ──────────────────────────────────

/// 字符串混淆宏（自定义档）。
///
/// 首参数为花括号配置块 `{ key: val, ... }`，次参数为字符串字面量 `"..."`。
/// 配置以均衡档（`s2`）为基底逐字段覆盖，展开类型规则同 [`s2`](s2!())，请使用类型推断。
///
/// 可用配置键（键支持 `key: val` 或 `key = val`，逗号分隔，允许尾随逗号，重复键报错）：
///
/// | 键 | 取值 | 说明 |
/// |---|---|---|
/// | `prefer_stack` | `bool` | 堆 / 栈掷币的栈偏好概率 |
/// | `force_heap` | `true` / `false` / `Some(bool)` / `None` | 主容器强制路由：`true` 堆、`false` 栈、`None` 继承特性与默认启发式；仅主容器生效 |
/// | `max_latency` | `u8`（≥ 2） | 单层延迟上限 |
/// | `min_security` | `u8`（≤ 99） | 组合安全度下限（乘法累积，达到即停） |
/// | `security_factor` | `f64`（> 0.0 的有限值） | 单层安全度缩放系数：`< 1.0` 折减单层贡献（层数更多），`> 1.0` 放大（层数更少） |
/// | `wrap_pct` | `0..=100` | 逐层非内联包装概率 |
/// | `junk_pct` | `0..=100` | 垃圾块填充概率 |
/// | `cold_pct` | `0..=100` | 冷数据延迟注入概率 |
/// | `pad_short` | `bool` | 短块（< 16B）强制 PKCS7 预填充 |
/// | `anti_disasm_pct` | `0..=100` | 反反汇编注入概率（需启用 `anti-disassembly` 特性） |
/// | `anti_disasm_count_range` | `lo..=hi`（u8） | 反反汇编块数随机区间 |
/// | `anti_disasm_garbage_range` | `lo..=hi`（u8） | 反反汇编垃圾字节基准随机区间 |
///
/// `force_heap` 的优先级为：`*_user!` 配置 > Cargo 特性（`force_heap` / `force_stack`）> 默认启发式；
/// `false` 表示强制走栈，而非关闭强制堆。分块中间容器始终按预算与 `prefer_stack` 启发式路由。
///
/// # Warning
///
/// 本族宏为实验性 API：配置键集合可能随版本增删，依赖前请阅读 CHANGELOG。
///
/// # Examples
///
/// ```rust
/// use obfstr2::s_user;
///
/// let s = s_user!({ max_latency: 50, min_security: 70 }, "hello");
/// assert_eq!(&*s, "hello");
///
/// // 空配置块等价于均衡档 s2!
/// let t = s_user!({}, "hello");
/// assert_eq!(&*t, "hello");
///
/// // 多行书写支持配置块内与载荷末尾的尾随逗号
/// let u = s_user!(
///     {
///         prefer_stack: true,
///         junk_pct: 10,
///     },
///     "hello",
/// );
/// assert_eq!(&*u, "hello");
/// ```
#[macro_export]
macro_rules! s_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::s_user!($($tt)*)
    };
}

/// 字节串混淆宏（自定义档）。
///
/// 首参数为花括号配置块 `{ key: val, ... }`（配置键见 [`s_user`](s_user!())），
/// 次参数为字节串字面量 `b"..."` 或字节数组 `[0x41, 66, ...]`（元素须为 0..=255
/// 的整数字面量），展开为求值即得原文的字节容器表达式
/// （`StackBytes<N>` / `HeapBytes<N>`，可解引用为 `[u8]`）。
///
/// # Examples
///
/// ```rust
/// use obfstr2::b_user;
///
/// let a = b_user!({ prefer_stack: true }, b"abc");
/// let b = b_user!({}, [0x61, 98, 99]);
/// assert_eq!(&*a, &*b);
/// assert_eq!(&*a, b"abc");
/// ```
#[macro_export]
macro_rules! b_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::b_user!($($tt)*)
    };
}

/// `u32` 数组混淆宏（自定义档）。
///
/// 首参数为花括号配置块 `{ key: val, ... }`（配置键见 [`s_user`](s_user!())），
/// 次参数为 `u32` 整数字面量数组，展开类型规则同 [`d2`](d2!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::d_user;
///
/// let words = d_user!({ prefer_stack: true }, [1u32, 2, 3]);
/// assert_eq!(&*words, &[1u32, 2, 3][..]);
/// ```
#[macro_export]
macro_rules! d_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::d_user!($($tt)*)
    };
}

/// `u64` 数组混淆宏（自定义档）。
///
/// 首参数为花括号配置块 `{ key: val, ... }`（配置键见 [`s_user`](s_user!())），
/// 次参数为 `u64` 整数字面量数组，展开类型规则同 [`q2`](q2!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::q_user;
///
/// let words = q_user!({ prefer_stack: true }, [1u64, 2, 3]);
/// assert_eq!(&*words, &[1u64, 2, 3][..]);
/// ```
#[macro_export]
macro_rules! q_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::q_user!($($tt)*)
    };
}

/// 文件混淆宏（自定义档）。
///
/// 首参数为花括号配置块 `{ key: val, ... }`（配置键见 [`s_user`](s_user!())），
/// 次参数为文件路径字面量（如 `"assets/fixture.bin"`，相对于被编译 crate 的
/// `CARGO_MANIFEST_DIR` 解析），编译期读入文件内容并混淆，展开为求值即得
/// 文件原文的字节容器表达式（`StackBytes<N>` / `HeapBytes<N>`）。
/// 需自行保证文件存在；文件缺失或不可读时报编译错误。
///
/// # Examples
///
/// ```rust
/// use obfstr2::f_user;
///
/// let b = f_user!({ prefer_stack: true }, "assets/fixture.bin");
/// assert_eq!(&*b, b"secret");
/// ```
#[macro_export]
macro_rules! f_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::f_user!($($tt)*)
    };
}

/// 格式化字符串混淆宏（自定义档）。
///
/// 首参数为花括号配置块 `{ key: val, ... }`（配置键见 [`s_user`](s_user!())），
/// 其后与 [`s_fmt1`](s_fmt1!()) 相同：格式串中的字面量片段逐个按自定义档混淆后
/// 注入 `format!` 调用，占位符与后续参数原样保留。返回 `String`，需要调用方有
/// `alloc`（`no_std` 可用）。
///
/// # Examples
///
/// ```rust
/// use obfstr2::s_fmt_user;
///
/// let name = "world";
/// let s = s_fmt_user!({ max_latency: 50 }, "hello, {}!", name);
/// assert_eq!(s, "hello, world!");
/// ```
#[macro_export]
macro_rules! s_fmt_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::s_fmt_user!($($tt)*)
    };
}

/// 整数混淆宏（自定义档）。
///
/// 首参数为花括号配置块 `{ key: val, ... }`（配置键见 [`s_user`](s_user!())），
/// 次参数为整数字面量（如 `42u8`、`-1`、`0xFFu16`，空后缀视为 `i32`），
/// 展开为求值即得原文的裸整数表达式。注意：返回裸值，无 `Drop` 自动清零
/// （与 `StackStr` 不同）。
///
/// # Examples
///
/// ```rust
/// use obfstr2::i_user;
///
/// assert_eq!(i_user!({ max_latency: 50 }, 42u8), 42u8);
/// assert_eq!(i_user!({}, -1), -1i32);
/// assert_eq!(i_user!({ min_security: 70 }, 0xFFu16), 0xFFu16);
/// ```
#[macro_export]
macro_rules! i_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::i_user!($($tt)*)
    };
}

/// 浮点混淆宏（自定义档）。
///
/// 首参数为花括号配置块 `{ key: val, ... }`（配置键见 [`s_user`](s_user!())），
/// 次参数为浮点字面量（如 `3.14f32`、`-1.0`、`1e10`，空后缀视为 `f64`），
/// 展开为求值即得原文的裸浮点表达式。仅接受有限常规值：`inf` / `NaN` 一律拒绝；
/// `-0.0` 按位保留符号位。注意：返回裸值，无 `Drop` 自动清零。
///
/// # Examples
///
/// ```rust
/// use obfstr2::fl_user;
///
/// assert_eq!(fl_user!({ max_latency: 50 }, 1.5f32).to_bits(), 1.5f32.to_bits());
/// assert_eq!(fl_user!({}, -0.0).to_bits(), (-0.0f64).to_bits());
/// ```
#[macro_export]
macro_rules! fl_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::fl_user!($($tt)*)
    };
}

/// C 字符串混淆宏（自定义档）。
///
/// 首参数为花括号配置块 `{ key: val, ... }`（配置键见 [`s_user`](s_user!())），
/// 次参数输入与展开规则同 [`cs2`](cs2!())（接受 `"..."` / `c"..."` / `b"..."`，
/// 载荷禁内部 NUL，宏追加唯一的结尾 `\0`）。
///
/// # Examples
///
/// ```rust
/// use obfstr2::cs_user;
///
/// let a = cs_user!({ prefer_stack: true }, "/bin/sh");
/// let b = cs_user!({}, c"/bin/sh");
/// assert_eq!(&*a, c"/bin/sh");
/// assert_eq!(&*b, c"/bin/sh");
/// ```
#[macro_export]
macro_rules! cs_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::cs_user!($($tt)*)
    };
}

/// 宽字符串混淆宏（自定义档）。
///
/// 首参数为花括号配置块 `{ key: val, ... }`（配置键见 [`s_user`](s_user!())），
/// 次参数输入与展开规则同 [`w2`](w2!())（UTF-16 码元，不追加结尾 NUL，
/// 允许内部 NUL）。
///
/// # Examples
///
/// ```rust
/// use obfstr2::w_user;
///
/// let ws = w_user!({ prefer_stack: true }, "a\0b");
/// assert_eq!(&*ws, &[0x61u16, 0, 0x62][..]);
/// ```
#[macro_export]
macro_rules! w_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::w_user!($($tt)*)
    };
}

/// 宽 C 字符串混淆宏（自定义档）。
///
/// 首参数为花括号配置块 `{ key: val, ... }`（配置键见 [`s_user`](s_user!())），
/// 次参数输入与展开规则同 [`cw2`](cw2!())（UTF-16 码元 + 自动结尾 `0u16`，
/// 载荷禁内部 NUL）。
///
/// # Examples
///
/// ```rust
/// use obfstr2::cw_user;
///
/// let w = cw_user!({ min_security: 70 }, "kernel32.dll");
/// assert_eq!(&w[..w.len() - 1], "kernel32.dll".encode_utf16().collect::<Vec<_>>().as_slice());
/// assert_eq!(w[w.len() - 1], 0);
/// ```
#[macro_export]
macro_rules! cw_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::cw_user!($($tt)*)
    };
}

// ── `*_key` / `*_key_user` 动态密钥档 ──────────────────────────────────────

/// 字符串混淆宏（动态密钥档）。
///
/// 语法：`s_key!({ build_key: <编译期密钥>, run_key: <运行期表达式> }, "...")`。
///
/// - `build_key` 只参与编译期加密，不会以明文形式写入产物；支持 `"..."`、
///   `b"..."`、`[0x12, 0x34, ...]` 与 `env!("VAR")`（编译期读取环境变量并注册
///   Cargo 环境依赖）。
/// - `run_key` 必须是运行期可得、类型为 `&[u8]` 的表达式（如 `key.as_bytes()`、`&buf[..]`
///   或 `&vec`）。字面量与编译期常量形式会被拒绝，以避免密钥重新硬编码进二进制。
/// - 运行期 `run_key` 必须与 `build_key` 逐字节一致；不一致时输出未定义（可能触发
///   解密链的完整性擦除），这是针对模拟 / 符号 / 切片执行的对抗设计。请使用高熵
///   随机密钥，不要使用口令。
///
/// 展开类型规则同 [`s2`](s2!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::s_key;
///
/// let key = String::from("obfstr2-demo-key");
/// let s = s_key!({ build_key: "obfstr2-demo-key", run_key: key.as_bytes() }, "hello");
/// assert_eq!(&*s, "hello");
/// ```
#[macro_export]
macro_rules! s_key {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::s_key!($($tt)*)
    };
}

/// 字节串混淆宏（动态密钥档）。
///
/// 语法：`b_key!({ build_key, run_key }, b"..." | [0x41, ...])`；
/// 密钥规则见 [`s_key`](s_key!())，展开类型规则同 [`b2`](b2!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::b_key;
///
/// let key = String::from("obfstr2-demo-key");
/// let b = b_key!({ build_key: "obfstr2-demo-key", run_key: key.as_bytes() }, b"abc");
/// assert_eq!(&*b, b"abc");
/// ```
#[macro_export]
macro_rules! b_key {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::b_key!($($tt)*)
    };
}

/// `u32` 数组混淆宏（动态密钥档）。
///
/// 语法：`d_key!({ build_key, run_key }, [0xDEAD_BEEF, 1u32])`；
/// 密钥规则见 [`s_key`](s_key!())，展开类型规则同 [`d2`](d2!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::d_key;
///
/// let key = String::from("obfstr2-demo-key");
/// let words = d_key!({ build_key: "obfstr2-demo-key", run_key: key.as_bytes() }, [1u32, 2, 3]);
/// assert_eq!(&*words, &[1u32, 2, 3][..]);
/// ```
#[macro_export]
macro_rules! d_key {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::d_key!($($tt)*)
    };
}

/// `u64` 数组混淆宏（动态密钥档）。
///
/// 语法：`q_key!({ build_key, run_key }, [0x0123_4567_89AB_CDEF, 1u64])`；
/// 密钥规则见 [`s_key`](s_key!())，展开类型规则同 [`q2`](q2!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::q_key;
///
/// let key = String::from("obfstr2-demo-key");
/// let words = q_key!({ build_key: "obfstr2-demo-key", run_key: key.as_bytes() }, [1u64, 2, 3]);
/// assert_eq!(&*words, &[1u64, 2, 3][..]);
/// ```
#[macro_export]
macro_rules! q_key {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::q_key!($($tt)*)
    };
}

/// 文件混淆宏（动态密钥档）。
///
/// 语法：`f_key!({ build_key, run_key }, "path")`；路径相对于被编译 crate 的
/// `CARGO_MANIFEST_DIR` 解析。密钥规则见 [`s_key`](s_key!())，展开类型规则同
/// [`f2`](f2!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::f_key;
///
/// let key = String::from("obfstr2-demo-key");
/// let b = f_key!({ build_key: "obfstr2-demo-key", run_key: key.as_bytes() }, "assets/fixture.bin");
/// assert_eq!(&*b, b"secret");
/// ```
#[macro_export]
macro_rules! f_key {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::f_key!($($tt)*)
    };
}

/// 格式化字符串混淆宏（动态密钥档）。
///
/// 语法：`s_fmt_key!({ build_key, run_key }, "fmt {}", args...)`；格式串中的
/// 字面量片段逐个以动态密钥档混淆后注入 `format!`。返回 `String`，需要 `alloc`。
/// 注意：每个字面量片段都会独立求值一次 `run_key` 表达式。
///
/// # Examples
///
/// ```rust
/// use obfstr2::s_fmt_key;
///
/// let key = String::from("obfstr2-demo-key");
/// let name = "world";
/// let s = s_fmt_key!(
///     { build_key: "obfstr2-demo-key", run_key: key.as_bytes() },
///     "hello, {}!",
///     name
/// );
/// assert_eq!(s, "hello, world!");
/// ```
#[macro_export]
macro_rules! s_fmt_key {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::s_fmt_key!($($tt)*)
    };
}

/// 整数混淆宏（动态密钥档）。
///
/// 语法：`i_key!({ build_key, run_key }, <int_lit>)`；展开类型规则同
/// [`i2`](i2!())，返回裸值，无 `Drop` 自动清零。
///
/// # Examples
///
/// ```rust
/// use obfstr2::i_key;
///
/// let key = String::from("obfstr2-demo-key");
/// assert_eq!(i_key!({ build_key: "obfstr2-demo-key", run_key: key.as_bytes() }, 42u8), 42u8);
/// ```
#[macro_export]
macro_rules! i_key {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::i_key!($($tt)*)
    };
}

/// 浮点混淆宏（动态密钥档）。
///
/// 语法：`fl_key!({ build_key, run_key }, <float_lit>)`；展开类型规则同
/// [`fl2`](fl2!())，返回裸值，无 `Drop` 自动清零。
///
/// # Examples
///
/// ```rust
/// use obfstr2::fl_key;
///
/// let key = String::from("obfstr2-demo-key");
/// let v = fl_key!({ build_key: "obfstr2-demo-key", run_key: key.as_bytes() }, 1.5f32);
/// assert_eq!(v.to_bits(), 1.5f32.to_bits());
/// ```
#[macro_export]
macro_rules! fl_key {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::fl_key!($($tt)*)
    };
}

/// C 字符串混淆宏（动态密钥档）。
///
/// 语法：`cs_key!({ build_key, run_key }, "..." | c"..." | b"...")`；展开类型规则
/// 同 [`cs2`](cs2!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::cs_key;
///
/// let key = String::from("obfstr2-demo-key");
/// let c = cs_key!({ build_key: "obfstr2-demo-key", run_key: key.as_bytes() }, c"/bin/sh");
/// assert_eq!(c.to_str().ok(), Some("/bin/sh"));
/// ```
#[macro_export]
macro_rules! cs_key {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::cs_key!($($tt)*)
    };
}

/// 宽字符串混淆宏（动态密钥档）。
///
/// 语法：`w_key!({ build_key, run_key }, "...")`；输入与展开规则同
/// [`w2`](w2!())。密钥规则见 [`s_key`](s_key!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::w_key;
///
/// let key = String::from("obfstr2-demo-key");
/// let ws = w_key!({ build_key: "obfstr2-demo-key", run_key: key.as_bytes() }, "a\0b");
/// assert_eq!(&*ws, &[0x61u16, 0, 0x62][..]);
/// ```
#[macro_export]
macro_rules! w_key {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::w_key!($($tt)*)
    };
}

/// 宽 C 字符串混淆宏（动态密钥档）。
///
/// 语法：`cw_key!({ build_key, run_key }, "...")`；输入与展开规则同
/// [`cw2`](cw2!())。密钥规则见 [`s_key`](s_key!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::cw_key;
///
/// let key = String::from("obfstr2-demo-key");
/// let w = cw_key!({ build_key: "obfstr2-demo-key", run_key: key.as_bytes() }, "kernel32.dll");
/// assert_eq!(&w[..w.len() - 1], "kernel32.dll".encode_utf16().collect::<Vec<_>>().as_slice());
/// assert_eq!(w[w.len() - 1], 0);
/// ```
#[macro_export]
macro_rules! cw_key {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::cw_key!($($tt)*)
    };
}

/// 字符串混淆宏（动态密钥 + 自定义档）。
///
/// 语法：`s_key_user!({ build_key, run_key }, { key: val, ... }, "...")`。
/// 配置块以均衡档为基底，配置键见 [`s_user`](s_user!())；密钥规则见
/// [`s_key`](s_key!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::s_key_user;
///
/// let key = String::from("obfstr2-demo-key");
/// let s = s_key_user!(
///     { build_key: "obfstr2-demo-key", run_key: key.as_bytes() },
///     { max_latency: 50 },
///     "hello"
/// );
/// assert_eq!(&*s, "hello");
/// ```
#[macro_export]
macro_rules! s_key_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::s_key_user!($($tt)*)
    };
}

/// 字节串混淆宏（动态密钥 + 自定义档）。
///
/// 语法：`b_key_user!({ build_key, run_key }, { key: val, ... }, b"..." | [...])`；
/// 配置键见 [`s_user`](s_user!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::b_key_user;
///
/// let key = String::from("obfstr2-demo-key");
/// let b = b_key_user!(
///     { build_key: "obfstr2-demo-key", run_key: key.as_bytes() },
///     { prefer_stack: true },
///     b"abc"
/// );
/// assert_eq!(&*b, b"abc");
/// ```
#[macro_export]
macro_rules! b_key_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::b_key_user!($($tt)*)
    };
}

/// `u32` 数组混淆宏（动态密钥 + 自定义档）。
///
/// 语法：`d_key_user!({ build_key, run_key }, { key: val, ... }, [1u32, 2, 3])`；
/// 配置键见 [`s_user`](s_user!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::d_key_user;
///
/// let key = String::from("obfstr2-demo-key");
/// let words = d_key_user!(
///     { build_key: "obfstr2-demo-key", run_key: key.as_bytes() },
///     { prefer_stack: true },
///     [1u32, 2, 3]
/// );
/// assert_eq!(&*words, &[1u32, 2, 3][..]);
/// ```
#[macro_export]
macro_rules! d_key_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::d_key_user!($($tt)*)
    };
}

/// `u64` 数组混淆宏（动态密钥 + 自定义档）。
///
/// 语法：`q_key_user!({ build_key, run_key }, { key: val, ... }, [1u64, 2, 3])`；
/// 配置键见 [`s_user`](s_user!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::q_key_user;
///
/// let key = String::from("obfstr2-demo-key");
/// let words = q_key_user!(
///     { build_key: "obfstr2-demo-key", run_key: key.as_bytes() },
///     { prefer_stack: true },
///     [1u64, 2, 3]
/// );
/// assert_eq!(&*words, &[1u64, 2, 3][..]);
/// ```
#[macro_export]
macro_rules! q_key_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::q_key_user!($($tt)*)
    };
}

/// 文件混淆宏（动态密钥 + 自定义档）。
///
/// 语法：`f_key_user!({ build_key, run_key }, { key: val, ... }, "path")`；
/// 配置键见 [`s_user`](s_user!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::f_key_user;
///
/// let key = String::from("obfstr2-demo-key");
/// let b = f_key_user!(
///     { build_key: "obfstr2-demo-key", run_key: key.as_bytes() },
///     { prefer_stack: true },
///     "assets/fixture.bin"
/// );
/// assert_eq!(&*b, b"secret");
/// ```
#[macro_export]
macro_rules! f_key_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::f_key_user!($($tt)*)
    };
}

/// 格式化字符串混淆宏（动态密钥 + 自定义档）。
///
/// 语法：`s_fmt_key_user!({ build_key, run_key }, { key: val, ... }, "fmt {}", args...)`。
/// 配置键见 [`s_user`](s_user!())；每个字面量片段独立求值 `run_key`。
///
/// # Examples
///
/// ```rust
/// use obfstr2::s_fmt_key_user;
///
/// let key = String::from("obfstr2-demo-key");
/// let name = "world";
/// let s = s_fmt_key_user!(
///     { build_key: "obfstr2-demo-key", run_key: key.as_bytes() },
///     { max_latency: 50 },
///     "hello, {}!",
///     name
/// );
/// assert_eq!(s, "hello, world!");
/// ```
#[macro_export]
macro_rules! s_fmt_key_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::s_fmt_key_user!($($tt)*)
    };
}

/// 整数混淆宏（动态密钥 + 自定义档）。
///
/// 语法：`i_key_user!({ build_key, run_key }, { key: val, ... }, <int_lit>)`；
/// 配置键见 [`s_user`](s_user!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::i_key_user;
///
/// let key = String::from("obfstr2-demo-key");
/// assert_eq!(
///     i_key_user!({ build_key: "obfstr2-demo-key", run_key: key.as_bytes() }, { max_latency: 50 }, 42u8),
///     42u8
/// );
/// ```
#[macro_export]
macro_rules! i_key_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::i_key_user!($($tt)*)
    };
}

/// 浮点混淆宏（动态密钥 + 自定义档）。
///
/// 语法：`fl_key_user!({ build_key, run_key }, { key: val, ... }, <float_lit>)`；
/// 配置键见 [`s_user`](s_user!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::fl_key_user;
///
/// let key = String::from("obfstr2-demo-key");
/// let v = fl_key_user!(
///     { build_key: "obfstr2-demo-key", run_key: key.as_bytes() },
///     { max_latency: 50 },
///     1.5f32
/// );
/// assert_eq!(v.to_bits(), 1.5f32.to_bits());
/// ```
#[macro_export]
macro_rules! fl_key_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::fl_key_user!($($tt)*)
    };
}

/// C 字符串混淆宏（动态密钥 + 自定义档）。
///
/// 语法：`cs_key_user!({ build_key, run_key }, { key: val, ... }, "..." | c"..." | b"...")`；
/// 配置键见 [`s_user`](s_user!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::cs_key_user;
///
/// let key = String::from("obfstr2-demo-key");
/// let c = cs_key_user!(
///     { build_key: "obfstr2-demo-key", run_key: key.as_bytes() },
///     { prefer_stack: true },
///     c"/bin/sh"
/// );
/// assert_eq!(c.to_str().ok(), Some("/bin/sh"));
/// ```
#[macro_export]
macro_rules! cs_key_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::cs_key_user!($($tt)*)
    };
}

/// 宽字符串混淆宏（动态密钥 + 自定义档）。
///
/// 语法：`w_key_user!({ build_key, run_key }, { key: val, ... }, "...")`；
/// 配置键见 [`s_user`](s_user!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::w_key_user;
///
/// let key = String::from("obfstr2-demo-key");
/// let ws = w_key_user!(
///     { build_key: "obfstr2-demo-key", run_key: key.as_bytes() },
///     { prefer_stack: true },
///     "a\0b"
/// );
/// assert_eq!(&*ws, &[0x61u16, 0, 0x62][..]);
/// ```
#[macro_export]
macro_rules! w_key_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::w_key_user!($($tt)*)
    };
}

/// 宽 C 字符串混淆宏（动态密钥 + 自定义档）。
///
/// 语法：`cw_key_user!({ build_key, run_key }, { key: val, ... }, "...")`；
/// 配置键见 [`s_user`](s_user!())。
///
/// # Examples
///
/// ```rust
/// use obfstr2::cw_key_user;
///
/// let key = String::from("obfstr2-demo-key");
/// let w = cw_key_user!(
///     { build_key: "obfstr2-demo-key", run_key: key.as_bytes() },
///     { min_security: 70 },
///     "kernel32.dll"
/// );
/// assert_eq!(&w[..w.len() - 1], "kernel32.dll".encode_utf16().collect::<Vec<_>>().as_slice());
/// assert_eq!(w[w.len() - 1], 0);
/// ```
#[macro_export]
macro_rules! cw_key_user {
    ($($tt:tt)*) => {
        ::obfstr2::obfstr2_macros::cw_key_user!($($tt)*)
    };
}
