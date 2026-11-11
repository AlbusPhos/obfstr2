// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 宽 C 字符串档位入口：`cw1` / `cw2` / `cw3`（低延迟 / 均衡 / 高强度）。
//!
//! 注册表见 `crate::crypto` / `crate::storage`，编排见 `crate::core`，
//! 本文件只保留档位入口；UTF-16 公共还原见 `crate::utils`，解析见 `crate::parse`。
//!
//! 设计：字符串字面量经 `encode_utf16` 转为码元序列，编译期拒绝内部 NUL 后追加
//! 唯一结尾 `0u16`，再按小端展开为 `Vec<u8>`（字节数为 `2 * N`）喂入
//! `b1`/`b2`/`b3`；运行时解密为字节容器后经
//! `Buffer::extend_from_zeroable_primitive_slice_le` 还原为 `StackU16<N>` / `HeapU16<N>`，
//! 随后立即擦除并释放中转字节容器。`N` 含结尾 `0u16`（空串即 `N = 1`）；
//! 返回自有容器（`Deref<Target = [u16]>`），`as_ptr()` 可直接用于 Win32 `PCWSTR`，
//! `Drop` 自动清零。
//!
//! 编译期固定使用小端线格式、运行时用 `_le` 还原，保证 host/target 端序不同时语义不变。

use super::{b1, b2, b3, want_heap};
use crate::parse::ParsedCWide;
use crate::utils::{wide_to_le, wrap_wide};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub(crate) fn cw1(parsed: ParsedCWide) -> Result<TokenStream2, String> {
    // 纯栈、无堆（nostd 可用），与 b1/s1/cs1 同策略。
    let ParsedCWide { units } = parsed;
    let n = units.len();
    let ts = b1(wide_to_le(&units))?;
    Ok(wrap_wide(ts, n, quote!(::obfstr2::types::buffer::StackU16)))
}

pub(crate) fn cw2(parsed: ParsedCWide) -> Result<TokenStream2, String> {
    let ParsedCWide { units } = parsed;
    let n = units.len();
    let ts = b2(wide_to_le(&units))?;

    let main_type_path = if want_heap() {
        quote!(::obfstr2::types::buffer::HeapU16)
    } else {
        quote!(::obfstr2::types::buffer::StackU16)
    };

    Ok(wrap_wide(ts, n, main_type_path))
}

pub(crate) fn cw3(parsed: ParsedCWide) -> Result<TokenStream2, String> {
    let ParsedCWide { units } = parsed;
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
    use crate::parse::parse_cwstr;

    fn token(s: &str) -> TokenStream2 {
        match s.parse() {
            Ok(t) => t,
            Err(e) => panic!("测试输入 {s:?} 不是合法 TokenStream: {e}"),
        }
    }

    fn parse(s: &str) -> ParsedCWide {
        match parse_cwstr(token(s)) {
            Ok(p) => p,
            Err(e) => panic!("解析 {s:?} 失败: {e}"),
        }
    }

    fn parse_err(s: &str) -> String {
        match parse_cwstr(token(s)) {
            Ok(_) => panic!("{s:?} 应当解析失败"),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn utf16_encoding_and_terminator() {
        // ASCII: "hi" -> [h, i, 0]
        let p = parse(r#""hi""#);
        assert_eq!(p.units, vec![0x0068, 0x0069, 0x0000]);
        assert_eq!(
            wide_to_le(&p.units),
            vec![0x68, 0x00, 0x69, 0x00, 0x00, 0x00]
        );

        // BMP 中文: "你好" -> 2 个码元 + 结尾 NUL
        let p = parse(r#""你好""#);
        let mut expected: Vec<u16> = "你好".encode_utf16().collect();
        expected.push(0);
        assert_eq!(p.units, expected);

        // 非 BMP 字符（Emoji 代理对）: "🦀" -> 2 个码元 + 结尾 NUL
        let p = parse(r#""🦀""#);
        assert_eq!(p.units, vec![0xD83E, 0xDD80, 0x0000]);
    }

    #[test]
    fn empty_string_has_only_terminator() {
        let p = parse(r#""""#);
        assert_eq!(p.units, vec![0u16]);
    }

    #[test]
    fn interior_nul_rejected_with_unit_position() {
        let e = parse_err(r#""a\0b""#);
        assert!(
            e.contains("内部 NUL") && e.contains("码元下标 1"),
            "实际: {e}"
        );
        let e = parse_err(r#""\0""#);
        assert!(e.contains("码元下标 0"), "实际: {e}");
    }

    #[test]
    fn non_literal_rejected() {
        for input in ["concat!(\"a\", \"b\")", "42", r#"b"hi""#] {
            let e = parse_err(input);
            assert!(e.contains("只接受字符串字面量"), "实际: {e}");
        }
    }

    #[test]
    fn polymorphic_two_expansions_differ() {
        let a = cw2(parse(r#""polymorphic宽字符""#))
            .expect("cw2")
            .to_string();
        let b = cw2(parse(r#""polymorphic宽字符""#))
            .expect("cw2")
            .to_string();
        assert_ne!(a, b, "两次展开应多态不同");
    }

    #[test]
    fn tiers_accept_empty_and_single_unit() {
        for f in [cw1, cw2, cw3] {
            // 空载荷 -> 仅结尾 NUL 的单码元内核（N = 1）
            assert!(!f(parse(r#""""#)).expect("tier").is_empty());
            assert!(!f(parse(r#""A""#)).expect("tier").is_empty());
        }
    }
}
