// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `crate::types::bytes` 单测：多态展开、档位边界、自定义档与 chunk 判定独立性。

#![allow(clippy::expect_used)]

use crate::core::{TIER_BALANCED, TierParams};
use crate::types::{b_user, b1, b2, b3};

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
fn user_tier_accepts_custom_params() {
    // 自定义档：合法自定义参数应正常展开，且保持多态性。
    let tier_a = TierParams {
        prefer_stack: true,
        ..TIER_BALANCED.clone()
    };
    let tier_b = TierParams {
        prefer_stack: false,
        max_latency: 50,
        ..TIER_BALANCED.clone()
    };
    assert!(!b_user(vec![1u8, 2, 3], tier_a).expect("b_user").is_empty());
    assert!(!b_user(vec![1u8, 2, 3], tier_b).expect("b_user").is_empty());
    let a = b_user(vec![42u8; 16], TIER_BALANCED.clone())
        .expect("b_user")
        .to_string();
    let b = b_user(vec![42u8; 16], TIER_BALANCED.clone())
        .expect("b_user")
        .to_string();
    assert_ne!(a, b, "同一自定义档两次展开应多态不同");
}

#[test]
fn stack_only_small_chunks_unaffected_by_large_total() {
    // 判定按 chunk 独立：整体 1025B 切出的小片段峰值未超限，纯栈 b1 应成功，
    // 不再被整体输入长度一票否决。
    let big = vec![0u8; 1025];
    assert!(!b1(big).expect("小片段不应受大整体影响").is_empty());
}
