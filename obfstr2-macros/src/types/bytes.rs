// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::core::{
    ObfuscatedBytes, TIER_BALANCED, TIER_HIGH, TIER_LOW, TierParams, build_obfuscated_bytes,
};
use crate::parse::KeyPair;
use proc_macro2::TokenStream as TokenStream2;

/// 任意档位的混淆内核：供档位变体（`b1` ~ `b3`）与自定义档位（`b_user`）复用。
///
/// 随 Token 流一并提供主容器族决策，`cs` / `s` / `w` / `cw` 外层包装与内层严格一致
pub(crate) fn b_full(input: Vec<u8>, tier: TierParams) -> Result<ObfuscatedBytes, String> {
    b_full_keyed(input, tier, None)
}

/// 带可选动态密钥的混淆内核：`*_key!` / `*_key_user!` 与常规档位共用。
pub(crate) fn b_full_keyed(
    input: Vec<u8>,
    tier: TierParams,
    key_pair: Option<KeyPair>,
) -> Result<ObfuscatedBytes, String> {
    build_obfuscated_bytes(input, tier, key_pair)
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

/// 动态密钥档（均衡档基底）：编译期 `build_key` 加密，运行期 `run_key` 解密。
pub fn b_key(input: Vec<u8>, keys: KeyPair) -> Result<TokenStream2, String> {
    Ok(b_full_keyed(input, TIER_BALANCED, Some(keys))?.tokens)
}

/// 动态密钥 + 自定义档：以调用方给定的 `TierParams` 混淆字节载荷。
pub fn b_key_user(input: Vec<u8>, keys: KeyPair, tier: TierParams) -> Result<TokenStream2, String> {
    Ok(b_full_keyed(input, tier, Some(keys))?.tokens)
}
