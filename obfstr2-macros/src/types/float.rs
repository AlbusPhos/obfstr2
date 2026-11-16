// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 浮点档位入口：`fl1` / `fl2` / `fl3`（低延迟 / 均衡 / 高强度）。

use crate::core::{TIER_BALANCED, TIER_HIGH, TIER_LOW, build_obfuscated_bytes};
use crate::parse::{FloatTy, ParsedFloat};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

fn decode_expr(ty: FloatTy) -> TokenStream2 {
    match ty {
        FloatTy::F32 => quote!(::core::primitive::f32::from_bits(
            ::core::primitive::u32::from_le_bytes(
                ::core::convert::TryInto::try_into(__float_bytes.as_slice()).unwrap_unchecked()
            )
        )),
        FloatTy::F64 => quote!(::core::primitive::f64::from_bits(
            ::core::primitive::u64::from_le_bytes(
                ::core::convert::TryInto::try_into(__float_bytes.as_slice()).unwrap_unchecked()
            )
        )),
    }
}

fn build_float(parsed: ParsedFloat, inner: TokenStream2) -> TokenStream2 {
    let decode = decode_expr(parsed.ty);
    quote! {
        {
            let mut __float_bytes = #inner;
            unsafe { #decode }
        }
    }
}

pub fn fl1(parsed: ParsedFloat) -> Result<TokenStream2, String> {
    let inner = build_obfuscated_bytes(parsed.bytes.clone(), TIER_LOW)?.tokens;
    Ok(build_float(parsed, inner))
}

pub fn fl2(parsed: ParsedFloat) -> Result<TokenStream2, String> {
    let inner = build_obfuscated_bytes(parsed.bytes.clone(), TIER_BALANCED)?.tokens;
    Ok(build_float(parsed, inner))
}

pub fn fl3(parsed: ParsedFloat) -> Result<TokenStream2, String> {
    let inner = build_obfuscated_bytes(parsed.bytes.clone(), TIER_HIGH)?.tokens;
    Ok(build_float(parsed, inner))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::parse::parse_float;

    fn token(s: &str) -> TokenStream2 {
        match s.parse() {
            Ok(t) => t,
            Err(e) => panic!("测试输入 {s:?} 不是合法 TokenStream: {e}"),
        }
    }

    fn parse(s: &str) -> ParsedFloat {
        match parse_float(token(s)) {
            Ok(p) => p,
            Err(e) => panic!("解析 {s:?} 失败: {e}"),
        }
    }

    fn parse_err(s: &str) -> String {
        match parse_float(token(s)) {
            Ok(_) => panic!("{s:?} 应当解析失败"),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn suffix_and_default_ty() {
        assert_eq!(parse("1.0f32").ty, FloatTy::F32);
        assert_eq!(parse("1.0f64").ty, FloatTy::F64);
        assert_eq!(parse("1.0").ty, FloatTy::F64);
        assert_eq!(parse("-1.0").ty, FloatTy::F64);
        assert_eq!(parse("5f32").ty, FloatTy::F32);
        assert_eq!(parse("1e10").ty, FloatTy::F64);
        assert_eq!(parse("1_000.5f32").ty, FloatTy::F32);
    }

    #[test]
    fn bits_roundtrip_encoding() {
        assert_eq!(
            parse("1.5f32").bytes,
            1.5f32.to_bits().to_le_bytes().to_vec()
        );
        assert_eq!(
            parse("-0.0").bytes,
            (-0.0f64).to_bits().to_le_bytes().to_vec()
        );
        assert_eq!(
            parse("-0.0f32").bytes,
            (-0.0f32).to_bits().to_le_bytes().to_vec()
        );
        // -0.0 与 0.0 位必须不同（符号位保留）
        assert_ne!(
            parse("-0.0f32").bytes,
            parse("0.0f32").bytes,
            "-0.0 符号位必须保留"
        );
    }

    #[test]
    fn boundary_values_accepted() {
        parse("340282346638528859811704183484516925440.0f32"); // f32::MAX
        parse("-340282346638528859811704183484516925440.0f32");
        parse("1.7976931348623157e308"); // f64::MAX
        parse("5e-324"); // 最小 f64 次正规数
        parse("1e-45f32"); // 次正规区
        parse("1.17549435e-38f32"); // f32::MIN_POSITIVE（最小正规格数）
    }

    #[test]
    fn overflow_to_inf_rejected() {
        let e = parse_err("1e999f32");
        assert!(e.contains("无穷"), "实际: {e}");
        let e = parse_err("1e999");
        assert!(e.contains("无穷"), "实际: {e}");
    }

    #[test]
    fn non_literal_rejected() {
        let e = parse_err("f64::MAX");
        assert!(e.contains("只接受浮点字面量"), "实际: {e}");
        let e = parse_err("1.0 + 2.0");
        assert!(e.contains("只接受浮点字面量"), "实际: {e}");
        let e = parse_err("f64::INFINITY");
        assert!(e.contains("只接受浮点字面量"), "实际: {e}");
        let e = parse_err("1.0f16");
        assert!(e.contains("后缀"), "实际: {e}");
    }

    #[test]
    fn polymorphic_two_expansions_differ() {
        let a = fl2(parse("3.15")).expect("fl2").to_string();
        let b = fl2(parse("3.15")).expect("fl2").to_string();
        assert_ne!(a, b, "两次展开应多态不同");
    }

    #[test]
    fn tiers_accept_edge_values() {
        for f in [fl1, fl2, fl3] {
            assert!(!f(parse("0.0f32")).expect("tier").is_empty());
            assert!(!f(parse("-0.0")).expect("tier").is_empty());
            assert!(!f(parse("1.7976931348623157e308")).expect("tier").is_empty());
        }
    }
}
