// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 定宽数组档位入口：`d1` / `d2` / `d3`（`u32` 数组）与 `q1` / `q2` / `q3`
//! （`u64` 数组），以及对应的 `*_user` / `*_key` / `*_key_user` 变体。

use super::b_full_keyed;
use crate::core::{ObfuscatedBytes, TIER_BALANCED, TIER_HIGH, TIER_LOW, TierParams};
use crate::parse::{KeyPair, ParsedDWords, ParsedQWords};
use crate::utils::{u32_to_le, u64_to_le, wrap_primitive};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

fn wrap_words(
    obf: ObfuscatedBytes,
    n: usize,
    stack_path: TokenStream2,
    heap_path: TokenStream2,
) -> TokenStream2 {
    let main_type_path = if obf.main_is_heap {
        heap_path
    } else {
        stack_path
    };
    wrap_primitive(obf.tokens, n, main_type_path)
}

fn d_impl(
    parsed: ParsedDWords,
    tier: TierParams,
    keys: Option<KeyPair>,
) -> Result<TokenStream2, String> {
    let ParsedDWords { words } = parsed;
    let n = words.len();
    let obf = b_full_keyed(u32_to_le(&words), tier, keys)?;
    Ok(wrap_words(
        obf,
        n,
        quote!(::obfstr2::types::buffer::StackU32),
        quote!(::obfstr2::types::buffer::HeapU32),
    ))
}

fn q_impl(
    parsed: ParsedQWords,
    tier: TierParams,
    keys: Option<KeyPair>,
) -> Result<TokenStream2, String> {
    let ParsedQWords { words } = parsed;
    let n = words.len();
    let obf = b_full_keyed(u64_to_le(&words), tier, keys)?;
    Ok(wrap_words(
        obf,
        n,
        quote!(::obfstr2::types::buffer::StackU64),
        quote!(::obfstr2::types::buffer::HeapU64),
    ))
}

pub(crate) fn d1(parsed: ParsedDWords) -> Result<TokenStream2, String> {
    d_impl(parsed, TIER_LOW, None)
}

pub(crate) fn d2(parsed: ParsedDWords) -> Result<TokenStream2, String> {
    d_impl(parsed, TIER_BALANCED, None)
}

pub(crate) fn d3(parsed: ParsedDWords) -> Result<TokenStream2, String> {
    d_impl(parsed, TIER_HIGH, None)
}

/// 自定义档：以调用方给定的 `TierParams` 混淆 `u32` 数组载荷。
pub(crate) fn d_user(parsed: ParsedDWords, tier: TierParams) -> Result<TokenStream2, String> {
    d_impl(parsed, tier, None)
}

/// 动态密钥档（均衡档基底）：编译期 `build_key` 加密，运行期 `run_key` 解密。
pub(crate) fn d_key(parsed: ParsedDWords, keys: KeyPair) -> Result<TokenStream2, String> {
    d_impl(parsed, TIER_BALANCED, Some(keys))
}

/// 动态密钥 + 自定义档：以调用方给定的 `TierParams` 混淆 `u32` 数组载荷。
pub(crate) fn d_key_user(
    parsed: ParsedDWords,
    keys: KeyPair,
    tier: TierParams,
) -> Result<TokenStream2, String> {
    d_impl(parsed, tier, Some(keys))
}

pub(crate) fn q1(parsed: ParsedQWords) -> Result<TokenStream2, String> {
    q_impl(parsed, TIER_LOW, None)
}

pub(crate) fn q2(parsed: ParsedQWords) -> Result<TokenStream2, String> {
    q_impl(parsed, TIER_BALANCED, None)
}

pub(crate) fn q3(parsed: ParsedQWords) -> Result<TokenStream2, String> {
    q_impl(parsed, TIER_HIGH, None)
}

/// 自定义档：以调用方给定的 `TierParams` 混淆 `u64` 数组载荷。
pub(crate) fn q_user(parsed: ParsedQWords, tier: TierParams) -> Result<TokenStream2, String> {
    q_impl(parsed, tier, None)
}

/// 动态密钥档（均衡档基底）：编译期 `build_key` 加密，运行期 `run_key` 解密。
pub(crate) fn q_key(parsed: ParsedQWords, keys: KeyPair) -> Result<TokenStream2, String> {
    q_impl(parsed, TIER_BALANCED, Some(keys))
}

/// 动态密钥 + 自定义档：以调用方给定的 `TierParams` 混淆 `u64` 数组载荷。
pub(crate) fn q_key_user(
    parsed: ParsedQWords,
    keys: KeyPair,
    tier: TierParams,
) -> Result<TokenStream2, String> {
    q_impl(parsed, tier, Some(keys))
}
