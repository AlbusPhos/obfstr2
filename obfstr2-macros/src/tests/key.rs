// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 动态密钥档解析层单测：`build_key` 形态、`run_key` 校验与双花括号前缀切分。

#![allow(clippy::expect_used)]

use crate::parse::*;
use quote::{ToTokens, quote};

/// 提取解析错误信息（`KeyPair` / `TierParams` 等载荷类型无 `Debug`，
/// 不能对整个 `Result` 调 `unwrap_err`）。
fn err_of<T>(r: syn::Result<T>) -> String {
    match r {
        Ok(_) => panic!("应当解析失败"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn build_key_literal_forms() {
    for (input, expected) in [
        (quote!("secret"), b"secret".as_slice()),
        (quote!(b"secret"), b"secret".as_slice()),
        (
            quote!([0x73, 0x65, 0x63, 0x72, 0x65, 0x74]),
            b"secret".as_slice(),
        ),
    ] {
        let (s, keys) = parse_str_key(quote!({ build_key: #input, run_key: key }, "hi"), "s_key!")
            .expect("literal build_key");
        assert_eq!(s, "hi");
        assert_eq!(keys.build_key, expected);
        assert!(keys.build_key_env.is_none());
        assert_eq!(keys.run_key.to_token_stream().to_string(), "key");
    }
}

#[test]
fn build_key_env_resolves() {
    const VAR: &str = "OBFSTR2_TEST_BUILD_KEY_PARSE_OK";
    // SAFETY: 测试使用独占变量名，且在同线程内设置 / 删除，不会与其他测试冲突。
    unsafe { std::env::set_var(VAR, "env-secret") };
    let (_, keys) = parse_str_key(
        quote!({ build_key: env!(#VAR), run_key: key }, "hi"),
        "s_key!",
    )
    .expect("env build_key");
    assert_eq!(keys.build_key, b"env-secret");
    assert_eq!(keys.build_key_env.expect("应记录 env 变量名").value(), VAR);
    // SAFETY: 同上，独占测试变量。
    unsafe { std::env::remove_var(VAR) };
}

#[test]
fn key_user_accepts_both_brace_layouts() {
    let (s, keys, tier) = parse_str_key_user(quote!(
        { build_key: "k", run_key: key },
        { max_latency: 50 },
        "hi"
    ))
    .expect("comma-separated key_user");
    assert_eq!(s, "hi");
    assert_eq!(keys.build_key, b"k");
    assert_eq!(tier.max_latency, 50);

    let (s, keys, tier) = parse_str_key_user(quote!(
        { build_key: "k", run_key: key }
        { prefer_stack: true },
        "hi"
    ))
    .expect("adjacent-brace key_user");
    assert_eq!(s, "hi");
    assert_eq!(keys.build_key, b"k");
    assert!(tier.prefer_stack);
}

#[test]
fn run_key_rejects_compile_time_forms() {
    for (idx, expr) in [
        quote!("k"),
        quote!(b"k"),
        quote!(&[0x6b]),
        quote!([0x6b]),
        quote!("k".as_bytes()),
        quote!(&b"k"[..]),
        quote!({ "k" }),
        quote!(if flag { "k" } else { "v" }),
        quote!(match flag {
            _ => "k",
        }),
        quote!((|| b"k")()),
        quote!(concat!("k")),
        quote!(env!("K")),
    ]
    .into_iter()
    .enumerate()
    {
        let err = match parse_str_key(
            quote!({ build_key: "secret", run_key: #expr }, "hi"),
            "s_key!",
        ) {
            Ok(_) => panic!("case #{idx} ({expr}) 应解析失败"),
            Err(e) => e.to_string(),
        };
        assert!(
            err.contains("`run_key` 不能为字面量"),
            "表达式 {expr} 应被拒绝，实际: {err}"
        );
    }
}

#[test]
fn run_key_accepts_runtime_forms() {
    for expr in [
        quote!(key),
        quote!(key.as_bytes()),
        quote!(get_key()),
        quote!(&buf[..]),
        quote!(env::var("K").unwrap().into_bytes()),
    ] {
        let (_, keys) = parse_str_key(
            quote!({ build_key: "secret", run_key: #expr }, "hi"),
            "s_key!",
        )
        .expect("runtime run_key");
        assert!(!keys.build_key.is_empty());
    }
}

#[test]
fn key_rejects_missing_unknown_and_empty() {
    let err = err_of(parse_str_key(quote!({ run_key: key }, "hi"), "s_key!"));
    assert!(err.contains("缺少必填字段 `build_key`"), "实际: {err}");

    let err = err_of(parse_str_key(quote!({ build_key: "k" }, "hi"), "s_key!"));
    assert!(err.contains("缺少必填字段 `run_key`"), "实际: {err}");

    let err = err_of(parse_str_key(
        quote!({ build_key: "k", run_key: key, foo: 1 }, "hi"),
        "s_key!",
    ));
    assert!(err.contains("未知的密钥配置字段"), "实际: {err}");

    let err = err_of(parse_str_key(
        quote!({ build_key: [], run_key: key }, "hi"),
        "s_key!",
    ));
    assert!(err.contains("密钥内容不能为空"), "实际: {err}");
}

#[test]
fn sfmt_key_prefix_preserves_variadic_args() {
    let (keys, rest) = split_key_prefix(
        quote!({ build_key: "k", run_key: key }, "hello {}, {}", name, { 1 + 2 }),
        "s_fmt_key!",
    )
    .expect("sfmt key split");
    assert_eq!(keys.build_key, b"k");
    assert_eq!(
        rest.to_string(),
        quote!("hello {}, {}", name, { 1 + 2 }).to_string()
    );
}
