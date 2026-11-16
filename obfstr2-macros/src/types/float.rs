// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 浮点档位入口：`fl1` / `fl2` / `fl3`（低延迟 / 均衡 / 高强度）。

use crate::core::{TIER_BALANCED, TIER_HIGH, TIER_LOW, TierParams, build_obfuscated_bytes};
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

pub fn fl_impl(parsed: ParsedFloat, tier: TierParams) -> Result<TokenStream2, String> {
    let inner = build_obfuscated_bytes(parsed.bytes.clone(), tier)?.tokens;
    Ok(build_float(parsed, inner))
}

pub fn fl1(parsed: ParsedFloat) -> Result<TokenStream2, String> {
    fl_impl(parsed, TIER_LOW)
}

pub fn fl2(parsed: ParsedFloat) -> Result<TokenStream2, String> {
    fl_impl(parsed, TIER_BALANCED)
}

pub fn fl3(parsed: ParsedFloat) -> Result<TokenStream2, String> {
    fl_impl(parsed, TIER_HIGH)
}

/// 自定义档：以调用方给定的 `TierParams` 混淆浮点载荷。
pub fn fl_user(parsed: ParsedFloat, tier: TierParams) -> Result<TokenStream2, String> {
    fl_impl(parsed, tier)
}
