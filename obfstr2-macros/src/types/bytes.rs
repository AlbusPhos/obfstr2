// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::core::{ObfuscatedBytes, TIER_BALANCED, TIER_HIGH, TIER_LOW, build_obfuscated_bytes};
use proc_macro2::TokenStream as TokenStream2;

/// 供 `cs` / `s` / `w` / `cw` 外层包装复用：随 Token 流一并提供主容器族决策，
/// 外层容器族与内层严格一致（判定只在 `build_obfuscated_bytes` 内做一次）。
pub(crate) fn b1_full(input: Vec<u8>) -> Result<ObfuscatedBytes, String> {
    build_obfuscated_bytes(input, TIER_LOW)
}

pub(crate) fn b2_full(input: Vec<u8>) -> Result<ObfuscatedBytes, String> {
    build_obfuscated_bytes(input, TIER_BALANCED)
}

pub(crate) fn b3_full(input: Vec<u8>) -> Result<ObfuscatedBytes, String> {
    build_obfuscated_bytes(input, TIER_HIGH)
}

pub fn b1(input: Vec<u8>) -> Result<TokenStream2, String> {
    Ok(b1_full(input)?.tokens)
}

pub fn b2(input: Vec<u8>) -> Result<TokenStream2, String> {
    Ok(b2_full(input)?.tokens)
}

pub fn b3(input: Vec<u8>) -> Result<TokenStream2, String> {
    Ok(b3_full(input)?.tokens)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    #[test]
    fn polymorphic_two_expansions_differ() {
        // 多态核心：同一输入两次展开的 Token 流应不同（随机分块/原语/存储/标识符）。
        let a = b2(vec![42u8; 32]).expect("b2").to_string();
        let b = b2(vec![42u8; 32]).expect("b2").to_string();
        assert_ne!(a, b, "两次展开应多态不同");
    }

    #[test]
    fn tiers_accept_empty_and_single_byte() {
        for f in [b1, b2, b3] {
            assert!(!f(Vec::new()).expect("empty").is_empty());
            assert!(!f(vec![0u8]).expect("single").is_empty());
        }
    }

    #[test]
    fn stack_only_small_chunks_unaffected_by_large_total() {
        // 判定按 chunk 独立：整体 1025B 切出的小片段峰值未超限，纯栈 b1 应成功，
        // 不再被整体输入长度一票否决。
        let big = vec![0u8; 1025];
        assert!(!b1(big).expect("小片段不应受大整体影响").is_empty());
    }
}
