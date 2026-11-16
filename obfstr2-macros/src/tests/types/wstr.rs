// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `crate::types::wstr` 单测：UTF-16 编码（无结尾 NUL）、内部 NUL 保留与档位 / 自定义档展开。

#![allow(clippy::expect_used)]

use crate::core::{TIER_BALANCED, TierParams};
use crate::parse::{ParsedWide, parse_wstr};
use crate::types::{w_user, w1, w2, w3};
use crate::utils::wide_to_le;
use proc_macro2::TokenStream as TokenStream2;

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

#[test]
fn user_tier_accepts_custom_params() {
    let tier = TierParams {
        prefer_stack: true,
        ..TIER_BALANCED.clone()
    };
    assert!(
        !w_user(parse(r#""宽user""#), tier)
            .expect("w_user")
            .is_empty()
    );
    assert!(
        !w_user(parse(r#""""#), TIER_BALANCED.clone())
            .expect("w_user")
            .is_empty()
    );
}
