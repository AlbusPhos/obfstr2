// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `crate::types::cstr` 单测：三种字面量形态、NUL 校验与档位 / 自定义档展开。

#![allow(clippy::expect_used)]

use crate::core::{TIER_BALANCED, TierParams};
use crate::parse::parse_cstr;
use crate::types::{cs_user, cs1, cs2, cs3};
use proc_macro2::TokenStream as TokenStream2;

fn token(s: &str) -> TokenStream2 {
    match s.parse() {
        Ok(t) => t,
        Err(e) => panic!("测试输入 {s:?} 不是合法 TokenStream: {e}"),
    }
}

fn parse(s: &str) -> Vec<u8> {
    match parse_cstr(token(s)) {
        Ok(p) => p,
        Err(e) => panic!("解析 {s:?} 失败: {e}"),
    }
}

fn parse_err(s: &str) -> String {
    match parse_cstr(token(s)) {
        Ok(_) => panic!("{s:?} 应当解析失败"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn three_literal_forms_agree() {
    assert_eq!(parse(r#""hi""#), b"hi");
    assert_eq!(parse(r#"c"hi""#), b"hi");
    assert_eq!(parse(r#"b"hi""#), b"hi");
    assert_eq!(parse(r#""""#), b"");
    assert_eq!(parse(r#"c"""#), b"");
    assert_eq!(parse(r#"b"""#), b"");
    // Unicode 与转义由各自字面量语义展开
    assert_eq!(parse(r#""你好""#), "你好".as_bytes());
    assert_eq!(parse(r#"c"a\nb""#), b"a\nb");
}

#[test]
fn non_utf8_byte_str_accepted() {
    // CStr 不校验 UTF-8：b"..." 可表达任意非 NUL 字节
    assert_eq!(parse(r#"b"\xff\xfe""#), vec![0xFF, 0xFE]);
}

#[test]
fn interior_nul_rejected_with_position() {
    let e = parse_err(r#""a\0b""#);
    assert!(e.contains("内部 NUL") && e.contains("下标 1"), "实际: {e}");
    let e = parse_err(r#"b"hi\0""#);
    assert!(e.contains("内部 NUL"), "实际: {e}");
    let e = parse_err(r#"b"\0""#);
    assert!(e.contains("下标 0"), "实际: {e}");
}

#[test]
fn non_literal_rejected() {
    let e = parse_err("concat!(\"a\", \"b\")");
    assert!(e.contains("只接受字符串"), "实际: {e}");
    let e = parse_err("42");
    assert!(e.contains("只接受字符串"), "实际: {e}");
}

#[test]
fn polymorphic_two_expansions_differ() {
    let a = cs2(b"poly".to_vec()).expect("cs2").to_string();
    let b = cs2(b"poly".to_vec()).expect("cs2").to_string();
    assert_ne!(a, b, "两次展开应多态不同");
}

#[test]
fn tiers_accept_empty_and_single_byte() {
    for f in [cs1, cs2, cs3] {
        // 空载荷 → 仅结尾 NUL 的单字节内核（N = 1）
        assert!(!f(Vec::new()).expect("tier").is_empty());
        assert!(!f(vec![0x61]).expect("tier").is_empty());
    }
}

#[test]
fn user_tier_accepts_custom_params() {
    let tier = TierParams {
        prefer_stack: true,
        ..TIER_BALANCED.clone()
    };
    assert!(!cs_user(b"poly".to_vec(), tier).expect("cs_user").is_empty());
    assert!(
        !cs_user(Vec::new(), TIER_BALANCED.clone())
            .expect("cs_user")
            .is_empty()
    );
}
