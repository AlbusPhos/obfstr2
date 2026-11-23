// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `*_user!` 自定义档解析层单测：`({ 配置 }, 载荷)` 前缀切分、档位覆盖与校验。
//!
//! 覆盖见 `crate::parse`（`split_user_prefix` / `split_user_input` /
//! `parse_tier_overrides`）与 `crate::types`（`*_user` builder）。

#![allow(clippy::expect_used)]

use crate::core::TIER_BALANCED;
use crate::parse::*;
use crate::types::b_user;
use quote::quote;

/// 提取解析错误信息（`TierParams` / `ParsedInt` 等载荷类型无 `Debug`，
/// 不能对整个 `Result` 调 `unwrap_err`）。
fn err_of<T>(r: syn::Result<T>) -> String {
    match r {
        Ok(_) => panic!("应当解析失败"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn empty_braces_equal_tier_balanced() {
    let (s, tier) = parse_str_user(quote!({}, "hello")).expect("empty override");
    assert_eq!(s, "hello");
    assert_eq!(tier.max_latency, TIER_BALANCED.max_latency);
    assert_eq!(tier.min_security, TIER_BALANCED.min_security);
    assert_eq!(tier.prefer_stack, TIER_BALANCED.prefer_stack);
    assert_eq!(tier.force_heap, TIER_BALANCED.force_heap);
    assert_eq!(tier.junk_pct, TIER_BALANCED.junk_pct);
    assert_eq!(tier.cold_pct, TIER_BALANCED.cold_pct);
    assert_eq!(tier.pad_short, TIER_BALANCED.pad_short);
    assert_eq!(tier.anti_disasm_pct, TIER_BALANCED.anti_disasm_pct);
    assert_eq!(tier.security_factor, TIER_BALANCED.security_factor);
}

#[test]
fn override_all_field_types_and_trailing_comma() {
    // `:` 与 `=` 混用、配置块与载荷尾随逗号均允许。
    // `anti_disasm_*` 字段依赖 `anti-disassembly` 特性（未启用时编译期拒绝），
    // 单独拆分至 `override_anti_disasm_fields_and_trailing_comma`。
    let (bytes, tier) = parse_bytes_user(quote!(
        {
            prefer_stack: true,
            force_heap: false,
            max_latency = 50,
            min_security: 80,
            wrap_pct: 25,
            junk_pct: 10,
            cold_pct: 5,
            pad_short: false,
        },
        [0x41, 0x42, 0x43],
    ))
    .expect("full override with trailing commas");

    assert_eq!(bytes, vec![0x41, 0x42, 0x43]);
    assert!(tier.prefer_stack);
    assert_eq!(tier.force_heap, Some(false));
    assert_eq!(tier.max_latency, 50);
    assert_eq!(tier.min_security, 80);
    assert!(!tier.pad_short);
    assert_eq!(tier.wrap_pct, 25);
    assert_eq!(tier.junk_pct, 10);
    assert_eq!(tier.cold_pct, 5);
}

/// 反反汇编字段覆盖：`parse_tier_overrides` 在未启用 `anti-disassembly` 特性时
/// 会拒绝显式配置（防静默 no-op 误解），故本测试仅在启用该特性时编译参与。
#[cfg(feature = "anti-disassembly")]
#[test]
fn override_anti_disasm_fields_and_trailing_comma() {
    let (_, tier) = parse_bytes_user(quote!(
        {
            anti_disasm_pct: 60,
            anti_disasm_count_range: 2..=4,
            anti_disasm_garbage_range = 6..=12,
        },
        [0x41, 0x42, 0x43],
    ))
    .expect("anti-disasm override with trailing commas");

    assert_eq!(tier.anti_disasm_pct, 60);
    assert_eq!(tier.anti_disasm_count_range, 2..=4);
    assert_eq!(tier.anti_disasm_garbage_range, 6..=12);
}

#[test]
fn force_heap_override_false_and_none() {
    // `false` 表示强制走栈，而非“关闭强制堆”
    let (_, tier) = parse_bytes_user(quote!({ force_heap: false }, b"x")).expect("bool false");
    assert_eq!(tier.force_heap, Some(false));

    let (_, tier) =
        parse_bytes_user(quote!({ force_heap = Some(false) }, b"x")).expect("Some(false)");
    assert_eq!(tier.force_heap, Some(false));

    // `None` 表示继承 Cargo 特性 / 默认启发式
    let (_, tier) = parse_bytes_user(quote!({ force_heap: None }, b"x")).expect("None");
    assert_eq!(tier.force_heap, None);
}

#[cfg(feature = "alloc")]
#[test]
fn force_heap_override_true_with_alloc() {
    let (_, tier) = parse_bytes_user(quote!({ force_heap: true }, b"x")).expect("bool true");
    assert_eq!(tier.force_heap, Some(true));

    let (_, tier) =
        parse_bytes_user(quote!({ force_heap = Some(true) }, b"x")).expect("Some(true)");
    assert_eq!(tier.force_heap, Some(true));
}

#[cfg(not(feature = "alloc"))]
#[test]
fn force_heap_true_rejected_without_alloc() {
    for input in [
        quote!({ force_heap: true }, b"x"),
        quote!({ force_heap: Some(true) }, b"x"),
    ] {
        let err = err_of(parse_bytes_user(input));
        assert!(err.contains("未启用 alloc"), "实际: {err}");
    }
}

#[test]
fn reject_invalid_force_heap_values() {
    let err = err_of(parse_bytes_user(quote!({ force_heap: 1 }, b"x")));
    assert!(err.contains("需要布尔字面量"), "实际: {err}");

    let err = err_of(parse_bytes_user(quote!({ force_heap: Some(1) }, b"x")));
    assert!(err.contains("内需要布尔字面量"), "实际: {err}");

    let err = err_of(parse_bytes_user(
        quote!({ force_heap: Some(true, false) }, b"x"),
    ));
    assert!(err.contains("多余 token"), "实际: {err}");
}

#[test]
fn sfmt_prefix_split_preserves_variadic_args() {
    // s_fmt_user! 的 rest 应原样保留格式串与变长参数（含嵌套花括号参数）
    let (tier, rest) = split_user_prefix(
        quote!({ min_security: 75 }, "hello {}, {}", name, { 1 + 2 }),
        "s_fmt_user!",
    )
    .expect("sfmt split");
    assert_eq!(tier.min_security, 75);
    // 两路均为 `quote!` 归一化输出，`to_string` 可稳定比较
    assert_eq!(
        rest.to_string(),
        quote!("hello {}, {}", name, { 1 + 2 }).to_string(),
        "变长参数流应原样保留"
    );
}

#[test]
fn reject_invalid_overrides() {
    // 1. 未知字段
    let err = err_of(parse_str_user(quote!({ unknown_field: 1 }, "hi")));
    assert!(err.contains("未知的 TierParams 字段"), "实际: {err}");

    // 2. 重复字段
    let err = err_of(parse_str_user(quote!({ wrap_pct: 10, wrap_pct: 20 }, "hi")));
    assert!(err.contains("重复配置字段"), "实际: {err}");

    // 3. min_security >= 100 死循环拦截
    let err = err_of(parse_int_user(quote!({ min_security: 100 }, -42i32)));
    assert!(err.contains("min_security 不能 >= 100"), "实际: {err}");

    // 4. 倒置的 RangeInclusive 拦截
    let err = err_of(parse_float_user(
        quote!({ anti_disasm_count_range: 5..=1 }, 3.14),
    ));
    assert!(err.contains("下界 (5) 不能大于上界 (1)"), "实际: {err}");

    // 5. max_latency 低于原语注册表最低单层延迟（2）拦截
    let err = err_of(parse_str_user(quote!({ max_latency: 1 }, "hi")));
    assert!(err.contains("max_latency 不能 < 2"), "实际: {err}");

    // 6. 百分比字段超界拦截
    let err = err_of(parse_str_user(quote!({ wrap_pct: 101 }, "hi")));
    assert!(err.contains("wrap_pct 必须在 0..=100"), "实际: {err}");
}

#[test]
fn reject_malformed_calls() {
    // 1. 完全无参数
    let err = err_of(parse_str_user(quote!()));
    assert!(err.contains("缺少参数"), "实际: {err}");

    // 2. 首参数不是花括号配置块
    let err = err_of(parse_str_user(quote!("hi")));
    assert!(err.contains("第一个参数必须是花括号配置块"), "实际: {err}");

    // 3. 配置块后缺少逗号
    let err = err_of(parse_str_user(quote!({} "hi")));
    assert!(err.contains("后缺少逗号"), "实际: {err}");

    // 4. 逗号后缺少载荷
    let err = err_of(parse_str_user(quote!({},)));
    assert!(err.contains("缺少字面量载荷参数"), "实际: {err}");

    // 5. 配置块内缺少逗号分隔
    let err = err_of(parse_str_user(
        quote!({ max_latency: 40 min_security: 60 }, "hi"),
    ));
    assert!(err.contains("缺少逗号"), "实际: {err}");
}

#[test]
fn reject_bad_value_literals() {
    // 非整数字面量
    let err = err_of(parse_str_user(quote!({ max_latency: "high" }, "hi")));
    assert!(
        err.contains("需要 0..=255 的无符号整数字面量"),
        "实际: {err}"
    );

    // 半开区间
    let err = err_of(parse_str_user(
        quote!({ anti_disasm_count_range: 1..3 }, "hi"),
    ));
    assert!(err.contains("闭区间运算符 `..=`"), "实际: {err}");

    // 超出 u8
    let err = err_of(parse_str_user(quote!({ min_security: 256 }, "hi")));
    assert!(err.contains("超出 u8"), "实际: {err}");
}

#[test]
fn security_factor_override_and_validation() {
    // 浮点与整数字面量两种写法
    let (_, tier) = parse_bytes_user(quote!({ security_factor: 0.5 }, b"x")).expect("float form");
    assert_eq!(tier.security_factor, 0.5);
    let (_, tier) = parse_bytes_user(quote!({ security_factor = 2 }, b"x")).expect("int form");
    assert_eq!(tier.security_factor, 2.0);

    // 0.0 / 负数 / 非有限值 → 编译期拒绝（死循环防护）
    for bad in [
        quote!({ security_factor: 0.0 }, b"x"),
        quote!({ security_factor: -1.0 }, b"x"),
    ] {
        let err = err_of(parse_bytes_user(bad));
        assert!(err.contains("必须是 > 0.0 的有限浮点数"), "实际: {err}");
    }

    // 非数字字面量 → 解析期拒绝
    let err = err_of(parse_bytes_user(quote!({ security_factor: "fast" }, b"x")));
    assert!(err.contains("需要正的有限浮点数字面量"), "实际: {err}");
}

#[test]
fn user_tier_builder_smoke() {
    // 解析 + 构建端到端：合法自定义档展开非空且保持多态性
    let (_, tier_a) = parse_bytes_user(quote!({ prefer_stack: true }, b"poly")).expect("parse");
    let (_, tier_b) = parse_bytes_user(quote!({ prefer_stack: false }, b"poly")).expect("parse");
    let a = b_user(b"poly".to_vec(), tier_a)
        .expect("b_user")
        .to_string();
    let b = b_user(b"poly".to_vec(), tier_b)
        .expect("b_user")
        .to_string();
    assert!(!a.is_empty() && !b.is_empty());
    assert_ne!(a, b, "两次展开应多态不同");
}

#[test]
fn none_group_wrapped_prefix_unwrapped() {
    // 模拟用户以 `:expr` / `:literal` 元变量转发时的 None 分组包裹
    let wrapped = quote!({ max_latency: 40 }, "hi");
    let (payload, tier) = split_user_input(
        proc_macro2::Group::new(proc_macro2::Delimiter::None, wrapped.clone()).stream(),
        "s_user!",
    )
    .expect("none-wrapped input");
    assert_eq!(payload.to_string(), quote!("hi").to_string());
    assert_eq!(tier.max_latency, 40);
}
