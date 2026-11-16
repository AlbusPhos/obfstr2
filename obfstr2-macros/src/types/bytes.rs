// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::core::{
    ObfuscatedBytes, TIER_BALANCED, TIER_HIGH, TIER_LOW, TierParams, build_obfuscated_bytes,
};
use proc_macro2::TokenStream as TokenStream2;

/// 任意档位的混淆内核：供档位变体（`b1` ~ `b3`）与自定义档位（`b_user`）复用。
///
/// 随 Token 流一并提供主容器族决策，`cs` / `s` / `w` / `cw` 外层包装与内层严格一致
pub(crate) fn b_full(input: Vec<u8>, tier: TierParams) -> Result<ObfuscatedBytes, String> {
    build_obfuscated_bytes(input, tier)
}

pub fn b1(input: Vec<u8>) -> Result<TokenStream2, String> {
    Ok(b_full(input, TIER_LOW)?.tokens)
}

pub fn b2(input: Vec<u8>) -> Result<TokenStream2, String> {
    Ok(b_full(input, TIER_BALANCED)?.tokens)
}

pub fn b3(input: Vec<u8>) -> Result<TokenStream2, String> {
    Ok(b_full(input, TIER_HIGH)?.tokens)
}

/// 自定义档：以调用方给定的 `TierParams` 混淆字节载荷。
pub fn b_user(input: Vec<u8>, tier: TierParams) -> Result<TokenStream2, String> {
    Ok(b_full(input, tier)?.tokens)
}
