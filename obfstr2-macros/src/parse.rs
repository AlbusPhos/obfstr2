// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 输入解析层：各宏字面量输入 → 载荷字节 / 编码结构。
//!
//! 档位适配见 `crate::types`，混淆编排见 `crate::core`。
//! 本文件只保留输入校验（字面量形态、后缀、范围、NUL、文件存在性），不做加密与发射。
//! 文末为 `*_user!` 自定义档的 `({ 配置 }, 载荷)` 统一前缀解析。

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
        syn::Expr::Array(arr) => {
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

// ── `*_user!` 自定义档：`({ key: val, ... }, 载荷)` 统一前缀解析 ────────────

use crate::core::{TIER_BALANCED, TierParams};
use proc_macro2::{Delimiter, TokenTree};
use std::collections::HashSet;
use std::ops::RangeInclusive;
use syn::parse::{Parse, ParseStream};

/// 解开 `macro_rules!` 元变量转发（`:literal` / `:expr`）产生的不可见 `None` 分组。
///
/// 与 `unwrap_group` 不同：本函数作用于裸 `TokenTree`（`split_user_prefix` 需在
/// `syn::Expr` 解析之前识别首参数的花括号分组，不能走 `syn::Expr` 入口）。
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
                format!("{macro_name} 第一个参数必须是花括号配置块 `{{ key: val, ... }}`"),
            ));
        }
    };

    let tier = parse_tier_overrides(brace_group.stream(), brace_group.span())?;

    match iter.next() {
        Some(TokenTree::Punct(p)) if p.as_char() == ',' => {}
        Some(other) => {
            return Err(syn::Error::new_spanned(
                other,
                format!("{macro_name} 配置块 `{{ ... }}` 后缺少逗号 `,`"),
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

    Ok((tier, rest))
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
    let mut tokens: Vec<TokenTree> = rest.into_iter().collect();

    if tokens.len() > 1
        && let Some(TokenTree::Punct(p)) = tokens.last()
        && p.as_char() == ','
    {
        tokens.pop();
    }

    let payload: TokenStream2 = tokens.into_iter().collect();
    Ok((payload, tier))
}

/// 解析花括号内部的 `key: value` 或 `key = value` 序列，以 `TIER_BALANCED` 为底覆盖。
fn parse_tier_overrides(
    stream: TokenStream2,
    brace_span: proc_macro2::Span,
) -> syn::Result<TierParams> {
    struct OverrideParser(TierParams);

    impl Parse for OverrideParser {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let mut tier = TIER_BALANCED;
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

                match key_str.as_str() {
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
                            &key,
                            format!(
                                "未知的 TierParams 字段 `{key_str}`，可用字段：\
                                 prefer_stack, max_latency, min_security, security_factor, \
                                 wrap_pct, junk_pct, cold_pct, pad_short, anti_disasm_pct, \
                                 anti_disasm_count_range, anti_disasm_garbage_range"
                            ),
                        ));
                    }
                }

                // 允许逗号分隔及尾随逗号
                if input.peek(syn::Token![,]) {
                    input.parse::<syn::Token![,]>()?;
                } else if !input.is_empty() {
                    return Err(input.error("配置项之间缺少逗号 `,`"));
                }
            }

            // 反反汇编注入依赖 `anti-disassembly` 特性：未启用时相关配置是静默 no-op，
            // 显式配置只会造成误解，直接拒绝。
            if !cfg!(feature = "anti-disassembly")
                && seen.iter().any(|k| k.starts_with("anti_disasm"))
            {
                return Err(input.error(
                    "当前构建未启用 anti-disassembly 特性，不能配置 anti_disasm_* 字段（请在 obfstr2 上启用默认特性后重试）",
                ));
            }

            Ok(OverrideParser(tier))
        }
    }

    let OverrideParser(tier) = syn::parse2(stream)?;
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

// ── 各类型的 `_user` 组合解析入口 ──────────────────────────────────────────

pub(crate) fn parse_str_user(input: TokenStream2) -> syn::Result<(String, TierParams)> {
    let (payload, tier) = split_user_input(input, "s_user!")?;
    Ok((parse_str(payload)?, tier))
}

pub(crate) fn parse_bytes_user(input: TokenStream2) -> syn::Result<(Vec<u8>, TierParams)> {
    let (payload, tier) = split_user_input(input, "b_user!")?;
    Ok((parse_bytes(payload)?, tier))
}

pub(crate) fn parse_file_user(input: TokenStream2) -> syn::Result<(Vec<u8>, TierParams)> {
    let (payload, tier) = split_user_input(input, "f_user!")?;
    Ok((parse_file(payload)?, tier))
}

pub(crate) fn parse_int_user(input: TokenStream2) -> syn::Result<(ParsedInt, TierParams)> {
    let (payload, tier) = split_user_input(input, "i_user!")?;
    Ok((parse_int(payload)?, tier))
}

pub(crate) fn parse_float_user(input: TokenStream2) -> syn::Result<(ParsedFloat, TierParams)> {
    let (payload, tier) = split_user_input(input, "fl_user!")?;
    Ok((parse_float(payload)?, tier))
}

pub(crate) fn parse_cstr_user(input: TokenStream2) -> syn::Result<(Vec<u8>, TierParams)> {
    let (payload, tier) = split_user_input(input, "cs_user!")?;
    Ok((parse_cstr(payload)?, tier))
}

pub(crate) fn parse_wstr_user(input: TokenStream2) -> syn::Result<(ParsedWide, TierParams)> {
    let (payload, tier) = split_user_input(input, "w_user!")?;
    Ok((parse_wstr(payload)?, tier))
}

pub(crate) fn parse_cwstr_user(input: TokenStream2) -> syn::Result<(ParsedCWide, TierParams)> {
    let (payload, tier) = split_user_input(input, "cw_user!")?;
    Ok((parse_cwstr(payload)?, tier))
}
