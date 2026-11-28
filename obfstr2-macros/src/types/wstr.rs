// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 宽字符串档位入口：`w1` / `w2` / `w3`（低延迟 / 均衡 / 高强度）与 `w_user`（自定义档）。

use super::b_full_keyed;
use crate::core::{TIER_BALANCED, TIER_HIGH, TIER_LOW, TierParams};
use crate::parse::{KeyPair, ParsedWide};
use crate::utils::{wide_to_le, wrap_primitive};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

fn w_impl(
    parsed: ParsedWide,
    tier: TierParams,
    keys: Option<KeyPair>,
) -> Result<TokenStream2, String> {
    let ParsedWide { units } = parsed;
    let n = units.len();
    // 外层容器族跟随 b_full 的单次判定，与内层主容器族保持一致。
    let obf = b_full_keyed(wide_to_le(&units), tier, keys)?;
    let main_type_path = if obf.main_is_heap {
        quote!(::obfstr2::types::buffer::HeapU16)
    } else {
        quote!(::obfstr2::types::buffer::StackU16)
    };
    Ok(wrap_primitive(obf.tokens, n, main_type_path))
}

pub(crate) fn w1(parsed: ParsedWide) -> Result<TokenStream2, String> {
    w_impl(parsed, TIER_LOW, None)
}

pub(crate) fn w2(parsed: ParsedWide) -> Result<TokenStream2, String> {
    w_impl(parsed, TIER_BALANCED, None)
}

pub(crate) fn w3(parsed: ParsedWide) -> Result<TokenStream2, String> {
    w_impl(parsed, TIER_HIGH, None)
}

/// 自定义档：以调用方给定的 `TierParams` 混淆宽字符串载荷。
pub(crate) fn w_user(parsed: ParsedWide, tier: TierParams) -> Result<TokenStream2, String> {
    w_impl(parsed, tier, None)
}

/// 动态密钥档（均衡档基底）：编译期 `build_key` 加密，运行期 `run_key` 解密。
pub(crate) fn w_key(parsed: ParsedWide, keys: KeyPair) -> Result<TokenStream2, String> {
    w_impl(parsed, TIER_BALANCED, Some(keys))
}

/// 动态密钥 + 自定义档：以调用方给定的 `TierParams` 混淆宽字符串载荷。
pub(crate) fn w_key_user(
    parsed: ParsedWide,
    keys: KeyPair,
    tier: TierParams,
) -> Result<TokenStream2, String> {
    w_impl(parsed, tier, Some(keys))
}
