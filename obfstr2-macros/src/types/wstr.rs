// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 宽字符串档位入口：`w1` / `w2` / `w3`（低延迟 / 均衡 / 高强度）。
//!
//! 与 `cw1` / `cw2` / `cw3` 的区别：
//!
//! - **不追加结尾 `0u16`**，`N` 即 UTF-16 码元数（空串 `N = 0`）；
//! - **不限制内部 NUL**，`U+0000` 作为普通码元保留（适合长度前缀 / 显式长度 API，
//!   或需要把 NUL 作为数据的一部分的场景）；
//! - 因此返回值不保证以 NUL 结尾，**不要**直接传给需要 `PCWSTR` 的 Win32 API；
//!   NUL 结尾需求请使用 `cwN!`。
//!
//! 其余链路与宽 C 字符串完全一致：`encode_utf16` → 小端字节流 → `bN!` 混淆 →
//! 运行时 `extend_from_zeroable_primitive_slice_le` 还原为 `StackU16<N>` /
//! `HeapU16<N>`，中转字节容器立即擦除，外层容器 `Drop` 自动清零。

use super::{b1, b2, b3, want_heap};
use crate::parse::ParsedWide;
use crate::utils::{wide_to_le, wrap_wide};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub(crate) fn w1(parsed: ParsedWide) -> Result<TokenStream2, String> {
    // 纯栈、无堆（nostd 可用），与 b1/s1/cs1/cw1 同策略。
    let ParsedWide { units } = parsed;
    let n = units.len();
    let ts = b1(wide_to_le(&units))?;
    Ok(wrap_wide(ts, n, quote!(::obfstr2::types::buffer::StackU16)))
}

pub(crate) fn w2(parsed: ParsedWide) -> Result<TokenStream2, String> {
    let ParsedWide { units } = parsed;
    let n = units.len();
    let ts = b2(wide_to_le(&units))?;

    let main_type_path = if want_heap() {
        quote!(::obfstr2::types::buffer::HeapU16)
    } else {
        quote!(::obfstr2::types::buffer::StackU16)
    };

    Ok(wrap_wide(ts, n, main_type_path))
}

pub(crate) fn w3(parsed: ParsedWide) -> Result<TokenStream2, String> {
    let ParsedWide { units } = parsed;
    let n = units.len();
    let ts = b3(wide_to_le(&units))?;

    let main_type_path = if cfg!(feature = "alloc") {
        quote!(::obfstr2::types::buffer::HeapU16)
    } else {
        quote!(::obfstr2::types::buffer::StackU16)
    };

    Ok(wrap_wide(ts, n, main_type_path))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::parse::parse_wstr;

    fn token(s: &str) -> TokenStream2 {
        match s.parse() {
            Ok(t) => t,
            Err(e) => panic!("测试输入 {s:?} 不是合法 TokenStream: {e}"),
        }
    }

    fn parse(s: &str) -> ParsedWide {
        match parse_wstr(token(s)) {
            Ok(p) => p,
            Err(e) => panic!("解析 {s:?} 失败: {e}"),
        }
    }

    fn parse_err(s: &str) -> String {
        match parse_wstr(token(s)) {
            Ok(_) => panic!("{s:?} 应当解析失败"),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn utf16_encoding_has_no_terminator() {
        // ASCII: "hi" -> [h, i]（无结尾 0）
        let p = parse(r#""hi""#);
        assert_eq!(p.units, vec![0x0068, 0x0069]);
        assert_eq!(wide_to_le(&p.units), vec![0x68, 0x00, 0x69, 0x00]);

        // BMP 中文: "你好" -> 2 个码元
        let p = parse(r#""你好""#);
        let expected: Vec<u16> = "你好".encode_utf16().collect();
        assert_eq!(p.units, expected);

        // 非 BMP 字符（Emoji 代理对）: "🦀" -> 2 个码元
        let p = parse(r#""🦀""#);
        assert_eq!(p.units, vec![0xD83E, 0xDD80]);
    }

    #[test]
    fn nul_units_are_preserved() {
        // 内部 / 首尾 NUL 都是普通码元，不拒绝、不追加、不截断
        assert_eq!(parse(r#""a\0b""#).units, vec![0x0061, 0x0000, 0x0062]);
        assert_eq!(parse(r#""\0""#).units, vec![0x0000]);
        assert_eq!(parse(r#""\0a\0""#).units, vec![0x0000, 0x0061, 0x0000]);
    }

    #[test]
    fn empty_string_has_zero_units() {
        let p = parse(r#""""#);
        assert!(p.units.is_empty());
    }

    #[test]
    fn non_literal_rejected() {
        for input in ["concat!(\"a\", \"b\")", "42", r#"b"hi""#] {
            let e = parse_err(input);
            assert!(
                e.contains("wN!") && e.contains("只接受字符串字面量"),
                "实际: {e}"
            );
        }
    }

    #[test]
    fn polymorphic_two_expansions_differ() {
        let a = w2(parse(r#""polymorphic宽字符""#)).expect("w2").to_string();
        let b = w2(parse(r#""polymorphic宽字符""#)).expect("w2").to_string();
        assert_ne!(a, b, "两次展开应多态不同");
    }

    #[test]
    fn tiers_accept_empty_and_single_unit() {
        for f in [w1, w2, w3] {
            // 空载荷 -> N = 0；单码元 -> N = 1，均不应在展开期报错
            assert!(!f(parse(r#""""#)).expect("empty").is_empty());
            assert!(!f(parse(r#""A""#)).expect("single").is_empty());
        }
    }
}
