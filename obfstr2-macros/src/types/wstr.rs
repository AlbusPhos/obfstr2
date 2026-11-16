// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 宽字符串档位入口：`w1` / `w2` / `w3`（低延迟 / 均衡 / 高强度）与 `w_user`（自定义档）。

use super::b_full;
use crate::core::{TIER_BALANCED, TIER_HIGH, TIER_LOW, TierParams};
use crate::parse::ParsedWide;
use crate::utils::{wide_to_le, wrap_wide};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

fn w_impl(parsed: ParsedWide, tier: TierParams) -> Result<TokenStream2, String> {
    let ParsedWide { units } = parsed;
    let n = units.len();
    // 外层容器族跟随 b_full 的单次判定，与内层主容器族保持一致。
    let obf = b_full(wide_to_le(&units), tier)?;
    let main_type_path = if obf.main_is_heap {
        quote!(::obfstr2::types::buffer::HeapU16)
    } else {
        quote!(::obfstr2::types::buffer::StackU16)
    };
    Ok(wrap_wide(obf.tokens, n, main_type_path))
}

pub(crate) fn w1(parsed: ParsedWide) -> Result<TokenStream2, String> {
    w_impl(parsed, TIER_LOW)
}

pub(crate) fn w2(parsed: ParsedWide) -> Result<TokenStream2, String> {
    w_impl(parsed, TIER_BALANCED)
}

pub(crate) fn w3(parsed: ParsedWide) -> Result<TokenStream2, String> {
    w_impl(parsed, TIER_HIGH)
}

/// 自定义档：以调用方给定的 `TierParams` 混淆宽字符串载荷。
pub(crate) fn w_user(parsed: ParsedWide, tier: TierParams) -> Result<TokenStream2, String> {
    w_impl(parsed, tier)
}
