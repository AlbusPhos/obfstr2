// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `crate::types::int` 单测：整数后缀推断、小端编码、范围校验与档位 / 边界值展开。

#![allow(clippy::expect_used)]

use crate::parse::{IntTy, ParsedInt, parse_int};
use crate::types::{i1, i2, i3};
use proc_macro2::TokenStream as TokenStream2;

fn token(s: &str) -> TokenStream2 {
    match s.parse() {
        Ok(t) => t,
        Err(e) => panic!("测试输入 {s:?} 不是合法 TokenStream: {e}"),
    }
}

fn parse(s: &str) -> ParsedInt {
    match parse_int(token(s)) {
        Ok(p) => p,
        Err(e) => panic!("解析 {s:?} 失败: {e}"),
    }
}

fn parse_err(s: &str) -> String {
    match parse_int(token(s)) {
        Ok(_) => panic!("{s:?} 应当解析失败"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn suffix_and_default_ty() {
    assert_eq!(parse("42u8").ty, IntTy::U8);
    assert_eq!(parse("42").ty, IntTy::I32);
    assert_eq!(parse("-1").ty, IntTy::I32);
    assert_eq!(parse("0xFFu16").ty, IntTy::U16);
    assert_eq!(parse("1_000u32").ty, IntTy::U32);
    assert_eq!(parse("1usize").ty, IntTy::Usize);
    assert_eq!(parse("-1isize").ty, IntTy::Isize);
}

#[test]
fn le_bytes_roundtrip_encoding() {
    assert_eq!(parse("0x0102u16").bytes, 0x0102u16.to_le_bytes().to_vec());
    assert_eq!(parse("-1i8").bytes, (-1i8).to_le_bytes().to_vec());
    assert_eq!(
        parse("170141183460469231731687303715884105727i128").bytes,
        i128::MAX.to_le_bytes().to_vec()
    );
}

#[test]
fn boundary_min_max_accepted() {
    parse("255u8");
    parse("127i8");
    parse("-128i8");
    parse("2147483647");
    parse("-2147483648");
    parse("340282366920938463463374607431768211455u128");
    parse("-170141183460469231731687303715884105728i128");
}

#[test]
fn out_of_range_rejected() {
    let e = parse_err("256u8");
    assert!(e.contains("超出 u8 范围"), "实际: {e}");
    let e = parse_err("-1u8");
    assert!(e.contains("负数不能用于无符号"), "实际: {e}");
    let e = parse_err("128i8");
    assert!(e.contains("超出 i8 范围"), "实际: {e}");
    let e = parse_err("-129i8");
    assert!(e.contains("超出 i8 范围"), "实际: {e}");
}

#[test]
fn non_literal_rejected() {
    let e = parse_err("u64::MAX");
    assert!(e.contains("只接受整数字面量"), "实际: {e}");
    let e = parse_err("1 + 2");
    assert!(e.contains("只接受整数字面量"), "实际: {e}");
    let e = parse_err("1f32");
    assert!(
        e.contains("只接受整数字面量") || e.contains("不支持的整数后缀"),
        "实际: {e}"
    );
}

#[test]
fn polymorphic_two_expansions_differ() {
    let p = || parse("42u32");
    // ParsedInt 不可 Clone（bytes Vec 可 clone 但 parse 每次重新来，保证输入一致）
    let a = i2(parse("42u32")).expect("i2").to_string();
    let b = i2(parse("42u32")).expect("i2").to_string();
    assert_ne!(a, b, "两次展开应多态不同");
    let _ = p;
}

#[test]
fn tiers_accept_edge_values() {
    for f in [i1, i2, i3] {
        assert!(!f(parse("0u8")).expect("tier").is_empty());
        assert!(!f(parse("-128i8")).expect("tier").is_empty());
        assert!(
            !f(parse("18446744073709551615u64"))
                .expect("tier")
                .is_empty()
        );
    }
}
