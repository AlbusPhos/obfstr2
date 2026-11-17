// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 浮点档位入口：`fl1` / `fl2` / `fl3`（低延迟 / 均衡 / 高强度）。

use crate::core::{TIER_BALANCED, TIER_HIGH, TIER_LOW, TierParams, build_obfuscated_bytes};
use crate::parse::{FloatTy, KeyPair, ParsedFloat};
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

pub fn fl_impl(
    parsed: ParsedFloat,
    tier: TierParams,
    keys: Option<KeyPair>,
) -> Result<TokenStream2, String> {
    let inner = build_obfuscated_bytes(parsed.bytes.clone(), tier, keys)?.tokens;
    Ok(build_float(parsed, inner))
}

pub fn fl1(parsed: ParsedFloat) -> Result<TokenStream2, String> {
    fl_impl(parsed, TIER_LOW, None)
}

pub fn fl2(parsed: ParsedFloat) -> Result<TokenStream2, String> {
    fl_impl(parsed, TIER_BALANCED, None)
}

pub fn fl3(parsed: ParsedFloat) -> Result<TokenStream2, String> {
    fl_impl(parsed, TIER_HIGH, None)
}

/// 自定义档：以调用方给定的 `TierParams` 混淆浮点载荷。
pub fn fl_user(parsed: ParsedFloat, tier: TierParams) -> Result<TokenStream2, String> {
    fl_impl(parsed, tier, None)
}

/// 动态密钥档（均衡档基底）：编译期 `build_key` 加密，运行期 `run_key` 解密。
pub fn fl_key(parsed: ParsedFloat, keys: KeyPair) -> Result<TokenStream2, String> {
    fl_impl(parsed, TIER_BALANCED, Some(keys))
}

/// 动态密钥 + 自定义档：以调用方给定的 `TierParams` 混淆浮点载荷。
pub fn fl_key_user(
    parsed: ParsedFloat,
    keys: KeyPair,
    tier: TierParams,
) -> Result<TokenStream2, String> {
    fl_impl(parsed, tier, Some(keys))
}
