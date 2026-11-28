// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `crate::types::words` 单测：`u32` / `u64` 数组解析、范围校验、档位与自定义档展开。

#![allow(clippy::expect_used)]

use crate::core::{TIER_BALANCED, TierParams};
use crate::parse::{ParsedDWords, ParsedQWords, parse_dwords, parse_qwords};
use crate::types::{d_user, d1, d2, d3, q_user, q1, q2, q3};
use proc_macro2::TokenStream as TokenStream2;

fn token(s: &str) -> TokenStream2 {
    match s.parse() {
        Ok(t) => t,
        Err(e) => panic!("测试输入 {s:?} 不是合法 TokenStream: {e}"),
    }
}

fn parse_d(s: &str) -> ParsedDWords {
    match parse_dwords(token(s)) {
        Ok(p) => p,
        Err(e) => panic!("解析 {s:?} 失败: {e}"),
    }
}

fn parse_q(s: &str) -> ParsedQWords {
    match parse_qwords(token(s)) {
        Ok(p) => p,
        Err(e) => panic!("解析 {s:?} 失败: {e}"),
    }
}

#[test]
fn d_parse_values() {
    assert_eq!(
        parse_d("[0, 1u32, 0xDEAD_BEEFu32]").words,
        vec![0, 1, 0xDEAD_BEEF]
    );
    assert!(parse_d("[]").words.is_empty());
    // 允许任意整数后缀，只要值落在 u32 范围内。
    assert_eq!(parse_d("[1u8, 2i16, 3usize]").words, vec![1, 2, 3]);
}

#[test]
fn q_parse_values() {
    assert_eq!(
        parse_q("[0, 1u64, 0x0123_4567_89AB_CDEFu64]").words,
        vec![0, 1, 0x0123_4567_89AB_CDEF]
    );
    assert!(parse_q("[]").words.is_empty());
    assert_eq!(parse_q("[1u8, 2i16, 3u32]").words, vec![1, 2, 3]);
}

#[test]
fn reject_invalid_elements() {
    for input in ["[1, 2.0]", "[-1]", "[value]", "[1f32]"] {
        let e = match parse_dwords(token(input)) {
            Ok(_) => panic!("{input:?} 应当解析失败"),
            Err(e) => e.to_string(),
        };
        assert!(e.contains("dN!"), "实际: {e}");
    }

    let e = match parse_qwords(token("[18446744073709551616u128]")) {
        Ok(_) => panic!("超出 u64 范围的字面量应当解析失败"),
        Err(e) => e.to_string(),
    };
    assert!(e.contains("超出 u64 范围"), "实际: {e}");
}

#[test]
fn reject_non_array() {
    for input in ["b\"abcd\"", "42u32", "some_expr"] {
        let e = match parse_dwords(token(input)) {
            Ok(_) => panic!("{input:?} 应当解析失败"),
            Err(e) => e.to_string(),
        };
        assert!(e.contains("只接受 u32 整数字面量数组"), "实际: {e}");
    }
}

#[test]
fn polymorphic_two_expansions_differ() {
    let a = d2(parse_d("[1u32, 2, 3]")).expect("d2").to_string();
    let b = d2(parse_d("[1u32, 2, 3]")).expect("d2").to_string();
    assert_ne!(a, b, "两次展开应多态不同");

    let a = q2(parse_q("[1u64, 2, 3]")).expect("q2").to_string();
    let b = q2(parse_q("[1u64, 2, 3]")).expect("q2").to_string();
    assert_ne!(a, b, "两次展开应多态不同");
}

#[test]
fn tiers_accept_empty_and_edge_values() {
    for f in [d1, d2, d3] {
        assert!(!f(parse_d("[]")).expect("empty").is_empty());
        assert!(!f(parse_d("[0, 0xFFFF_FFFFu32]")).expect("edge").is_empty());
    }
    for f in [q1, q2, q3] {
        assert!(!f(parse_q("[]")).expect("empty").is_empty());
        assert!(
            !f(parse_q("[0, 0xFFFF_FFFF_FFFF_FFFFu64]"))
                .expect("edge")
                .is_empty()
        );
    }
}

#[test]
fn user_tier_accepts_custom_params() {
    let tier = TierParams {
        prefer_stack: true,
        ..TIER_BALANCED.clone()
    };
    assert!(
        !d_user(parse_d("[1u32, 2, 3]"), tier.clone())
            .expect("d_user")
            .is_empty()
    );
    assert!(
        !q_user(parse_q("[1u64, 2, 3]"), tier)
            .expect("q_user")
            .is_empty()
    );
}
