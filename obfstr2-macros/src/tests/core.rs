// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `crate::core` 编排内核单测（自 `core.rs` 内联测试模块迁移）：
//! 包装概率函数、`security_factor` 层数缩放、折叠树收敛边界与三档选层批量模拟。

#![allow(clippy::expect_used)]

use crate::core::{
    ChunkNode, FOLD_GROUP_MAX, TIER_BALANCED, TIER_HIGH, TIER_LOW, TierParams, effective_wrap_pct,
    encrypt_layers, fold_chunk_nodes,
};
use crate::crypto::CRYPTOS;
use lib_unknown::rand::core::random;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

#[test]
fn effective_wrap_pct_starts_at_base() {
    assert_eq!(effective_wrap_pct(15, 0), 15);
    assert_eq!(effective_wrap_pct(50, 0), 50);
    assert_eq!(effective_wrap_pct(85, 0), 85);
    assert_eq!(effective_wrap_pct(0, 999), 0);
}

/// `security_factor` 应按缩放后的单层安全度上界影响层数：
/// - 均衡档可用原语的单层安全度上界为 57（`mse`），factor=100 时单层即达
///   `min_security=63`，层数受 `target_layers`（≤2）约束；
/// - factor=0.1 时单层贡献上界 5.7%，达到 63% 需 `ln(0.37)/ln(0.943) ≈ 17` 层
///   （数学下界，与随机原语选择无关）。
#[test]
fn security_factor_scales_layer_count() {
    let fast = TierParams {
        security_factor: 100.0,
        ..TIER_BALANCED.clone()
    };
    let (_, layers) = encrypt_layers(vec![0u8; 32], &fast, random::<u64>(), "f100")
        .expect("encrypt with security_factor=100");
    assert!(
        layers.len() <= 2,
        "factor > 1 应减少层数，实际 {} 层",
        layers.len()
    );

    let slow = TierParams {
        security_factor: 0.1,
        ..TIER_BALANCED.clone()
    };
    let (_, layers) = encrypt_layers(vec![0u8; 32], &slow, random::<u64>(), "f01")
        .expect("encrypt with security_factor=0.1");
    assert!(
        layers.len() >= 16,
        "factor < 1 应增加层数，实际 {} 层",
        layers.len()
    );
}

/// 折叠树必须把根节点数收敛到 `FOLD_ROOT_MAX`，且小块输入不生成任何折叠壳。
#[test]
fn fold_chunk_nodes_bounds_root_spine() {
    let main_ty = quote!(u8);
    let bytes = format_ident!("__b");
    let ghost = format_ident!("__g");
    let make = |n: usize| {
        (0..n)
            .map(|i| ChunkNode {
                stmt: quote!(let _ = #i as u8;),
            })
            .collect::<Vec<_>>()
    };

    let mut prelude = TokenStream2::new();
    let roots = fold_chunk_nodes(
        make(1),
        1,
        &main_ty,
        &bytes,
        &ghost,
        &mut prelude,
        &TIER_LOW,
    );
    assert_eq!(roots.len(), 1);
    assert!(
        !prelude.to_string().contains("__fold_"),
        "单块不应生成折叠壳"
    );

    let mut prelude = TokenStream2::new();
    let roots = fold_chunk_nodes(
        make(1000),
        2,
        &main_ty,
        &bytes,
        &ghost,
        &mut prelude,
        &TIER_LOW,
    );
    assert!(
        roots.len() <= FOLD_GROUP_MAX,
        "根节点数应收敛到 {FOLD_GROUP_MAX}"
    );
    assert!(
        prelude.to_string().contains("__fold_"),
        "大规模必须生成折叠函数"
    );
}

/// 批量模拟三档 Tier 的加密选层过程（`encrypt_layers`），用于离线评估配置合理性。
#[test]
fn simulate_crypto_selection_across_tiers() {
    use lib_unknown::rand::core::fill_bytes;
    use std::collections::{BTreeMap, HashSet};

    const ITERATIONS: usize = 100;
    const CHUNK_SIZES: [usize; 9] = [1, 2, 4, 8, 16, 32, 64, 256, 1024];

    struct TierCase {
        name: &'static str,
        tier: TierParams,
    }

    let cases = [
        TierCase {
            name: "LOW",
            tier: TIER_LOW,
        },
        TierCase {
            name: "BALANCED",
            tier: TIER_BALANCED,
        },
        TierCase {
            name: "HIGH",
            tier: TIER_HIGH,
        },
    ];

    for case in &cases {
        let tier = case.tier.clone();
        println!(
            "\n{:=^100}",
            format!(
                " Tier {} (max_latency={}, min_security={}, pad_short={}) ",
                case.name, tier.max_latency, tier.min_security, tier.pad_short
            )
        );

        // 原语使用计数（跨所有 chunk size 聚合）
        let mut usage: BTreeMap<&'static str, u64> = BTreeMap::new();
        let mut layer_counts: Vec<usize> = Vec::new();
        let mut securities: Vec<u8> = Vec::new();
        let mut latency_sums: Vec<u32> = Vec::new();
        let mut fail = 0u64;
        let mut total = 0u64;

        // 按 chunk size 细分的层数 / 安全度均值，用于观察规模效应（是否随长度漂移）
        let mut per_size_layers: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        let mut per_size_security: BTreeMap<usize, Vec<u8>> = BTreeMap::new();

        for &size in &CHUNK_SIZES {
            for _ in 0..ITERATIONS {
                total += 1;
                let mut data = vec![0u8; size];
                if size > 0 {
                    fill_bytes(&mut data);
                }
                match encrypt_layers(data, &tier, random::<u64>(), "sim") {
                    Ok((_, layers)) => {
                        let pre_padded = tier.pad_short && size < 16;
                        let mut vuln = 1.0_f64;
                        let mut lat_sum: u32 = 0;
                        for (idx, l) in layers.iter().enumerate() {
                            *usage.entry(l.crypto.name).or_insert(0) += 1;
                            lat_sum += l.crypto.latency as u32;
                            // 跳过前置 pkcs7_16 预填充层的安全度贡献，与 encrypt_layers 口径一致
                            // （含 security_factor 缩放）
                            if !(pre_padded && idx == 0 && l.crypto.name == "pkcs7_16") {
                                vuln *= (100.0
                                    - (l.crypto.security as f64 * tier.security_factor)
                                        .clamp(0.0, 100.0))
                                    / 100.0;
                            }
                        }
                        let sec = (100.0 * (1.0 - vuln)) as u8;

                        layer_counts.push(layers.len());
                        securities.push(sec);
                        latency_sums.push(lat_sum);
                        per_size_layers.entry(size).or_default().push(layers.len());
                        per_size_security.entry(size).or_default().push(sec);
                    }
                    Err(_e) => {
                        fail += 1;
                    }
                }
            }
        }

        let avg = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
        let avg_u = |v: &[usize]| avg(&v.iter().map(|&x| x as f64).collect::<Vec<_>>());
        let avg_u8 = |v: &[u8]| avg(&v.iter().map(|&x| x as f64).collect::<Vec<_>>());
        let avg_u32 = |v: &[u32]| avg(&v.iter().map(|&x| x as f64).collect::<Vec<_>>());

        println!(
            "调用次数: {total}, 失败次数: {fail} ({:.3}%)",
            fail as f64 / total as f64 * 100.0
        );
        println!(
            "层数: min={} avg={:.2} max={}",
            layer_counts.iter().copied().min().unwrap_or(0),
            avg_u(&layer_counts),
            layer_counts.iter().copied().max().unwrap_or(0),
        );
        let below_target = securities
            .iter()
            .filter(|&&s| s < tier.min_security)
            .count();
        println!(
            "组合安全度: min={} avg={:.2} max={}, 未达标占比={:.2}% (目标>={})",
            securities.iter().copied().min().unwrap_or(0),
            avg_u8(&securities),
            securities.iter().copied().max().unwrap_or(0),
            below_target as f64 / securities.len().max(1) as f64 * 100.0,
            tier.min_security,
        );
        println!(
            "单 chunk 层延迟之和估算: min={} avg={:.2} max={}",
            latency_sums.iter().copied().min().unwrap_or(0),
            avg_u32(&latency_sums),
            latency_sums.iter().copied().max().unwrap_or(0),
        );

        // 按 chunk size 展示规模效应（层数 / 安全度是否随 size 增长而漂移）
        println!(
            "\n{:<10} | {:<10} | {:<10}",
            "ChunkSize", "AvgLayers", "AvgSecurity"
        );
        println!("{:-<36}", "");
        for &size in &CHUNK_SIZES {
            let layers = per_size_layers.get(&size).cloned().unwrap_or_default();
            let secs = per_size_security.get(&size).cloned().unwrap_or_default();
            println!(
                "{:<10} | {:<10.2} | {:<10.2}",
                size,
                avg_u(&layers),
                avg_u8(&secs),
            );
        }

        // 原语使用频率分布
        let total_usage: u64 = usage.values().sum();
        let mut rows: Vec<(&str, u64)> = usage.iter().map(|(&k, &v)| (k, v)).collect();
        rows.sort_by_key(|&(_, count)| std::cmp::Reverse(count));

        println!(
            "\n{:<15} | {:<10} | {:<10} | {:<10} | {:<10}",
            "Crypto", "Count", "Freq%", "Security", "Latency"
        );
        println!("{:-<65}", "");
        for (name, count) in &rows {
            if let Some(crypto) = CRYPTOS.iter().find(|c| c.name == *name) {
                println!(
                    "{:<15} | {:<10} | {:<9.2}% | {:<10} | {:<10}",
                    name,
                    count,
                    *count as f64 / total_usage.max(1) as f64 * 100.0,
                    crypto.security,
                    crypto.latency,
                );
            }
        }

        let available: HashSet<&str> = CRYPTOS
            .iter()
            .filter(|c| c.latency <= tier.max_latency)
            .map(|c| c.name)
            .collect();
        let used: HashSet<&str> = rows.iter().map(|&(n, _)| n).collect();
        let never_used: Vec<&&str> = available.iter().filter(|n| !used.contains(*n)).collect();
        if !never_used.is_empty() {
            println!(
                "\n⚠ 从未被选中的可用原语（latency<={}）: {:?}",
                tier.max_latency, never_used
            );
        }

        assert_eq!(
            fail, 0,
            "tier {} 存在无法完成加密选层的 chunk 长度",
            case.name
        );
    }
}
