// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! C 字符串档位入口：`cs1` / `cs2` / `cs3`（低延迟 / 均衡 / 高强度）与 `cs_user`（自定义档）。

use super::b_full_keyed;
use crate::core::{TIER_BALANCED, TIER_HIGH, TIER_LOW, TierParams};
use crate::parse::KeyPair;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

/// 载荷追加结尾 `\0`（调用方已保证载荷无内部 NUL，追加后恰为合法 C 串字节）。
fn with_nul(mut payload: Vec<u8>) -> Vec<u8> {
    payload.push(0);
    payload
}

fn cs_impl(
    payload: Vec<u8>,
    tier: TierParams,
    keys: Option<KeyPair>,
) -> Result<TokenStream2, String> {
    // 容量 N 含结尾 `\0`；主容器族跟随 b_full 的单次判定（alloc 下按档位 prefer_stack 掷币）。
    let size = payload.len() + 1;
    // 主容器族必须与 b_full 内部选择一致，才能用 from_inner_with_nul 零拷贝消费：
    let obf = b_full_keyed(with_nul(payload), tier, keys)?;
    let ts = obf.tokens;
    let main_type_path = if obf.main_is_heap {
        quote!(::obfstr2::types::cstr::HeapCStr)
    } else {
        quote!(::obfstr2::types::cstr::StackCStr)
    };

    Ok(quote! {
        {
            let bytes = #ts;
            unsafe { #main_type_path::<#size>::from_inner_with_nul(bytes).unwrap_unchecked() }
        }
    })
}

pub fn cs1(payload: Vec<u8>) -> Result<TokenStream2, String> {
    cs_impl(payload, TIER_LOW, None)
}

pub fn cs2(payload: Vec<u8>) -> Result<TokenStream2, String> {
    cs_impl(payload, TIER_BALANCED, None)
}

pub fn cs3(payload: Vec<u8>) -> Result<TokenStream2, String> {
    cs_impl(payload, TIER_HIGH, None)
}

/// 自定义档：以调用方给定的 `TierParams` 混淆 C 字符串载荷。
pub fn cs_user(payload: Vec<u8>, tier: TierParams) -> Result<TokenStream2, String> {
    cs_impl(payload, tier, None)
}

/// 动态密钥档（均衡档基底）：编译期 `build_key` 加密，运行期 `run_key` 解密。
pub fn cs_key(payload: Vec<u8>, keys: KeyPair) -> Result<TokenStream2, String> {
    cs_impl(payload, TIER_BALANCED, Some(keys))
}

/// 动态密钥 + 自定义档：以调用方给定的 `TierParams` 混淆 C 字符串载荷。
pub fn cs_key_user(
    payload: Vec<u8>,
    keys: KeyPair,
    tier: TierParams,
) -> Result<TokenStream2, String> {
    cs_impl(payload, tier, Some(keys))
}
