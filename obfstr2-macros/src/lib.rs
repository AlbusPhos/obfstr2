// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `obfstr2` 的内部过程宏实现 crate，无运行时，不可 doctest。
//!
//! 对外暴露 `s1~3!`、`b1~3!`、`f1~3!`、`i1~3!`、`fl1~3!`、`cs1~3!`、`w1~3!`、`cw1~3!`（低延迟 / 均衡 / 高强度三档）、`s_fmt!`，
//! 以及 `*_user!` 自定义档与 `*_key!` / `*_key_user!` 动态密钥档的过程宏本体。
//!
//! # Warning
//!
//! 本 crate 文档仅面向实现者，用户请直接依赖 `obfstr2`，使用根 crate 同名宏（`macro_rules` 包装）并以其文档与可运行示例为准。
#![cfg_attr(docsrs, feature(doc_cfg))]

mod core;
mod crypto;
mod fmt;
mod parse;
mod storage;
mod types;

#[cfg(test)]
mod tests;
mod utils;

use proc_macro::TokenStream;

fn err_to_compile_error(span: proc_macro2::Span, msg: String) -> TokenStream {
    syn::Error::new(span, msg).to_compile_error().into()
}

fn expand_str(
    input: TokenStream,
    build: fn(String) -> Result<proc_macro2::TokenStream, String>,
) -> TokenStream {
    match parse::parse_str(input.into()) {
        Ok(s) => match build(s) {
            Ok(ts) => ts.into(),
            Err(e) => err_to_compile_error(proc_macro2::Span::call_site(), e),
        },
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_bytes(
    input: TokenStream,
    build: fn(Vec<u8>) -> Result<proc_macro2::TokenStream, String>,
) -> TokenStream {
    match parse::parse_bytes(input.into()) {
        Ok(bytes) => match build(bytes) {
            Ok(ts) => ts.into(),
            Err(e) => err_to_compile_error(proc_macro2::Span::call_site(), e),
        },
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_int(
    input: TokenStream,
    build: fn(parse::ParsedInt) -> Result<proc_macro2::TokenStream, String>,
) -> TokenStream {
    match parse::parse_int(input.into()) {
        Ok(parsed) => match build(parsed) {
            Ok(ts) => ts.into(),
            Err(e) => err_to_compile_error(proc_macro2::Span::call_site(), e),
        },
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_float(
    input: TokenStream,
    build: fn(parse::ParsedFloat) -> Result<proc_macro2::TokenStream, String>,
) -> TokenStream {
    match parse::parse_float(input.into()) {
        Ok(parsed) => match build(parsed) {
            Ok(ts) => ts.into(),
            Err(e) => err_to_compile_error(proc_macro2::Span::call_site(), e),
        },
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_cstr(
    input: TokenStream,
    build: fn(Vec<u8>) -> Result<proc_macro2::TokenStream, String>,
) -> TokenStream {
    match parse::parse_cstr(input.into()) {
        Ok(payload) => match build(payload) {
            Ok(ts) => ts.into(),
            Err(e) => err_to_compile_error(proc_macro2::Span::call_site(), e),
        },
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_cwstr(
    input: TokenStream,
    build: fn(parse::ParsedCWide) -> Result<proc_macro2::TokenStream, String>,
) -> TokenStream {
    match parse::parse_cwstr(input.into()) {
        Ok(parsed) => match build(parsed) {
            Ok(ts) => ts.into(),
            Err(e) => err_to_compile_error(proc_macro2::Span::call_site(), e),
        },
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_wstr(
    input: TokenStream,
    build: fn(parse::ParsedWide) -> Result<proc_macro2::TokenStream, String>,
) -> TokenStream {
    match parse::parse_wstr(input.into()) {
        Ok(parsed) => match build(parsed) {
            Ok(ts) => ts.into(),
            Err(e) => err_to_compile_error(proc_macro2::Span::call_site(), e),
        },
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_file(
    input: TokenStream,
    build: fn(Vec<u8>) -> Result<proc_macro2::TokenStream, String>,
) -> TokenStream {
    match parse::parse_file(input.into()) {
        Ok(data) => match build(data) {
            Ok(ts) => ts.into(),
            Err(e) => err_to_compile_error(proc_macro2::Span::call_site(), e),
        },
        Err(e) => e.to_compile_error().into(),
    }
}
/// 字符串混淆宏本体（低延迟档）：只接受字符串字面量 `"..."`，展开为求值即得原文的表达式。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn s1(input: TokenStream) -> TokenStream {
    expand_str(input, types::s1)
}

/// 字符串混淆宏本体（均衡档）：只接受字符串字面量 `"..."`；返回的具体字符串类型（`StackStr` /
/// `HeapStr`）同一宏名下可能随编译变化。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn s2(input: TokenStream) -> TokenStream {
    expand_str(input, types::s2)
}

/// 字符串混淆宏本体（高强度档）：只接受字符串字面量 `"..."`，展开类型规则同 `s2`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn s3(input: TokenStream) -> TokenStream {
    expand_str(input, types::s3)
}

/// 字节串混淆宏本体（低延迟档）：接受字节串字面量 `b"..."` 或字节数组 `[0x41, 66, ...]`
/// （元素须为 0..=255 的整数字面量），展开为求值即得原文的字节容器表达式。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn b1(input: TokenStream) -> TokenStream {
    expand_bytes(input, types::b1)
}

/// 字节串混淆宏本体（均衡档）：接受 `b"..."` 或 `[0x41, 66, ...]`（元素须为 0..=255 的整数字面量）。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn b2(input: TokenStream) -> TokenStream {
    expand_bytes(input, types::b2)
}

/// 字节串混淆宏本体（高强度档）：接受 `b"..."` 或 `[0x41, 66, ...]`（元素须为 0..=255 的整数字面量）。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn b3(input: TokenStream) -> TokenStream {
    expand_bytes(input, types::b3)
}

/// 文件混淆宏本体（低延迟档）：接受文件路径字面量（相对于被编译 crate 的
/// `CARGO_MANIFEST_DIR` 解析），编译期读入文件内容并混淆；文件缺失或不可读时报编译错误。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn f1(input: TokenStream) -> TokenStream {
    expand_file(input, types::b1)
}

/// 文件混淆宏本体（均衡档）：输入与展开规则同 `f1`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn f2(input: TokenStream) -> TokenStream {
    expand_file(input, types::b2)
}

/// 文件混淆宏本体（高强度档）：输入与展开规则同 `f1`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn f3(input: TokenStream) -> TokenStream {
    expand_file(input, types::b3)
}

/// 格式化字符串混淆宏本体（低延迟档）：输入与展开规则同 `s_fmt`，字面量片段走 `s1` 档混淆。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/s_fmt.rs`。
#[proc_macro]
pub fn s_fmt1(input: TokenStream) -> TokenStream {
    fmt::sfmt(input.into(), types::s1).into()
}

/// 格式化字符串混淆宏本体（均衡档）：输入与展开规则同 `s_fmt`，字面量片段走 `s2` 档混淆。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/s_fmt.rs`。
#[proc_macro]
pub fn s_fmt2(input: TokenStream) -> TokenStream {
    fmt::sfmt(input.into(), types::s2).into()
}

/// 格式化字符串混淆宏本体（高强度档）：输入与展开规则同 `s_fmt`，字面量片段走 `s3` 档混淆。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/s_fmt.rs`。
#[proc_macro]
pub fn s_fmt3(input: TokenStream) -> TokenStream {
    fmt::sfmt(input.into(), types::s3).into()
}

/// 整数混淆宏本体（低延迟档）：只接受整数字面量（如 `42u8`、`-1`、`0xFFu16`，空后缀视为 `i32`），
/// 展开为求值即得原文的裸整数表达式；返回裸值，无 `Drop` 自动清零。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn i1(input: TokenStream) -> TokenStream {
    expand_int(input, types::i1)
}

/// 整数混淆宏本体（均衡档）：输入与展开规则同 `i1`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn i2(input: TokenStream) -> TokenStream {
    expand_int(input, types::i2)
}

/// 整数混淆宏本体（高强度档）：输入与展开规则同 `i1`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn i3(input: TokenStream) -> TokenStream {
    expand_int(input, types::i3)
}

/// 浮点混淆宏本体（低延迟档）：只接受浮点字面量（如 `3.14f32`、`-1.0`、`1e10`，空后缀视为 `f64`），
/// 展开为求值即得原文的裸浮点表达式；仅接受有限常规值，`-0.0` 按位保留符号位；返回裸值，无 `Drop` 自动清零。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn fl1(input: TokenStream) -> TokenStream {
    expand_float(input, types::fl1)
}

/// 浮点混淆宏本体（均衡档）：输入与展开规则同 `fl1`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn fl2(input: TokenStream) -> TokenStream {
    expand_float(input, types::fl2)
}

/// 浮点混淆宏本体（高强度档）：输入与展开规则同 `fl1`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn fl3(input: TokenStream) -> TokenStream {
    expand_float(input, types::fl3)
}

/// C 字符串混淆宏本体（低延迟档）：接受 `"..."` / `c"..."` / `b"..."` 三种字面量（语义等价，
/// `b"..."` 可表达非 UTF-8 载荷）；载荷禁内部 NUL，宏追加唯一的结尾 `\0`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn cs1(input: TokenStream) -> TokenStream {
    expand_cstr(input, types::cs1)
}

/// C 字符串混淆宏本体（均衡档）：输入与展开规则同 `cs1`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn cs2(input: TokenStream) -> TokenStream {
    expand_cstr(input, types::cs2)
}
/// C 字符串混淆宏本体（高强度档）：输入与展开规则同 `cs1`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn cs3(input: TokenStream) -> TokenStream {
    expand_cstr(input, types::cs3)
}

/// 宽字符串混淆宏本体（低延迟档）：只接受字符串字面量 `"..."`，按 UTF-16 展开，
/// **不追加**结尾 `0u16`，内部 NUL 允许；展开为 `StackU16<N>`（`N` 为码元数，
/// 空串为 0）。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn w1(input: TokenStream) -> TokenStream {
    expand_wstr(input, types::w1)
}

/// 宽字符串混淆宏本体（均衡档）：输入与展开规则同 `w1`；返回容器在
/// `StackU16` / `HeapU16` 间随机。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn w2(input: TokenStream) -> TokenStream {
    expand_wstr(input, types::w2)
}

/// 宽字符串混淆宏本体（高强度档）：输入与展开规则同 `w1`；启用 `alloc`
/// 时主容器走 `HeapU16`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn w3(input: TokenStream) -> TokenStream {
    expand_wstr(input, types::w3)
}

/// 宽 C 字符串混淆宏本体（低延迟档）：只接受字符串字面量 `"..."`，按 UTF-16
/// 展开后追加唯一结尾 `0u16`；载荷禁内部 NUL，展开为 `StackU16<N>`（`N` 含结尾码元）。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn cw1(input: TokenStream) -> TokenStream {
    expand_cwstr(input, types::cw1)
}

/// 宽 C 字符串混淆宏本体（均衡档）：输入与展开规则同 `cw1`；返回容器在
/// `StackU16` / `HeapU16` 间随机。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn cw2(input: TokenStream) -> TokenStream {
    expand_cwstr(input, types::cw2)
}

/// 宽 C 字符串混淆宏本体（高强度档）：输入与展开规则同 `cw1`；启用 `alloc`
/// 时主容器走 `HeapU16`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn cw3(input: TokenStream) -> TokenStream {
    expand_cwstr(input, types::cw3)
}

// ── `*_user` 自定义档：`({ key: val, ... }, 载荷)`，以 TIER_BALANCED 为覆盖底 ──

fn expand_user<T>(
    input: TokenStream,
    parser: fn(proc_macro2::TokenStream) -> syn::Result<(T, core::TierParams)>,
    builder: fn(T, core::TierParams) -> Result<proc_macro2::TokenStream, String>,
) -> TokenStream {
    match parser(input.into()) {
        Ok((parsed, tier)) => match builder(parsed, tier) {
            Ok(ts) => ts.into(),
            Err(e) => err_to_compile_error(proc_macro2::Span::call_site(), e),
        },
        Err(e) => e.to_compile_error().into(),
    }
}

/// 字符串混淆宏本体（自定义档）：接受 `({ key: val, ... }, "...")`，以 `TIER_BALANCED` 为基底。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs` 与本 crate `tests/user.rs`。
#[proc_macro]
pub fn s_user(input: TokenStream) -> TokenStream {
    expand_user(input, parse::parse_str_user, types::s_user)
}

/// 字节串混淆宏本体（自定义档）：接受 `({ key: val, ... }, b"..." | [...])`，以 `TIER_BALANCED` 为基底。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs` 与本 crate `tests/user.rs`。
#[proc_macro]
pub fn b_user(input: TokenStream) -> TokenStream {
    expand_user(input, parse::parse_bytes_user, types::b_user)
}

/// 文件混淆宏本体（自定义档）：接受 `({ key: val, ... }, "path")`，以 `TIER_BALANCED` 为基底。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn f_user(input: TokenStream) -> TokenStream {
    expand_user(input, parse::parse_file_user, types::b_user)
}

/// 格式化字符串混淆宏本体（自定义档）：接受 `({ key: val, ... }, "fmt {}", args...)`，
/// 以 `TIER_BALANCED` 为基底，字面量片段走 `s_user` 档混淆。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/s_fmt.rs`。
#[proc_macro]
pub fn s_fmt_user(input: TokenStream) -> TokenStream {
    match parse::split_user_prefix(input.into(), "s_fmt_user!") {
        Ok((tier, rest)) => fmt::sfmt(rest, |s| types::s_user(s, tier.clone())).into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// 整数混淆宏本体（自定义档）：接受 `({ key: val, ... }, <int_lit>)`，以 `TIER_BALANCED` 为基底。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn i_user(input: TokenStream) -> TokenStream {
    expand_user(input, parse::parse_int_user, types::i_user)
}

/// 浮点混淆宏本体（自定义档）：接受 `({ key: val, ... }, <float_lit>)`，以 `TIER_BALANCED` 为基底。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn fl_user(input: TokenStream) -> TokenStream {
    expand_user(input, parse::parse_float_user, types::fl_user)
}

/// C 字符串混淆宏本体（自定义档）：接受 `({ key: val, ... }, "..." | c"..." | b"...")`，
/// 以 `TIER_BALANCED` 为基底。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn cs_user(input: TokenStream) -> TokenStream {
    expand_user(input, parse::parse_cstr_user, types::cs_user)
}

/// 宽字符串混淆宏本体（自定义档）：接受 `({ key: val, ... }, "...")`，以 `TIER_BALANCED` 为基底。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn w_user(input: TokenStream) -> TokenStream {
    expand_user(input, parse::parse_wstr_user, types::w_user)
}

/// 宽 C 字符串混淆宏本体（自定义档）：接受 `({ key: val, ... }, "...")`，以 `TIER_BALANCED` 为基底。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
/// 用户文档与可运行示例见 `obfstr2` 根 crate，覆盖见根 crate `tests/smoke.rs`。
#[proc_macro]
pub fn cw_user(input: TokenStream) -> TokenStream {
    expand_user(input, parse::parse_cwstr_user, types::cw_user)
}

// ── `*_key` 动态密钥档：`({ build_key, run_key }, 载荷)`，以 TIER_BALANCED 为基底 ──

fn expand_key<T>(
    input: TokenStream,
    macro_name: &str,
    parser: fn(proc_macro2::TokenStream, &str) -> syn::Result<(T, parse::KeyPair)>,
    builder: fn(T, parse::KeyPair) -> Result<proc_macro2::TokenStream, String>,
) -> TokenStream {
    match parser(input.into(), macro_name) {
        Ok((parsed, keys)) => match builder(parsed, keys) {
            Ok(ts) => ts.into(),
            Err(e) => err_to_compile_error(proc_macro2::Span::call_site(), e),
        },
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_key_user<T>(
    input: TokenStream,
    parser: fn(proc_macro2::TokenStream) -> syn::Result<(T, parse::KeyPair, core::TierParams)>,
    builder: fn(T, parse::KeyPair, core::TierParams) -> Result<proc_macro2::TokenStream, String>,
) -> TokenStream {
    match parser(input.into()) {
        Ok((parsed, keys, tier)) => match builder(parsed, keys, tier) {
            Ok(ts) => ts.into(),
            Err(e) => err_to_compile_error(proc_macro2::Span::call_site(), e),
        },
        Err(e) => e.to_compile_error().into(),
    }
}

/// 字符串混淆宏本体（动态密钥档）：接受 `({ build_key: ..., run_key: ... }, "...")`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
#[proc_macro]
pub fn s_key(input: TokenStream) -> TokenStream {
    expand_key(input, "s_key!", parse::parse_str_key, types::s_key)
}

/// 字节串混淆宏本体（动态密钥档）：接受 `({ build_key: ..., run_key: ... }, b"..." | [...])`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
#[proc_macro]
pub fn b_key(input: TokenStream) -> TokenStream {
    expand_key(input, "b_key!", parse::parse_bytes_key, types::b_key)
}

/// 文件混淆宏本体（动态密钥档）：接受 `({ build_key: ..., run_key: ... }, "path")`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
#[proc_macro]
pub fn f_key(input: TokenStream) -> TokenStream {
    expand_key(input, "f_key!", parse::parse_file_key, types::b_key)
}

/// 格式化字符串混淆宏本体（动态密钥档）：接受 `({ build_key: ..., run_key: ... }, "fmt {}", args...)`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
#[proc_macro]
pub fn s_fmt_key(input: TokenStream) -> TokenStream {
    match parse::split_key_prefix(input.into(), "s_fmt_key!") {
        Ok((keys, rest)) => fmt::sfmt(rest, move |s| types::s_key(s, keys.clone())).into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// 整数混淆宏本体（动态密钥档）：接受 `({ build_key: ..., run_key: ... }, <int_lit>)`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
#[proc_macro]
pub fn i_key(input: TokenStream) -> TokenStream {
    expand_key(input, "i_key!", parse::parse_int_key, types::i_key)
}

/// 浮点混淆宏本体（动态密钥档）：接受 `({ build_key: ..., run_key: ... }, <float_lit>)`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
#[proc_macro]
pub fn fl_key(input: TokenStream) -> TokenStream {
    expand_key(input, "fl_key!", parse::parse_float_key, types::fl_key)
}

/// C 字符串混淆宏本体（动态密钥档）：接受 `({ build_key: ..., run_key: ... }, "..." | c"..." | b"...")`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
#[proc_macro]
pub fn cs_key(input: TokenStream) -> TokenStream {
    expand_key(input, "cs_key!", parse::parse_cstr_key, types::cs_key)
}

/// 宽字符串混淆宏本体（动态密钥档）：接受 `({ build_key: ..., run_key: ... }, "...")`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
#[proc_macro]
pub fn w_key(input: TokenStream) -> TokenStream {
    expand_key(input, "w_key!", parse::parse_wstr_key, types::w_key)
}

/// 宽 C 字符串混淆宏本体（动态密钥档）：接受 `({ build_key: ..., run_key: ... }, "...")`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
#[proc_macro]
pub fn cw_key(input: TokenStream) -> TokenStream {
    expand_key(input, "cw_key!", parse::parse_cwstr_key, types::cw_key)
}

/// 字符串混淆宏本体（动态密钥 + 自定义档）：接受 `({ build_key, run_key }, { key: val, ... }, "...")`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
#[proc_macro]
pub fn s_key_user(input: TokenStream) -> TokenStream {
    expand_key_user(input, parse::parse_str_key_user, types::s_key_user)
}

/// 字节串混淆宏本体（动态密钥 + 自定义档）：接受 `({ build_key, run_key }, { key: val, ... }, b"..." | [...])`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
#[proc_macro]
pub fn b_key_user(input: TokenStream) -> TokenStream {
    expand_key_user(input, parse::parse_bytes_key_user, types::b_key_user)
}

/// 文件混淆宏本体（动态密钥 + 自定义档）：接受 `({ build_key, run_key }, { key: val, ... }, "path")`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
#[proc_macro]
pub fn f_key_user(input: TokenStream) -> TokenStream {
    expand_key_user(input, parse::parse_file_key_user, types::b_key_user)
}

/// 格式化字符串混淆宏本体（动态密钥 + 自定义档）：接受 `({ build_key, run_key }, { key: val, ... }, "fmt {}", args...)`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
#[proc_macro]
pub fn s_fmt_key_user(input: TokenStream) -> TokenStream {
    match parse::split_key_user_prefix(input.into(), "s_fmt_key_user!") {
        Ok((keys, tier, rest)) => fmt::sfmt(rest, move |s| {
            types::s_key_user(s, keys.clone(), tier.clone())
        })
        .into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// 整数混淆宏本体（动态密钥 + 自定义档）：接受 `({ build_key, run_key }, { key: val, ... }, <int_lit>)`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
#[proc_macro]
pub fn i_key_user(input: TokenStream) -> TokenStream {
    expand_key_user(input, parse::parse_int_key_user, types::i_key_user)
}

/// 浮点混淆宏本体（动态密钥 + 自定义档）：接受 `({ build_key, run_key }, { key: val, ... }, <float_lit>)`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
#[proc_macro]
pub fn fl_key_user(input: TokenStream) -> TokenStream {
    expand_key_user(input, parse::parse_float_key_user, types::fl_key_user)
}

/// C 字符串混淆宏本体（动态密钥 + 自定义档）：接受 `({ build_key, run_key }, { key: val, ... }, "..." | c"..." | b"...")`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
#[proc_macro]
pub fn cs_key_user(input: TokenStream) -> TokenStream {
    expand_key_user(input, parse::parse_cstr_key_user, types::cs_key_user)
}

/// 宽字符串混淆宏本体（动态密钥 + 自定义档）：接受 `({ build_key, run_key }, { key: val, ... }, "...")`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
#[proc_macro]
pub fn w_key_user(input: TokenStream) -> TokenStream {
    expand_key_user(input, parse::parse_wstr_key_user, types::w_key_user)
}

/// 宽 C 字符串混淆宏本体（动态密钥 + 自定义档）：接受 `({ build_key, run_key }, { key: val, ... }, "...")`。
///
/// # Warning
///
/// 内部实现，外部请使用 `obfstr2` 根 crate 的同名宏；宏本体与展开形态可随版本变更，不做稳定性承诺。
#[proc_macro]
pub fn cw_key_user(input: TokenStream) -> TokenStream {
    expand_key_user(input, parse::parse_cwstr_key_user, types::cw_key_user)
}
