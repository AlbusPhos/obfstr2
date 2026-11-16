// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `crate::types::cwstr` 单测：UTF-16 编码与结尾 NUL、内部 NUL 拒绝与档位 / 自定义档展开。

#![allow(clippy::expect_used)]

use crate::core::{TIER_BALANCED, TierParams};
use crate::parse::{ParsedCWide, parse_cwstr};
use crate::types::{cw_user, cw1, cw2, cw3};
use crate::utils::wide_to_le;
use proc_macro2::TokenStream as TokenStream2;

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

#[test]
fn user_tier_accepts_custom_params() {
    let tier = TierParams {
        prefer_stack: true,
        ..TIER_BALANCED.clone()
    };
    assert!(
        !cw_user(parse(r#""宽Cuser""#), tier)
            .expect("cw_user")
            .is_empty()
    );
    assert!(
        !cw_user(parse(r#""""#), TIER_BALANCED.clone())
            .expect("cw_user")
            .is_empty()
    );
}
