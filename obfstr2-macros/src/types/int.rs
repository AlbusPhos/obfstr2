// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 整数档位入口：`i1` / `i2` / `i3`（低延迟 / 均衡 / 高强度）。

use crate::core::{TIER_BALANCED, TIER_HIGH, TIER_LOW, TierParams, build_obfuscated_bytes};
use crate::parse::{IntTy, KeyPair, ParsedInt};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

/// 解码表达式：字节容器 `->` 裸整数。`usize/isize` 经 `u64/i64` 中转（64 位语义）。
macro_rules! decode_le {
    ($ty:ty) => {
        quote!(::core::primitive::$ty::from_le_bytes(
            ::core::convert::TryInto::try_into(__int_bytes.as_slice()).unwrap_unchecked()
        ))
    };
    ($ty:ty as $cast:ty) => {
        quote!(
            (::core::primitive::$ty::from_le_bytes(
                ::core::convert::TryInto::try_into(__int_bytes.as_slice()).unwrap_unchecked()
            ) as $cast)
        )
    };
}

fn decode_expr(ty: IntTy) -> TokenStream2 {
    match ty {
        IntTy::U8 => decode_le!(u8),
        IntTy::I8 => decode_le!(i8),
        IntTy::U16 => decode_le!(u16),
        IntTy::I16 => decode_le!(i16),
        IntTy::U32 => decode_le!(u32),
        IntTy::I32 => decode_le!(i32),
        IntTy::U64 => decode_le!(u64),
        IntTy::I64 => decode_le!(i64),
        IntTy::U128 => decode_le!(u128),
        IntTy::I128 => decode_le!(i128),
        IntTy::Usize => decode_le!(u64 as usize),
        IntTy::Isize => decode_le!(i64 as isize),
    }
}

fn build_int(parsed: ParsedInt, inner: TokenStream2) -> TokenStream2 {
    let decode = decode_expr(parsed.ty);
    quote! {
        {
            let mut __int_bytes = #inner;
            unsafe { #decode }
        }
    }
}

pub fn i_impl(
    parsed: ParsedInt,
    tier: TierParams,
    keys: Option<KeyPair>,
) -> Result<TokenStream2, String> {
    let inner = build_obfuscated_bytes(parsed.bytes.clone(), tier, keys)?.tokens;
    Ok(build_int(parsed, inner))
}

pub fn i1(parsed: ParsedInt) -> Result<TokenStream2, String> {
    i_impl(parsed, TIER_LOW, None)
}

pub fn i2(parsed: ParsedInt) -> Result<TokenStream2, String> {
    i_impl(parsed, TIER_BALANCED, None)
}

pub fn i3(parsed: ParsedInt) -> Result<TokenStream2, String> {
    i_impl(parsed, TIER_HIGH, None)
}

/// 自定义档：以调用方给定的 `TierParams` 混淆整数载荷。
pub fn i_user(parsed: ParsedInt, tier: TierParams) -> Result<TokenStream2, String> {
    i_impl(parsed, tier, None)
}

/// 动态密钥档（均衡档基底）：编译期 `build_key` 加密，运行期 `run_key` 解密。
pub fn i_key(parsed: ParsedInt, keys: KeyPair) -> Result<TokenStream2, String> {
    i_impl(parsed, TIER_BALANCED, Some(keys))
}

/// 动态密钥 + 自定义档：以调用方给定的 `TierParams` 混淆整数载荷。
pub fn i_key_user(
    parsed: ParsedInt,
    keys: KeyPair,
    tier: TierParams,
) -> Result<TokenStream2, String> {
    i_impl(parsed, tier, Some(keys))
}
