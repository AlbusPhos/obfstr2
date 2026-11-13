use crate::storage::STORAGES;
use crate::tests::obfstr2_dep;
use lib_unknown::dyntest::DnyRun;
use lib_unknown::rand::core::{fill_bytes, shuffle};
use quote::{format_ident, quote};
use std::fs;
use std::time::Duration;

#[derive(Clone, Copy)]
struct StorageMetricRecord {
    bin_size: f64,
    run_ms: f64,
    build_ms: f64,
}

#[derive(Clone)]
struct StorageAggregatedStat {
    med_size: f64,
    med_run: f64,
    med_build: f64,
    calibrated_score: f64,
}

#[test]
#[ignore = "需5到10分钟的STORAGES多维度压测与latency校准"]
#[allow(clippy::needless_range_loop)]
fn test_storages_latency() {
    let mut runner = DnyRun::new("fn main() {}", &obfstr2_dep(true));
    runner.release(true);
    let ret = runner.build(Some(Duration::from_secs(15)));
    if !ret.ok {
        println!("{}", ret);
        panic!("初始化构建测试环境失败");
    }

    // 覆盖短载荷、全策略交集区 (128~384)、中长载荷
    let sample_sizes = &[8, 16, 64, 128, 256, 384, 512, 1024, 2048, 10240];
    const ROUNDS: usize = 10;
    const BENCH_ITERS: usize = 1000;

    // 权重配置：Run 50%, Size 30%, Build 20%
    const WEIGHT_RUN: f64 = 0.35;
    const WEIGHT_SIZE: f64 = 0.40;
    const WEIGHT_BUILD: f64 = 0.25;

    let num_storages = STORAGES.len();
    let num_samples = sample_sizes.len();

    let mut raw_data: Vec<Vec<Vec<StorageMetricRecord>>> =
        vec![vec![Vec::with_capacity(ROUNDS); num_samples]; num_storages];

    println!(
        "\n>>> 开始执行 {} 轮 Storage 压测采样 (解码循环: {} 次, 随机打乱执行序列)...",
        ROUNDS, BENCH_ITERS
    );

    for round in 1..=ROUNDS {
        println!(
            ">>> [Round {:02}/{:02}] 正在采集全量存储策略测试数据...",
            round, ROUNDS
        );

        for (sample_idx, &sample_size) in sample_sizes.iter().enumerate() {
            let mut data = vec![0u8; sample_size];
            fill_bytes(data.as_mut());
            let expected_hex: String = data.iter().map(|b| format!("{:02x}", b)).collect();

            let storage_indices: Vec<usize> = shuffle((0..num_storages).collect());

            's: for &s_idx in &storage_indices {
                let storage = &STORAGES[s_idx];
                if !(storage.support)(sample_size) {
                    continue 's;
                }

                let ident = format_ident!("d");
                let family = quote!(::obfstr2::types::buffer::StackBytes);
                // 生成当前 storage 的 AST 片段
                let storage_ast = (storage.ast)(&ident, &family, &data).to_string();

                // 独立词法作用域：防止 static 变量重名及局部变量生命周期穿透
                let main_code = format!(
                    r#"
fn main() {{
    // 1. 正确性校验
    {{
        {storage_ast}
        let hex_str: String = d.iter().map(|b| format!("{{:02x}}", b)).collect();
        println!("__HEX:{{}}", hex_str);
    }}

    // 2. 纳秒级性能压测
    let start = ::std::time::Instant::now();
    for _ in 0..::core::hint::black_box({BENCH_ITERS}) {{
        {{
            {storage_ast}
            ::core::hint::black_box(&d);
        }}
    }}
    let elapsed_ns = start.elapsed().as_nanos();
    println!("__TIMING_NS:{{}}", elapsed_ns);
}}
"#,
                    storage_ast = storage_ast,
                    BENCH_ITERS = BENCH_ITERS
                );

                runner.reset_main_code(&main_code);
                let ret = runner.run(Some(Duration::from_secs(10)));
                assert!(
                    ret.ok,
                    "存储策略 [{}] (len={}) 执行失败:\n{}",
                    storage.name, sample_size, ret
                );

                // 解析协议输出
                let mut actual_hex = String::new();
                let mut pure_run_ms = ret.run_duration.as_secs_f64() * 1000.0;

                for line in ret.stdout.lines() {
                    let line = line.trim();
                    if let Some(h) = line.strip_prefix("__HEX:") {
                        actual_hex = h.to_string();
                    } else if let Some(ns_str) = line.strip_prefix("__TIMING_NS:")
                        && let Ok(ns) = ns_str.trim().parse::<u128>()
                    {
                        pure_run_ms = ns as f64 / 1_000_000.0;
                    }
                }

                assert_eq!(
                    actual_hex, expected_hex,
                    "策略 [{}] 还原明文与原始数据不匹配!",
                    storage.name
                );

                let bin_size = fs::metadata(runner.bin_path())
                    .map(|m| m.len())
                    .unwrap_or(0) as f64;
                let build_ms = ret.build_duration.as_secs_f64() * 1000.0;

                raw_data[s_idx][sample_idx].push(StorageMetricRecord {
                    bin_size,
                    run_ms: pure_run_ms,
                    build_ms,
                });
            }
        }
    }

    // 中位数计算与归一化闭包
    let median = |mut vals: Vec<f64>| -> f64 {
        if vals.is_empty() {
            return 0.0;
        }
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = vals.len() / 2;
        if vals.len().is_multiple_of(2) {
            (vals[mid - 1] + vals[mid]) / 2.0
        } else {
            vals[mid]
        }
    };

    let norm = |val: f64, min: f64, max: f64| -> f64 {
        if (max - min).abs() > f64::EPSILON {
            (val - min) / (max - min)
        } else {
            0.0
        }
    };

    // 阶段 1：组内稳健中位数聚合
    let mut agg_matrix: Vec<Vec<Option<StorageAggregatedStat>>> =
        vec![vec![None; num_samples]; num_storages];

    for s_idx in 0..num_storages {
        for smp_idx in 0..num_samples {
            let runs = &raw_data[s_idx][smp_idx];
            if !runs.is_empty() {
                agg_matrix[s_idx][smp_idx] = Some(StorageAggregatedStat {
                    med_size: median(runs.iter().map(|r| r.bin_size).collect()),
                    med_run: median(runs.iter().map(|r| r.run_ms).collect()),
                    med_build: median(runs.iter().map(|r| r.build_ms).collect()),
                    calibrated_score: 0.0,
                });
            }
        }
    }

    // 阶段 2：跨 Storage 归一化评分 (仅在当前长度下支持的策略之间对比)
    for smp_idx in 0..num_samples {
        let valid_stats: Vec<&StorageAggregatedStat> = (0..num_storages)
            .filter_map(|s| agg_matrix[s][smp_idx].as_ref())
            .collect();

        if valid_stats.is_empty() {
            continue;
        }

        let min_size = valid_stats
            .iter()
            .map(|s| s.med_size)
            .fold(f64::INFINITY, f64::min);
        let max_size = valid_stats
            .iter()
            .map(|s| s.med_size)
            .fold(f64::NEG_INFINITY, f64::max);

        // 使用对数尺度平滑解码耗时的巨大差异 (例如 bytes 对比 hb2/pem)
        let min_run_log = valid_stats
            .iter()
            .map(|s| (s.med_run + 1.0).ln())
            .fold(f64::INFINITY, f64::min);
        let max_run_log = valid_stats
            .iter()
            .map(|s| (s.med_run + 1.0).ln())
            .fold(f64::NEG_INFINITY, f64::max);

        let min_build = valid_stats
            .iter()
            .map(|s| s.med_build)
            .fold(f64::INFINITY, f64::min);
        let max_build = valid_stats
            .iter()
            .map(|s| s.med_build)
            .fold(f64::NEG_INFINITY, f64::max);

        for s_idx in 0..num_storages {
            if let Some(stat) = &mut agg_matrix[s_idx][smp_idx] {
                let n_size = norm(stat.med_size, min_size, max_size);
                let n_run = norm((stat.med_run + 1.0).ln(), min_run_log, max_run_log);
                let n_build = norm(stat.med_build, min_build, max_build);

                let composite_score =
                    n_run * WEIGHT_RUN + n_size * WEIGHT_SIZE + n_build * WEIGHT_BUILD;

                // 映射至 1.0 ..= 100.0 区间
                stat.calibrated_score = composite_score * 99.0 + 1.0;
            }
        }
    }

    // 输出样本详细报告
    for (sample_idx, &sample_size) in sample_sizes.iter().enumerate() {
        let header = format!(
            " Storage 样本 #{idx} (长度: {len} 字节, 解码循环: {iters} 次, 聚合 {rounds} 轮采样) 校准报告 ",
            idx = sample_idx + 1,
            len = sample_size,
            iters = BENCH_ITERS,
            rounds = ROUNDS
        );
        println!("\n{:=^115}", header);
        println!(
            "{:<15} | {:<10} | {:<10} | {:<12} | {:<12} | {:<15}",
            "Storage Name",
            "Size (KB)",
            "Med Run(ms)",
            "Med Build(ms)",
            "Old Latency",
            "Calibrated Score"
        );
        println!("{:-^115}", "");

        for (s_idx, storage) in STORAGES.iter().enumerate() {
            if let Some(stat) = &agg_matrix[s_idx][sample_idx] {
                println!(
                    "{:<15} | {:<10.1} | {:<10.3} | {:<12.1} | {:<12} | \x1b[1;32m{:<5.1}\x1b[0m",
                    storage.name,
                    stat.med_size / 1024.0,
                    stat.med_run,
                    stat.med_build,
                    storage.latency,
                    stat.calibrated_score
                );
            } else {
                println!(
                    "{:<15} | {:<10} | {:<10} | {:<12} | {:<12} | \x1b[90m{:<15}\x1b[0m",
                    storage.name, "N/A", "N/A", "N/A", storage.latency, "Unsupported"
                );
            }
        }
        println!("{:=^115}\n", "");
    }

    // 输出最终横向推荐表
    println!(
        "\n{:=^135}",
        " 各长度样本稳健中位数校准 Latency 横向对比与最终推荐表 (Run 50%, Size 30%, Build 20%) "
    );
    print!("{:<15} | {:<8}", "Storage Name", "Old Lat");
    for sz in sample_sizes {
        print!(" | Len {:<5}", sz);
    }
    println!(" | \x1b[1;32m{:<9}\x1b[0m", "Final Rec");
    println!("{:-^135}", "");

    for (s_idx, storage) in STORAGES.iter().enumerate() {
        print!("{:<15} | {:<8}", storage.name, storage.latency);
        let mut sum_score = 0.0;
        let mut valid_samples = 0;

        for smp_idx in 0..num_samples {
            if let Some(stat) = &agg_matrix[s_idx][smp_idx] {
                let sc = stat.calibrated_score;
                sum_score += sc;
                valid_samples += 1;
                print!(" | {:<9.1}", sc);
            } else {
                print!(" | \x1b[90m{:<9}\x1b[0m", "N/A");
            }
        }

        if valid_samples > 0 {
            let final_rec = (sum_score / valid_samples as f64).round() as u8;
            println!(" | \x1b[1;32m{:<9}\x1b[0m", final_rec);
        } else {
            println!(" | \x1b[90m{:<9}\x1b[0m", "N/A");
        }
    }
    println!("{:=^135}\n", "");
}
