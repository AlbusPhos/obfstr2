// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 输入解析层：各宏字面量输入 → 载荷字节 / 编码结构。
//!
//! 档位适配见 `crate::types`，混淆编排见 `crate::core`。
//! 本文件只保留输入校验（字面量形态、后缀、范围、NUL、文件存在性、密钥合法性），不做加密与发射。
//! 文末为 `*_user!`、`*_key!` 与 `*_key_user!` 的花括号前缀统一解析。

use proc_macro2::TokenStream as TokenStream2;

/// 负号拆分：`42u8` → `(字面量, false)`，`-42u8` → `(字面量, true)`。
///
/// 调用方按自身字面量类型做二次匹配（`int` 要 `Lit::Int`，`float` 要 `Lit::Float`，
/// 另有整数 token 兼容分支）；`what` 为各宏的“只接受……”前缀，错误文案与原内联版本逐字一致。
pub(crate) fn split_neg(expr: syn::Expr, what: &str) -> syn::Result<(syn::Lit, bool)> {
    match unwrap_group(expr) {
        syn::Expr::Lit(syn::ExprLit { lit, .. }) => Ok((lit, false)),
        syn::Expr::Unary(u) if matches!(u.op, syn::UnOp::Neg(_)) => match unwrap_group(*u.expr) {
            syn::Expr::Lit(syn::ExprLit { lit, .. }) => Ok((lit, true)),
            other => Err(syn::Error::new_spanned(
                &other,
                format!("{what}，不支持表达式"),
            )),
        },
        other => Err(syn::Error::new_spanned(
            &other,
            format!("{what}，不支持表达式"),
        )),
    }
}

/// 解析后的整数：小端字节 + 目标类型标记。
pub(crate) struct ParsedInt {
    pub(crate) bytes: Vec<u8>,
    /// 解码表达式的整数类型，如 `u32`；`usize`/`isize` 需经 `u64`/`i64` 中转。
    pub(crate) ty: IntTy,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum IntTy {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    U128,
    I128,
    Usize,
    Isize,
}

impl IntTy {
    fn from_suffix(suffix: &str) -> Option<Self> {
        match suffix {
            "u8" => Some(Self::U8),
            "i8" => Some(Self::I8),
            "u16" => Some(Self::U16),
            "i16" => Some(Self::I16),
            "u32" => Some(Self::U32),
            "i32" => Some(Self::I32),
            "u64" => Some(Self::U64),
            "i64" => Some(Self::I64),
            "u128" => Some(Self::U128),
            "i128" => Some(Self::I128),
            "usize" => Some(Self::Usize),
            "isize" => Some(Self::Isize),
            "" => Some(Self::I32),
            _ => None,
        }
    }

    fn display(&self) -> &'static str {
        match self {
            Self::U8 => "u8",
            Self::I8 => "i8",
            Self::U16 => "u16",
            Self::I16 => "i16",
            Self::U32 => "u32",
            Self::I32 => "i32",
            Self::U64 => "u64",
            Self::I64 => "i64",
            Self::U128 => "u128",
            Self::I128 => "i128",
            Self::Usize => "usize",
            Self::Isize => "isize",
        }
    }

    fn is_signed(&self) -> bool {
        matches!(
            self,
            Self::I8 | Self::I16 | Self::I32 | Self::I64 | Self::I128 | Self::Isize
        )
    }
}

/// 解析 `iN!` 输入：只接受整数字面量（可选前导 `-`），拒绝表达式/路径/函数调用。
pub(crate) fn parse_int(input: TokenStream2) -> syn::Result<ParsedInt> {
    let expr: syn::Expr = syn::parse2(input)?;
    match split_neg(expr, "iN! 只接受整数字面量（如 42u8、-1、0xFFu16)")? {
        (syn::Lit::Int(n), false) => parse_int_positive(&n),
        (syn::Lit::Int(n), true) => parse_int_negative(&n),
        (lit, _) => Err(syn::Error::new_spanned(
            &lit,
            "iN! 只接受整数字面量（如 42u8、-1、0xFFu16），不支持表达式",
        )),
    }
}

fn int_suffix_err(n: &syn::LitInt) -> syn::Error {
    syn::Error::new_spanned(
        n,
        format!(
            "不支持的整数后缀 {:?}，可用：u8/i8/u16/i16/u32/i32/u64/i64/u128/i128/usize/isize（空后缀视为 i32）",
            n.suffix()
        ),
    )
}

fn range_err(n: &syn::LitInt, ty: IntTy) -> syn::Error {
    syn::Error::new_spanned(n, format!("整数字面量超出 {} 范围", ty.display()))
}

/// 正数字面量单臂生成：`$parse` 为实际解析类型（`usize` 按 `u64`、`isize` 按 `i64` 解析，
/// 见模块文档与 README 限制说明）。泛型函数无法表述各整数类型的 `to_le_bytes`（无公共 trait，
/// 为此引入新依赖得不偿失），故用局内宏。
macro_rules! parse_pos {
    ($n:expr, $ty:expr, $parse:ty) => {{
        let v: $parse = $n.base10_parse().map_err(|_| range_err($n, $ty))?;
        v.to_le_bytes().to_vec()
    }};
}

/// 正数字面量：`base10_parse` 天然处理 `0x/0o/0b`/下划线并做范围校验。
fn parse_int_positive(n: &syn::LitInt) -> syn::Result<ParsedInt> {
    let ty = IntTy::from_suffix(n.suffix()).ok_or_else(|| int_suffix_err(n))?;
    let bytes = match ty {
        IntTy::U8 => parse_pos!(n, ty, u8),
        IntTy::I8 => parse_pos!(n, ty, i8),
        IntTy::U16 => parse_pos!(n, ty, u16),
        IntTy::I16 => parse_pos!(n, ty, i16),
        IntTy::U32 => parse_pos!(n, ty, u32),
        IntTy::I32 => parse_pos!(n, ty, i32),
        IntTy::U64 => parse_pos!(n, ty, u64),
        IntTy::I64 => parse_pos!(n, ty, i64),
        IntTy::U128 => parse_pos!(n, ty, u128),
        IntTy::I128 => parse_pos!(n, ty, i128),
        IntTy::Usize => parse_pos!(n, ty, u64),
        IntTy::Isize => parse_pos!(n, ty, i64),
    };
    Ok(ParsedInt { bytes, ty })
}

/// 负数字面量：先按 `u128` 解析绝对值再取负，避免 `-MIN` 这类边界溢出误拒。
fn parse_int_negative(n: &syn::LitInt) -> syn::Result<ParsedInt> {
    let ty = IntTy::from_suffix(n.suffix()).ok_or_else(|| int_suffix_err(n))?;
    if !ty.is_signed() {
        return Err(syn::Error::new_spanned(
            n,
            format!("负数不能用于无符号整数类型 {}", ty.display()),
        ));
    }
    // 绝对值按 `u128` 解析，兼容 `-i128::MIN`（其绝对值超出 `i128::MAX`）。
    let mag: u128 = n.base10_parse().map_err(|_| range_err(n, ty))?;
    // 各有符号类型的绝对值上限 = MAX + 1（允许 MIN）。
    macro_rules! neg_checked {
        ($ty:ty) => {{
            const LIMIT: u128 = (<$ty>::MAX as u128) + 1;
            if mag > LIMIT {
                return Err(range_err(n, ty));
            }
            // `mag == LIMIT` 时 `as $ty` 按二补数得到 MIN，`wrapping_neg(MIN)` 仍为 MIN；
            // 其余情况下得到 `-mag`。无需为 i128 单独特判。
            (mag as $ty).wrapping_neg().to_le_bytes().to_vec()
        }};
    }
    let bytes = match ty {
        IntTy::I8 => neg_checked!(i8),
        IntTy::I16 => neg_checked!(i16),
        IntTy::I32 => neg_checked!(i32),
        IntTy::I64 | IntTy::Isize => neg_checked!(i64),
        IntTy::I128 => neg_checked!(i128),
        _ => unreachable!("无符号类型已在函数入口拒绝"),
    };
    Ok(ParsedInt { bytes, ty })
}

/// 解析后的浮点：小端字节 + 目标类型标记。
pub(crate) struct ParsedFloat {
    pub(crate) bytes: Vec<u8>,
    pub(crate) ty: FloatTy,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FloatTy {
    F32,
    F64,
}

impl FloatTy {
    fn from_suffix(suffix: &str) -> Option<Self> {
        match suffix {
            "f32" => Some(Self::F32),
            "f64" => Some(Self::F64),
            "" => Some(Self::F64),
            _ => None,
        }
    }

    fn display(&self) -> &'static str {
        match self {
            Self::F32 => "f32",
            Self::F64 => "f64",
        }
    }
}

/// 解析 `flN!` 输入：只接受浮点字面量（可选前导 `-`），拒绝表达式/路径/常量。
pub(crate) fn parse_float(input: TokenStream2) -> syn::Result<ParsedFloat> {
    let expr: syn::Expr = syn::parse2(input)?;
    match split_neg(
        expr,
        "flN! 只接受浮点字面量（如 3.14f32、-1.0、1e10，空后缀视为 f64)",
    )? {
        (syn::Lit::Float(n), neg) => parse_lit_float(&n, neg),
        // `5f32` 形态：整数 token + 浮点后缀，syn 可能判为 Lit::Int，引导至浮点处理。
        (syn::Lit::Int(n), neg) if n.suffix() == "f32" || n.suffix() == "f64" => {
            parse_float_from_int_token(&n, neg)
        }
        (lit, _) => Err(syn::Error::new_spanned(
            &lit,
            "flN! 只接受浮点字面量（如 3.14f32、-1.0、1e10，空后缀视为 f64），不支持表达式",
        )),
    }
}

fn float_suffix_err(span: proc_macro2::Span, suffix: &str) -> syn::Error {
    syn::Error::new(
        span,
        format!("不支持的浮点后缀 {suffix:?}，可用：f32/f64（空后缀视为 f64）"),
    )
}

fn non_finite_err(span: proc_macro2::Span, ty: FloatTy) -> syn::Error {
    syn::Error::new(
        span,
        format!(
            "浮点字面量已上溢为无穷（{}），请直接使用 INFINITY 常量而非宏",
            ty.display()
        ),
    )
}

fn nan_err(span: proc_macro2::Span) -> syn::Error {
    syn::Error::new(span, "NaN 不能作为混淆输入（无字面量写法且语义不稳定）")
}

/// 有限性检查 + `to_bits` 小端编码的公共尾段（`f32` / `f64` 均有 `is_finite` / `to_bits`，
/// 宏展开后按具体类型解析；`nan` 式额外拒绝 NaN，供字面量形态使用，整数 token 形态不可能读出 NaN）。
macro_rules! finish_float {
    ($v:expr, $span:expr, $ty:expr) => {{
        let v = $v;
        if !v.is_finite() {
            return Err(non_finite_err($span, $ty));
        }
        Ok(ParsedFloat {
            bytes: v.to_bits().to_le_bytes().to_vec(),
            ty: $ty,
        })
    }};
    ($v:expr, $span:expr, $ty:expr, nan) => {{
        let v = $v;
        if v.is_nan() {
            return Err(nan_err($span));
        }
        if !v.is_finite() {
            return Err(non_finite_err($span, $ty));
        }
        Ok(ParsedFloat {
            bytes: v.to_bits().to_le_bytes().to_vec(),
            ty: $ty,
        })
    }};
}

/// 浮点字面量：`base10_parse` 处理下划线/指数/后缀，`neg` 控制是否取负；
/// 取负后立即 `to_bits`，因此 `-0.0` 的符号位由 `-mag` 精确保留。
fn parse_lit_float(n: &syn::LitFloat, neg: bool) -> syn::Result<ParsedFloat> {
    let ty =
        FloatTy::from_suffix(n.suffix()).ok_or_else(|| float_suffix_err(n.span(), n.suffix()))?;
    match ty {
        FloatTy::F32 => {
            let mag: f32 = n
                .base10_parse()
                .map_err(|e| syn::Error::new(n.span(), format!("浮点字面量解析失败：{e}")))?;
            finish_float!(if neg { -mag } else { mag }, n.span(), ty, nan)
        }
        FloatTy::F64 => {
            let mag: f64 = n
                .base10_parse()
                .map_err(|e| syn::Error::new(n.span(), format!("浮点字面量解析失败：{e}")))?;
            finish_float!(if neg { -mag } else { mag }, n.span(), ty, nan)
        }
    }
}

/// `5f32` / `-5f32` 防御分支：整数 token + 浮点后缀时按浮点解析。
fn parse_float_from_int_token(n: &syn::LitInt, neg: bool) -> syn::Result<ParsedFloat> {
    let ty =
        FloatTy::from_suffix(n.suffix()).ok_or_else(|| float_suffix_err(n.span(), n.suffix()))?;
    // 整数形态无小数点/指数，按对应浮点重解析其十进制数字。
    let digits = n.base10_digits();
    match ty {
        FloatTy::F32 => {
            let mag: f32 = digits
                .parse()
                .map_err(|e| syn::Error::new(n.span(), format!("浮点字面量解析失败：{e}")))?;
            finish_float!(if neg { -mag } else { mag }, n.span(), ty)
        }
        FloatTy::F64 => {
            let mag: f64 = digits
                .parse()
                .map_err(|e| syn::Error::new(n.span(), format!("浮点字面量解析失败：{e}")))?;
            finish_float!(if neg { -mag } else { mag }, n.span(), ty)
        }
    }
}

/// 解析 `csN!` 输入：只接受 `"..."` / `c"..."` / `b"..."` 字面量，拒绝表达式/路径/函数调用。
///
/// 返回不含结尾 `\0` 的载荷字节；载荷中任何位置出现 NUL 即报错并给出下标
/// （宏会追加唯一的结尾 NUL，故任何输入 NUL 都是内部 NUL）。
pub(crate) fn parse_cstr(input: TokenStream2) -> syn::Result<Vec<u8>> {
    let expr: syn::Expr = syn::parse2(input)?;
    let (payload, span) = match unwrap_group(expr) {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(lit),
            ..
        }) => (lit.value().into_bytes(), lit.span()),
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::CStr(lit),
            ..
        }) => (lit.value().as_bytes().to_vec(), lit.span()),
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::ByteStr(lit),
            ..
        }) => (lit.value(), lit.span()),
        other => {
            return Err(syn::Error::new_spanned(
                &other,
                "csN! 只接受字符串/C 字符串/字节串字面量（如 \"hi\"、c\"hi\"、b\"hi\"），不支持表达式",
            ));
        }
    };
    if let Some(pos) = payload.iter().position(|&b| b == 0) {
        return Err(syn::Error::new(
            span,
            format!("C 字符串载荷不能包含内部 NUL（载荷下标 {pos}）"),
        ));
    }
    Ok(payload)
}

/// 解析后的宽字符串：UTF-16 码元序列（不含自动追加的结尾 NUL）。
pub(crate) struct ParsedWide {
    pub(crate) units: Vec<u16>,
}

/// 解析后的宽 C 字符串：UTF-16 码元序列（已含唯一结尾 `0`）。
pub(crate) struct ParsedCWide {
    pub(crate) units: Vec<u16>,
}

/// 解析 `wN!` 输入：只接受字符串字面量 `"..."`，按 UTF-16 展开，不追加结尾 `0u16`。
///
/// 与 `cwN!` 不同：不限制内部 NUL（`U+0000` 作为普通码元保留），也不自动追加
/// 结尾 NUL；`N` 即码元数，空串 `N = 0`。
pub(crate) fn parse_wstr(input: TokenStream2) -> syn::Result<ParsedWide> {
    let (_lit, units) = parse_wide_literal(input, "wN!")?;
    Ok(ParsedWide { units })
}

/// 解析 `cwN!` 输入：只接受字符串字面量 `"..."`，按 UTF-16 展开后追加结尾 `0u16`。
///
/// 载荷中任何位置出现 `U+0000` 即报错并给出码元下标（宏会追加唯一的结尾 NUL，
/// 故任何输入 NUL 都是内部 NUL），与 `csN!` 的 NUL 语义一致。
pub(crate) fn parse_cwstr(input: TokenStream2) -> syn::Result<ParsedCWide> {
    let (lit, mut units) = parse_wide_literal(input, "cwN!")?;
    if let Some(pos) = units.iter().position(|&u| u == 0) {
        return Err(syn::Error::new(
            lit.span(),
            format!("宽 C 字符串载荷不能包含内部 NUL（码元下标 {pos}）"),
        ));
    }
    units.push(0);
    Ok(ParsedCWide { units })
}

/// 宽字符串字面量解析与 UTF-16 编码的共用逻辑；`macro_name` 仅用于错误信息。
fn parse_wide_literal(
    input: TokenStream2,
    macro_name: &str,
) -> syn::Result<(syn::LitStr, Vec<u16>)> {
    let expr: syn::Expr = syn::parse2(input)?;
    let lit = match unwrap_group(expr) {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(lit),
            ..
        }) => lit,
        other => {
            return Err(syn::Error::new_spanned(
                &other,
                format!("{macro_name} 只接受字符串字面量（如 \"hi\"、\"你好\"），不支持表达式"),
            ));
        }
    };
    let units: Vec<u16> = lit.value().encode_utf16().collect();
    Ok((lit, units))
}

/// 解开 `macro_rules!` 元变量（`:literal` / `:expr`）转发产生的不可见 `None` 分组。
///
/// `syn::Expr` 为保留表达式优先级会把它保留为 `Expr::Group`；所有基于 `Expr`
/// 的解析入口（`iN!` / `flN!` / `csN!` / `bN!` / `wN!` / `cwN!`）都必须先解包，
/// 否则用户把宏再包一层 `macro_rules!` 时会误报“不支持表达式”。
fn unwrap_group(expr: syn::Expr) -> syn::Expr {
    let mut expr = expr;
    loop {
        match expr {
            syn::Expr::Group(group) => expr = *group.expr,
            other => return other,
        }
    }
}

/// `unwrap_group` 的引用版本，供数组元素等只需借用、不宜 clone 的场景使用。
fn unwrap_group_ref(mut expr: &syn::Expr) -> &syn::Expr {
    loop {
        match expr {
            syn::Expr::Group(group) => expr = &*group.expr,
            other => return other,
        }
    }
}

/// 解析 `sN!` 输入：只接受字符串字面量 `"..."`。
pub(crate) fn parse_str(input: TokenStream2) -> syn::Result<String> {
    // `LitStr` 属于底层 Token 解析，`syn` 游标会自动穿透 `Delimiter::None`
    // 隐式分组，因此无需像 `syn::Expr` 入口那样显式 `unwrap_group`。
    syn::parse2::<syn::LitStr>(input).map(|lit| lit.value())
}

/// 解析 `[0x41, 66, ...]` 形式的 `u8` 整数字面量数组（供 `bN!` 与 `build_key` 共用）。
fn parse_byte_array_lit(arr: &syn::ExprArray) -> syn::Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(arr.elems.len());
    for elem in &arr.elems {
        match unwrap_group_ref(elem) {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(n),
                ..
            }) => match n.base10_parse::<u8>() {
                Ok(b) => bytes.push(b),
                Err(_) => {
                    return Err(syn::Error::new_spanned(
                        n,
                        "字节数组元素必须在 0..=255 范围内",
                    ));
                }
            },
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "字节数组只接受 0..=255 的整数字面量",
                ));
            }
        }
    }
    Ok(bytes)
}

/// 解析 `bN!` 输入：`b"..."` 字节串或 `[0x41, 66, ...]` 数组（元素须为 0..=255 的整数字面量）。
pub(crate) fn parse_bytes(input: TokenStream2) -> syn::Result<Vec<u8>> {
    let expr: syn::Expr = syn::parse2(input)?;
    match unwrap_group(expr) {
        // 1. b"..." 字节串形式
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::ByteStr(lit),
            ..
        }) => Ok(lit.value()),
        // 2. [0x41, 66, ...] 字节数组形式
        syn::Expr::Array(arr) => parse_byte_array_lit(&arr),
        other => Err(syn::Error::new_spanned(
            &other,
            "bN! 只接受字节串字面量 b\"...\" 或字节数组 [0x41, 66, ...]，不支持表达式",
        )),
    }
}

/// 解析 `fN!` 输入：路径字面量相对被编译 crate 的 `CARGO_MANIFEST_DIR` 读入文件
/// （与 `include_bytes!` 一致）；文件缺失或不可读时报错。
pub(crate) fn parse_file(input: TokenStream2) -> syn::Result<Vec<u8>> {
    let path_lit: syn::LitStr = syn::parse2(input)?;
    let rel = path_lit.value();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let full = std::path::Path::new(&manifest_dir).join(&rel);
    std::fs::read(&full).map_err(|e| {
        syn::Error::new(
            path_lit.span(),
            format!("无法读取文件 {}: {e}", full.display()),
        )
    })
}

// ── `({ ... }, 载荷)` 与 `({ ... }, { ... }, 载荷)` 通用花括号前缀解析骨架 ─────

use crate::core::{TIER_BALANCED, TierParams};
use proc_macro2::{Delimiter, TokenTree};
use std::collections::HashSet;
use std::ops::RangeInclusive;
use syn::parse::{Parse, ParseStream, Parser};

/// 解开 `macro_rules!` 元变量转发（`:literal` / `:expr`）产生的不可见 `None` 分组。
///
/// 与 `unwrap_group` 不同：本函数作用于裸 `TokenTree`（花括号块前缀拆解需在
/// `syn::Expr` 解析之前识别花括号分组，不能走 `syn::Expr` 入口）。
fn unwrap_none_tt(mut tt: TokenTree) -> TokenTree {
    while let TokenTree::Group(g) = &tt {
        if g.delimiter() == Delimiter::None {
            let mut iter = g.stream().into_iter();
            if let (Some(inner), None) = (iter.next(), iter.next()) {
                tt = inner;
                continue;
            }
        }
        break;
    }
    tt
}

/// 校验花括号块之后的逗号 `,` 并收集剩余载荷 TokenStream。
fn split_payload_after_brace(
    mut iter: proc_macro2::token_stream::IntoIter,
    macro_name: &str,
    block_name: &str,
    usage: &str,
) -> syn::Result<TokenStream2> {
    match iter.next() {
        Some(TokenTree::Punct(p)) if p.as_char() == ',' => {}
        Some(other) => {
            return Err(syn::Error::new_spanned(
                other,
                format!("{macro_name} {block_name} `{{ ... }}` 后缺少逗号 `,`"),
            ));
        }
        None => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("{macro_name} 缺少载荷参数，用法：{usage}"),
            ));
        }
    }

    let rest: TokenStream2 = iter.collect();
    if rest.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("{macro_name} 逗号后缺少字面量载荷参数"),
        ));
    }

    Ok(rest)
}

/// 提取单花括号块前缀 `({ ... }, <rest...>)`，供 `*_user!` 与 `*_key!` 共用。
fn split_braced_prefix<T>(
    input: TokenStream2,
    macro_name: &str,
    block_name: &str,
    block_syntax: &str,
    usage: &str,
    parse_block: impl FnOnce(TokenStream2, proc_macro2::Span) -> syn::Result<T>,
) -> syn::Result<(T, TokenStream2)> {
    let mut iter = input.into_iter();
    let Some(first) = iter.next() else {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("{macro_name} 缺少参数，用法：{usage}"),
        ));
    };

    let first = unwrap_none_tt(first);
    let brace_group = match first {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => g,
        other => {
            return Err(syn::Error::new_spanned(
                other,
                format!("{macro_name} 第一个参数必须是花括号{block_name} `{block_syntax}`"),
            ));
        }
    };

    let parsed = parse_block(brace_group.stream(), brace_group.span())?;
    let rest = split_payload_after_brace(iter, macro_name, block_name, usage)?;
    Ok((parsed, rest))
}

/// 剥离单载荷宏末尾可选的尾随逗号（如 `s_user!({ ... }, "hi",)`）。
fn strip_trailing_comma(rest: TokenStream2) -> TokenStream2 {
    let mut tokens: Vec<TokenTree> = rest.into_iter().collect();
    if tokens.len() > 1
        && let Some(TokenTree::Punct(p)) = tokens.last()
        && p.as_char() == ','
    {
        tokens.pop();
    }
    tokens.into_iter().collect()
}

/// 解析花括号内 `key: val` 或 `key = val` 序列的通用骨架：统一处理重复键、分隔符与尾随逗号。
fn parse_kv_fields(
    stream: TokenStream2,
    mut on_field: impl FnMut(&syn::Ident, &str, ParseStream) -> syn::Result<()>,
    on_finish: impl FnOnce(&HashSet<String>, ParseStream) -> syn::Result<()>,
) -> syn::Result<()> {
    (|input: ParseStream| {
        let mut seen = HashSet::new();

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            let key_str = key.to_string();

            if !seen.insert(key_str.clone()) {
                return Err(syn::Error::new_spanned(
                    &key,
                    format!("重复配置字段 `{key_str}`"),
                ));
            }

            // 同时兼容 `key: val` 与 `key = val`
            if input.peek(syn::Token![:]) {
                input.parse::<syn::Token![:]>()?;
            } else if input.peek(syn::Token![=]) {
                input.parse::<syn::Token![=]>()?;
            } else {
                return Err(input.error("字段名后应为 `:` 或 `=`"));
            }

            on_field(&key, &key_str, input)?;

            // 允许逗号分隔及尾随逗号
            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            } else if !input.is_empty() {
                return Err(input.error("配置项之间缺少逗号 `,`"));
            }
        }

        on_finish(&seen, input)
    })
    .parse2(stream)
}

// ── `*_user!` 自定义档：`({ key: val, ... }, 载荷)` 解析 ─────────────────────

/// 提取所有 `*_user!` 宏的公共前缀：`({ key: val, ... }, <rest...>)` → `(TierParams, rest)`。
///
/// `s_fmt_user!` 直接使用返回的 `rest` 作为格式串与变长参数流（尾随逗号由
/// `sfmt` 的 `FormatArgs` 语义处理）；其余单载荷宏通过 [`split_user_input`]
/// 进一步剥离可选的末尾逗号。
pub(crate) fn split_user_prefix(
    input: TokenStream2,
    macro_name: &str,
) -> syn::Result<(TierParams, TokenStream2)> {
    let usage = format!("{macro_name}({{ key: val, ... }}, <字面量>)");
    split_braced_prefix(
        input,
        macro_name,
        "配置块",
        "{ key: val, ... }",
        &usage,
        parse_tier_overrides,
    )
}

/// 拆分单载荷宏 `*_user!({ <overrides> }, <payload> $(,)?)` 的顶层输入。
///
/// 返回 `(载荷 TokenStream, 覆盖后的 TierParams)`。允许多行书写时载荷末尾
/// 带一个可选的尾随逗号：`s_user!({ ... }, "hi",)`。
pub(crate) fn split_user_input(
    input: TokenStream2,
    macro_name: &str,
) -> syn::Result<(TokenStream2, TierParams)> {
    let (tier, rest) = split_user_prefix(input, macro_name)?;
    Ok((strip_trailing_comma(rest), tier))
}

/// 解析花括号内部的 `key: value` 或 `key = value` 序列，以 `TIER_BALANCED` 为底覆盖。
fn parse_tier_overrides(
    stream: TokenStream2,
    brace_span: proc_macro2::Span,
) -> syn::Result<TierParams> {
    let mut tier = TIER_BALANCED;

    parse_kv_fields(
        stream,
        |key, key_str, input| {
            match key_str {
                "prefer_stack" => {
                    tier.prefer_stack = input.parse::<syn::LitBool>()?.value();
                }
                "pad_short" => {
                    tier.pad_short = input.parse::<syn::LitBool>()?.value();
                }
                "max_latency" => {
                    tier.max_latency = parse_u8_field(input, "max_latency")?;
                }
                "min_security" => {
                    tier.min_security = parse_u8_field(input, "min_security")?;
                }
                "security_factor" => {
                    tier.security_factor = parse_f64_field(input, "security_factor")?;
                }
                "wrap_pct" => {
                    tier.wrap_pct = parse_u8_field(input, "wrap_pct")?;
                }
                "junk_pct" => {
                    tier.junk_pct = parse_u8_field(input, "junk_pct")?;
                }
                "cold_pct" => {
                    tier.cold_pct = parse_u8_field(input, "cold_pct")?;
                }
                "anti_disasm_pct" => {
                    tier.anti_disasm_pct = parse_u8_field(input, "anti_disasm_pct")?;
                }
                "anti_disasm_count_range" => {
                    tier.anti_disasm_count_range =
                        parse_u8_inclusive_range(input, "anti_disasm_count_range")?;
                }
                "anti_disasm_garbage_range" => {
                    tier.anti_disasm_garbage_range =
                        parse_u8_inclusive_range(input, "anti_disasm_garbage_range")?;
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        key,
                        format!(
                            "未知的 TierParams 字段 `{key_str}`，可用字段：\
                             prefer_stack, max_latency, min_security, security_factor, \
                             wrap_pct, junk_pct, cold_pct, pad_short, anti_disasm_pct, \
                             anti_disasm_count_range, anti_disasm_garbage_range"
                        ),
                    ));
                }
            }
            Ok(())
        },
        |seen, input| {
            // 反反汇编注入依赖 `anti-disassembly` 特性：未启用时相关配置是静默 no-op，
            // 显式配置只会造成误解，直接拒绝。
            if !cfg!(feature = "anti-disassembly")
                && seen.iter().any(|k| k.starts_with("anti_disasm"))
            {
                return Err(input.error(
                    "当前构建未启用 anti-disassembly 特性，不能配置 anti_disasm_* 字段（请在 obfstr2 上启用默认特性后重试）",
                ));
            }
            Ok(())
        },
    )?;

    tier.validate()
        .map_err(|e| syn::Error::new(brace_span, e))?;
    Ok(tier)
}

fn parse_u8_field(input: ParseStream, field: &str) -> syn::Result<u8> {
    let lit: syn::LitInt = input
        .parse()
        .map_err(|_| input.error(format!("`{field}` 需要 0..=255 的无符号整数字面量")))?;
    lit.base10_parse::<u8>()
        .map_err(|_| syn::Error::new_spanned(&lit, format!("`{field}` 超出 u8 (0..=255) 范围")))
}

/// 解析 f64 字段：兼容 `1.5` 浮点字面量与 `2` 整数字面量两种写法
/// （syn 将 `5f32` 记为整数 token，`peek(syn::LitFloat)` 无法可靠区分，故统一走 `syn::Lit`）。
fn parse_f64_field(input: ParseStream, field: &str) -> syn::Result<f64> {
    let lit: syn::Lit = input.parse().map_err(|_| {
        input.error(format!(
            "`{field}` 需要正的有限浮点数字面量（如 1.0、0.5、2）"
        ))
    })?;
    let v = match &lit {
        syn::Lit::Float(f) => f
            .base10_parse::<f64>()
            .map_err(|_| syn::Error::new_spanned(&lit, format!("`{field}` 不是合法浮点数")))?,
        syn::Lit::Int(i) => i
            .base10_parse::<f64>()
            .map_err(|_| syn::Error::new_spanned(&lit, format!("`{field}` 不是合法数字")))?,
        _ => {
            return Err(syn::Error::new_spanned(
                &lit,
                format!("`{field}` 需要正的有限浮点数字面量（如 1.0、0.5、2）"),
            ));
        }
    };
    Ok(v)
}

fn parse_u8_inclusive_range(input: ParseStream, field: &str) -> syn::Result<RangeInclusive<u8>> {
    let lo_lit: syn::LitInt = input
        .parse()
        .map_err(|_| input.error(format!("`{field}` 需要闭区间字面量（如 `1..=3`）")))?;
    let dots: syn::Token![..=] = input.parse().map_err(|_| {
        input.error(format!(
            "`{field}` 必须使用闭区间运算符 `..=`（如 `1..=3`）"
        ))
    })?;
    let hi_lit: syn::LitInt = input
        .parse()
        .map_err(|_| input.error(format!("`{field}` 的 `..=` 后缺少上界整数字面量")))?;

    let lo = lo_lit.base10_parse::<u8>().map_err(|_| {
        syn::Error::new_spanned(&lo_lit, format!("`{field}` 下界超出 u8 (0..=255) 范围"))
    })?;
    let hi = hi_lit.base10_parse::<u8>().map_err(|_| {
        syn::Error::new_spanned(&hi_lit, format!("`{field}` 上界超出 u8 (0..=255) 范围"))
    })?;

    if lo > hi {
        return Err(syn::Error::new_spanned(
            dots,
            format!("`{field}` 范围下界 ({lo}) 不能大于上界 ({hi})"),
        ));
    }
    Ok(lo..=hi)
}

// ── 动态密钥档：`*_key!` 与 `*_key_user!` 解析 ──────────────────────────────

/// 解析后的双密钥配置：编译期加密密钥 + 运行期解密密钥表达式。
#[derive(Clone)]
pub(crate) struct KeyPair {
    /// 编译期用于加密载荷的实际密钥字节（非空）。
    pub(crate) build_key: Vec<u8>,
    /// 若 `build_key` 来自 `env!("VAR")`，记录其环境变量名字面量。
    ///
    /// 代码发射层应生成 `const _: usize = ::core::env!(#var).len();` 以注册 Cargo
    /// 环境变量依赖，避免生成 `&str` 常量导致密钥明文残留在 `.rodata` 段中。
    pub(crate) build_key_env: Option<syn::LitStr>,
    /// 运行期用于绑定 `let key: &[u8] = #run_key;` 的表达式（已拒绝字面量及字面量衍生形式）。
    pub(crate) run_key: syn::Expr,
}

/// 提取 `*_key!` 宏的公共前缀：`({ build_key: ..., run_key: ... }, <rest...>)` → `(KeyPair, rest)`。
pub(crate) fn split_key_prefix(
    input: TokenStream2,
    macro_name: &str,
) -> syn::Result<(KeyPair, TokenStream2)> {
    let usage =
        format!("{macro_name}({{ build_key: \"...\", run_key: <运行期表达式> }}, <字面量>)");
    split_braced_prefix(
        input,
        macro_name,
        "密钥块",
        "{ build_key: ..., run_key: ... }",
        &usage,
        |stream, span| parse_key_pair(stream, span, macro_name),
    )
}

/// 拆分单载荷动态密钥宏 `*_key!({ build_key: ..., run_key: ... }, <payload> $(,)?)` 的顶层输入。
pub(crate) fn split_key_input(
    input: TokenStream2,
    macro_name: &str,
) -> syn::Result<(TokenStream2, KeyPair)> {
    let (keys, rest) = split_key_prefix(input, macro_name)?;
    Ok((strip_trailing_comma(rest), keys))
}

/// 提取 `*_key_user!` 宏的双花括号前缀（密钥块在前，用户配置块在后）：
///
/// 同时兼容两种写法：
/// - 逗号分隔：`macro!({ build_key: ..., run_key: ... }, { key: val, ... }, <rest...>)`
/// - 紧邻花括号：`macro!({ build_key: ..., run_key: ... } { key: val, ... }, <rest...>)`
pub(crate) fn split_key_user_prefix(
    input: TokenStream2,
    macro_name: &str,
) -> syn::Result<(KeyPair, TierParams, TokenStream2)> {
    let usage = format!(
        "{macro_name}({{ build_key: \"...\", run_key: <运行期表达式> }}, {{ key: val, ... }}, <字面量>)"
    );
    let mut iter = input.into_iter();

    // 1. 解析第一个花括号块：密钥配置 `{ build_key, run_key }`
    let Some(first) = iter.next() else {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("{macro_name} 缺少参数，用法：{usage}"),
        ));
    };
    let first = unwrap_none_tt(first);
    let key_group = match first {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => g,
        other => {
            return Err(syn::Error::new_spanned(
                other,
                format!(
                    "{macro_name} 第一个参数必须是花括号密钥块 `{{ build_key: ..., run_key: ... }}`"
                ),
            ));
        }
    };
    let keys = parse_key_pair(key_group.stream(), key_group.span(), macro_name)?;

    // 2. 解析第二个花括号块：用户配置 `{ key: val, ... }`（允许两花括号之间带或不带逗号 `,`）
    let Some(next_tt) = iter.next() else {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("{macro_name} 缺少配置块与载荷参数，用法：{usage}"),
        ));
    };
    let second = match next_tt {
        TokenTree::Punct(p) if p.as_char() == ',' => match iter.next() {
            Some(tt) => unwrap_none_tt(tt),
            None => {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!("{macro_name} 密钥块逗号后缺少配置块与载荷参数，用法：{usage}"),
                ));
            }
        },
        other => unwrap_none_tt(other),
    };
    let tier_group = match second {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => g,
        other => {
            return Err(syn::Error::new_spanned(
                other,
                format!("{macro_name} 第二个参数必须是花括号配置块 `{{ key: val, ... }}`"),
            ));
        }
    };
    let tier = parse_tier_overrides(tier_group.stream(), tier_group.span())?;

    // 3. 校验第二个花括号块后的逗号并提取载荷
    let rest = split_payload_after_brace(iter, macro_name, "配置块", &usage)?;
    Ok((keys, tier, rest))
}

/// 拆分单载荷宏 `*_key_user!({ <keys> } $(,)? { <overrides> }, <payload> $(,)?)` 的顶层输入。
///
/// 返回 `(载荷 TokenStream, KeyPair, 覆盖后的 TierParams)`。
pub(crate) fn split_key_user_input(
    input: TokenStream2,
    macro_name: &str,
) -> syn::Result<(TokenStream2, KeyPair, TierParams)> {
    let (keys, tier, rest) = split_key_user_prefix(input, macro_name)?;
    Ok((strip_trailing_comma(rest), keys, tier))
}

/// 解析花括号内部的 `build_key` 与 `run_key` 键值对。
fn parse_key_pair(
    stream: TokenStream2,
    brace_span: proc_macro2::Span,
    macro_name: &str,
) -> syn::Result<KeyPair> {
    let mut build_key = None;
    let mut run_key = None;

    parse_kv_fields(
        stream,
        |key, key_str, input| {
            match key_str {
                "build_key" => {
                    let expr: syn::Expr = input.parse()?;
                    build_key = Some(parse_build_key(expr)?);
                }
                "run_key" => {
                    let expr: syn::Expr = input.parse()?;
                    run_key = Some(validate_run_key(expr)?);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        key,
                        format!("未知的密钥配置字段 `{key_str}`，可用字段：build_key, run_key"),
                    ));
                }
            }
            Ok(())
        },
        |_seen, _input| Ok(()),
    )?;

    let Some((build_key, build_key_env)) = build_key else {
        return Err(syn::Error::new(
            brace_span,
            format!("{macro_name} 缺少必填字段 `build_key`"),
        ));
    };
    let Some(run_key) = run_key else {
        return Err(syn::Error::new(
            brace_span,
            format!("{macro_name} 缺少必填字段 `run_key`"),
        ));
    };

    Ok(KeyPair {
        build_key,
        build_key_env,
        run_key,
    })
}

/// 解析 `build_key`：支持 `"..."`、`b"..."`、`[0x12, 0x34, ...]` 字节数组及 `env!("VAR")`。
fn parse_build_key(expr: syn::Expr) -> syn::Result<(Vec<u8>, Option<syn::LitStr>)> {
    let span = syn::spanned::Spanned::span(&expr);
    let (bytes, env_lit) = match unwrap_group(expr) {
        // 1. 字符串字面量 "..."
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) => (s.value().into_bytes(), None),

        // 2. 字节串字面量 b"..."
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::ByteStr(b),
            ..
        }) => (b.value(), None),

        // 3. 字节数组字面量 [0x12, 0x34, ...]
        syn::Expr::Array(arr) => (parse_byte_array_lit(&arr)?, None),

        // 4. 编译期环境变量宏 env!("VAR") 或 env!("VAR", "自定义错误")
        syn::Expr::Macro(syn::ExprMacro { mac, .. }) if is_env_macro(&mac.path) => {
            eval_env_macro(&mac)?
        }

        other => {
            return Err(syn::Error::new_spanned(
                &other,
                "`build_key` 只接受字符串字面量 `\"...\"`、字节串字面量 `b\"...\"`、字节数组 `[0x12, ...]` 或 `env!(\"...\")`",
            ));
        }
    };

    if bytes.is_empty() {
        return Err(syn::Error::new(span, "`build_key` 密钥内容不能为空"));
    }

    Ok((bytes, env_lit))
}

/// 判断宏路径是否为 `env!` / `std::env!` / `core::env!`。
fn is_env_macro(path: &syn::Path) -> bool {
    if path.is_ident("env") {
        return true;
    }
    let mut iter = path.segments.iter();
    if let (Some(first), Some(second), None) = (iter.next(), iter.next(), iter.next()) {
        return (first.ident == "std" || first.ident == "core") && second.ident == "env";
    }
    false
}

/// 在过程宏阶段求值 `env!("VAR" [, "err_msg"])`。
fn eval_env_macro(mac: &syn::Macro) -> syn::Result<(Vec<u8>, Option<syn::LitStr>)> {
    struct EnvArgs {
        var_lit: syn::LitStr,
        custom_err: Option<syn::LitStr>,
    }

    impl Parse for EnvArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let var_lit: syn::LitStr = input.parse().map_err(|_| {
                input.error("`env!` 第一个参数必须是环境变量名字符串字面量，如 `env!(\"OBF_KEY\")`")
            })?;
            let mut custom_err = None;
            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
                if !input.is_empty() {
                    custom_err = Some(input.parse::<syn::LitStr>().map_err(|_| {
                        input.error("`env!` 第二个参数（自定义错误提示）必须是字符串字面量")
                    })?);
                    if input.peek(syn::Token![,]) {
                        input.parse::<syn::Token![,]>()?;
                    }
                }
            }
            if !input.is_empty() {
                return Err(input.error("`env!` 参数过多，最多接受 2 个参数"));
            }
            Ok(EnvArgs {
                var_lit,
                custom_err,
            })
        }
    }

    let args: EnvArgs = mac.parse_body()?;
    let var_name = args.var_lit.value();

    match std::env::var(&var_name) {
        Ok(val) => Ok((val.into_bytes(), Some(args.var_lit))),
        Err(_) => {
            let msg = match args.custom_err {
                Some(custom) => custom.value(),
                None => format!(
                    "编译期未找到环境变量 `{var_name}`（由 `build_key: env!({var_name:?})` 请求）"
                ),
            };
            Err(syn::Error::new_spanned(mac, msg))
        }
    }
}

/// 校验 `run_key`：必须能绑定至 `let key: &[u8] = #run_key;`，且严禁编译期字面量写法。
fn validate_run_key(expr: syn::Expr) -> syn::Result<syn::Expr> {
    let expr = unwrap_group(expr);
    if is_compile_time_literal_key(&expr) {
        return Err(syn::Error::new_spanned(
            &expr,
            "`run_key` 不能为字面量或编译期常量表达式（否则密钥仍会硬编码进二进制，违背动态密钥威胁模型）；\
             请传入运行期变量或表达式（类型需满足 `&[u8]`，如 `&server_key` 或 `get_key()`）",
        ));
    }
    Ok(expr)
}

/// 递归检测表达式是否在编译期退化为硬编码字面量。
///
/// 穿透引用、切片、纯视图方法调用、括号、数组、代码块、分支及常量宏，
/// 防止 `b"key"`、`"key".as_bytes()`、`&[0x12, 0x34]`、`&b"key"[..]`、`env!(...)` 等硬编码形式混入。
fn is_compile_time_literal_key(expr: &syn::Expr) -> bool {
    match unwrap_group_ref(expr) {
        syn::Expr::Lit(_) => true,

        syn::Expr::Array(arr) => {
            !arr.elems.is_empty() && arr.elems.iter().all(is_compile_time_literal_key)
        }
        syn::Expr::Repeat(rep) => is_compile_time_literal_key(&rep.expr),

        syn::Expr::Reference(r) => is_compile_time_literal_key(&r.expr),
        syn::Expr::Index(idx) => is_compile_time_literal_key(&idx.expr),
        syn::Expr::Paren(p) => is_compile_time_literal_key(&p.expr),
        syn::Expr::Cast(c) => is_compile_time_literal_key(&c.expr),

        syn::Expr::MethodCall(m) => {
            let method = m.method.to_string();
            let is_view_or_conv = matches!(
                method.as_str(),
                "as_bytes"
                    | "as_slice"
                    | "as_ref"
                    | "as_mut_slice"
                    | "into_bytes"
                    | "to_vec"
                    | "to_owned"
                    | "clone"
                    | "trim"
                    | "trim_ascii"
                    | "repeat"
            );
            is_view_or_conv && is_compile_time_literal_key(&m.receiver)
        }

        syn::Expr::Block(b) => matches!(
            b.block.stmts.last(),
            Some(syn::Stmt::Expr(tail, None)) if is_compile_time_literal_key(tail)
        ),

        syn::Expr::If(i) => {
            let then_is_lit = matches!(
                i.then_branch.stmts.last(),
                Some(syn::Stmt::Expr(tail, None)) if is_compile_time_literal_key(tail)
            );
            let else_is_lit = i
                .else_branch
                .as_ref()
                .is_some_and(|(_, e)| is_compile_time_literal_key(e));
            then_is_lit || else_is_lit
        }
        syn::Expr::Match(m) => m
            .arms
            .iter()
            .any(|arm| is_compile_time_literal_key(&arm.body)),

        syn::Expr::Call(c) => match unwrap_group_ref(&c.func) {
            syn::Expr::Closure(cl) => is_compile_time_literal_key(&cl.body),
            // `(|| b"key")()`：`syn` 将带括号的闭包保留为 `Expr::Paren`
            syn::Expr::Paren(p) => matches!(
                unwrap_group_ref(&p.expr),
                syn::Expr::Closure(cl) if is_compile_time_literal_key(&cl.body)
            ),
            _ => false,
        },

        syn::Expr::Macro(m) => {
            let last_ident = m.mac.path.segments.last().map(|s| s.ident.to_string());
            matches!(
                last_ident.as_deref(),
                Some("env" | "option_env" | "include_bytes" | "include_str" | "concat")
            )
        }

        _ => false,
    }
}

macro_rules! def_combined_parsers {
    ($(($user_fn:ident, $user_mac:literal, $key_fn:ident, $key_user_fn:ident, $key_user_mac:literal, $parse_fn:ident -> $ret:ty)),* $(,)?) => {
        $(
            pub(crate) fn $user_fn(input: TokenStream2) -> syn::Result<($ret, TierParams)> {
                let (payload, tier) = split_user_input(input, $user_mac)?;
                Ok(($parse_fn(payload)?, tier))
            }

            pub(crate) fn $key_fn(input: TokenStream2, macro_name: &str) -> syn::Result<($ret, KeyPair)> {
                let (payload, keys) = split_key_input(input, macro_name)?;
                Ok(($parse_fn(payload)?, keys))
            }

            pub(crate) fn $key_user_fn(input: TokenStream2) -> syn::Result<($ret, KeyPair, TierParams)> {
                let (payload, keys, tier) = split_key_user_input(input, $key_user_mac)?;
                Ok(($parse_fn(payload)?, keys, tier))
            }
        )*
    };
}

def_combined_parsers! {
    (parse_str_user,   "s_user!",  parse_str_key,   parse_str_key_user,   "s_key_user!",  parse_str   -> String),
    (parse_bytes_user, "b_user!",  parse_bytes_key, parse_bytes_key_user, "b_key_user!",  parse_bytes -> Vec<u8>),
    (parse_file_user,  "f_user!",  parse_file_key,  parse_file_key_user,  "f_key_user!",  parse_file  -> Vec<u8>),
    (parse_int_user,   "i_user!",  parse_int_key,   parse_int_key_user,   "i_key_user!",  parse_int   -> ParsedInt),
    (parse_float_user, "fl_user!", parse_float_key, parse_float_key_user, "fl_key_user!", parse_float -> ParsedFloat),
    (parse_cstr_user,  "cs_user!", parse_cstr_key,  parse_cstr_key_user,  "cs_key_user!", parse_cstr  -> Vec<u8>),
    (parse_wstr_user,  "w_user!",  parse_wstr_key,  parse_wstr_key_user,  "w_key_user!",  parse_wstr  -> ParsedWide),
    (parse_cwstr_user, "cw_user!", parse_cwstr_key, parse_cwstr_key_user, "cw_key_user!", parse_cwstr -> ParsedCWide),
}
