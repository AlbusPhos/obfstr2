// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// obfstr2-macros/src/tests/crypto.rs
use crate::crypto::{CRYPTOS, LayerCtx};
use crate::storage::STORAGES;
use crate::tests::obfstr2_dep;
use lib_unknown::dyntest::DnyRun;
use lib_unknown::rand::core::{fill_bytes, random, shuffle};
use quote::{format_ident, quote};
use std::env;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[test]
#[allow(clippy::expect_used)]
fn mse2_primitive_roundtrip() {
    use lib_unknown::crypto::mse2::{imse2, mse2};

    let key = 0xDEAD_BEEF_CAFE_BABE_u64;
    for len in [1usize, 8, 15, 16, 63, 64, 65, 255] {
        let mut plaintext = vec![0u8; len];
        fill_bytes(plaintext.as_mut());

        let mut ciphertext = vec![0u8; len + 64];
        let ct_len = mse2(&plaintext, &mut ciphertext, key, 0x1122_3344)
            .expect("mse2 encryption should succeed");

        let mut decrypted = vec![0u8; len];
        let pt_len = imse2(&ciphertext[..ct_len], &mut decrypted, key, 0x1122_3344)
            .expect("mse2 decryption should succeed");

        assert_eq!(pt_len, len);
        assert_eq!(decrypted, plaintext);
    }
}

#[test]
#[allow(clippy::expect_used)]
fn chacha20_primitive_roundtrip() {
    use lib_unknown::crypto::chacha20::chacha20_1;

    let key = 0x0123_4567_89AB_CDEF_u64;
    let nonce = 0xA5u8;
    for len in [1usize, 8, 64, 255] {
        let mut plaintext = vec![0u8; len];
        fill_bytes(plaintext.as_mut());

        let mut buf = plaintext.clone();
        chacha20_1(&mut buf, key, nonce).expect("chacha20 encryption should succeed");
        assert_ne!(buf, plaintext);

        chacha20_1(&mut buf, key, nonce).expect("chacha20 decryption should succeed");
        assert_eq!(buf, plaintext);
    }
}

#[test]
#[allow(clippy::expect_used)]
fn dynamic_box_inversion_roundtrip() {
    use lib_unknown::crypto::core::{boxs, mix};

    for seed in [0u64, 1, 0xDEAD_BEEF, u64::MAX] {
        let s = boxs::gen_s(mix::mix64(seed));
        let s_inv = boxs::inv_s(&s).expect("generated s-box must be invertible");
        for x in 0..=u8::MAX {
            assert_eq!(boxs::s8(boxs::s8(x, &s), &s_inv), x);
        }

        let p = boxs::gen_p(mix::mix64(seed));
        let p_inv = boxs::inv_p(&p).expect("generated p-box must be invertible");
        for x in 0..=u8::MAX {
            assert_eq!(boxs::p8(boxs::p8(x, &p), &p_inv), x);
        }
    }
}

#[derive(Clone, Copy)]
struct MetricRecord {
    bin_size: f64,
    run_ms: f64,
    build_ms: f64,
}

struct AggregatedStat {
    med_size: f64,
    med_run: f64,
    med_build: f64,
    calibrated_score: f64,
}

#[test]
#[ignore = "5到10分钟的CRYPTOS验证和latency矫正"]
#[allow(clippy::needless_range_loop)]
fn test_cryptos_latency() {
    let mut runner = DnyRun::new("fn main() {}", &obfstr2_dep(false));
    runner.release(true);
    let ret = runner.build(Some(Duration::from_secs(10)));
    if !ret.ok {
        println!("{}", ret);
        panic!("初始化构建测试环境失败");
    }

    let sample_sizes = &[0, 1, 8, 15, 32, 127, 128, 512, 1024, 2047];
    const ROUNDS: usize = 10;
    const BENCH_ITERS: usize = 1000;

    // 权重配置：Run 50%, Size 30%, Build 20%
    const WEIGHT_RUN: f64 = 0.50;
    const WEIGHT_SIZE: f64 = 0.30;
    const WEIGHT_BUILD: f64 = 0.20;

    let num_cryptos = CRYPTOS.len();
    let num_samples = sample_sizes.len();

    let mut raw_data: Vec<Vec<Vec<MetricRecord>>> =
        vec![vec![Vec::with_capacity(ROUNDS); num_samples]; num_cryptos];

    println!(
        "\n>>> 开始执行 {} 轮多样本采样压测 (随机混洗执行序列，解密循环: {} 次)...",
        ROUNDS, BENCH_ITERS
    );

    for round in 1..=ROUNDS {
        println!(
            ">>> [Round {:02}/{:02}] 正在采集全量原语测试数据...",
            round, ROUNDS
        );

        for (sample_idx, &sample_size) in sample_sizes.iter().enumerate() {
            let mut data = vec![0u8; sample_size];
            if sample_size > 0 {
                fill_bytes(data.as_mut());
            }

            let crypto_indices: Vec<usize> = shuffle((0..num_cryptos).collect());

            'c: for &c_idx in &crypto_indices {
                let crypto = &CRYPTOS[c_idx];
                if !(crypto.support)(sample_size) {
                    continue 'c;
                }

                let mut d = data.clone();
                let key = random::<u64>();
                let iv = random::<u8>();

                let mut ctx = LayerCtx {
                    type_path: quote!(::obfstr2::types::buffer::HeapBytes),
                    key,
                    key_ident: format_ident!("k"),
                    iv,
                    in_size: d.len(),
                    out_size: d.len(),
                    ident_tag: "".to_string(),
                };
                d = (crypto.enc)(&mut ctx, d);
                assert_eq!(d.len(), ctx.out_size);
                assert_eq!(data.len(), ctx.in_size);

                let storage = (STORAGES[0].ast)(
                    &format_ident!("d"),
                    &quote!(::obfstr2::types::buffer::StackBytes),
                    &d,
                )
                .to_string();
                let (dec, ident) = (crypto.dec)(&ctx, format_ident!("d"));

                let main = format!(
                    "fn main() {{ \
                            {storage} \
                            let k: u64 = {key}; \
                            {dec} \
                            let hex_str: String = {ident}.iter().map(|b| format!(\"{{:02x}}\", b)).collect(); \
                            println!(\"__HEX:{{}}\", hex_str); \
                            let start = ::std::time::Instant::now(); \
                            for _ in 0..::core::hint::black_box({BENCH_ITERS}) {{ \
                                {storage} \
                                {dec} \
                                ::core::hint::black_box({ident}); \
                            }} \
                            let elapsed_ns = start.elapsed().as_nanos(); \
                            println!(\"__TIMING_NS:{{}}\", elapsed_ns); \
                        }}",
                    storage = storage,
                    key = key,
                    dec = dec.to_string(),
                    ident = ident.to_string(),
                );

                runner.reset_main_code(&main);
                let ret = runner.run(Some(Duration::from_secs(10)));
                assert!(ret.ok, "原语执行失败: {}\n{}", crypto.name, ret);

                // 协议解析与正确性验证
                let expected_hex: String = data.iter().map(|b| format!("{:02x}", b)).collect();
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
                    "原语 [{}] 解密明文与原始明文不匹配!",
                    crypto.name
                );

                let bin_size = fs::metadata(runner.bin_path())
                    .map(|m| m.len())
                    .unwrap_or(0) as f64;
                let build_ms = ret.build_duration.as_secs_f64() * 1000.0;

                raw_data[c_idx][sample_idx].push(MetricRecord {
                    bin_size,
                    run_ms: pure_run_ms,
                    build_ms,
                });
            }
        }
    }

    // 稳健中位数辅助闭包（增加空切片防护）
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

    // 阶段 1：组内聚合 —— 使用 Option<AggregatedStat> 区分是否支持该样本长度
    let mut agg_matrix: Vec<Vec<Option<AggregatedStat>>> = Vec::with_capacity(num_cryptos);
    for c_idx in 0..num_cryptos {
        let mut sample_stats = Vec::with_capacity(num_samples);
        for s_idx in 0..num_samples {
            let runs = &raw_data[c_idx][s_idx];
            if runs.is_empty() {
                sample_stats.push(None);
            } else {
                sample_stats.push(Some(AggregatedStat {
                    med_size: median(runs.iter().map(|r| r.bin_size).collect()),
                    med_run: median(runs.iter().map(|r| r.run_ms).collect()),
                    med_build: median(runs.iter().map(|r| r.build_ms).collect()),
                    calibrated_score: 0.0,
                }));
            }
        }
        agg_matrix.push(sample_stats);
    }

    // 阶段 2：跨原语归一化 —— 仅在支持当前样本长度的原语之间计算极值与得分
    for s_idx in 0..num_samples {
        let valid_stats: Vec<&AggregatedStat> = (0..num_cryptos)
            .filter_map(|c| agg_matrix[c][s_idx].as_ref())
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

        for c_idx in 0..num_cryptos {
            if let Some(stat) = &mut agg_matrix[c_idx][s_idx] {
                let n_size = norm(stat.med_size, min_size, max_size);
                let n_run = norm((stat.med_run + 1.0).ln(), min_run_log, max_run_log);
                let n_build = norm(stat.med_build, min_build, max_build);

                let composite_score =
                    n_run * WEIGHT_RUN + n_size * WEIGHT_SIZE + n_build * WEIGHT_BUILD;
                // 全程保留 f64 精度，映射到 1.0 ..= 100.0 区间
                stat.calibrated_score = composite_score * 99.0 + 1.0;
            }
        }
    }

    // 输出按长度细分的稳健报告
    for (sample_idx, &sample_size) in sample_sizes.iter().enumerate() {
        let header = format!(
            " 样本 #{idx} (长度: {len} 字节, 循环: {iters} 次, 稳健中位数聚合 {rounds} 轮采样) 校准报告 ",
            idx = sample_idx + 1,
            len = sample_size,
            iters = BENCH_ITERS,
            rounds = ROUNDS
        );
        println!("\n{:=^115}", header);
        println!(
            "{:<15} | {:<8} | {:<10} | {:<10} | {:<12} | {:<12} | {:<15}",
            "Crypto Name",
            "Security",
            "Size (KB)",
            "Med Run(ms)",
            "Med Build(ms)",
            "Old Latency",
            "Calibrated Score"
        );
        println!("{:-^115}", "");

        for (c_idx, crypto) in CRYPTOS.iter().enumerate() {
            if let Some(stat) = &agg_matrix[c_idx][sample_idx] {
                println!(
                    "{:<15} | {:<8} | {:<10.1} | {:<10.3} | {:<12.1} | {:<12} | \x1b[1;32m{:<5.1}\x1b[0m",
                    crypto.name,
                    crypto.security,
                    stat.med_size / 1024.0,
                    stat.med_run,
                    stat.med_build,
                    crypto.latency,
                    stat.calibrated_score
                );
            } else {
                println!(
                    "{:<15} | {:<8} | {:<10} | {:<10} | {:<12} | {:<12} | \x1b[90m{:<15}\x1b[0m",
                    crypto.name,
                    crypto.security,
                    "N/A",
                    "N/A",
                    "N/A",
                    crypto.latency,
                    "Unsupported"
                );
            }
        }
        println!("{:=^115}\n", "");
    }

    // 最终全量横向推荐对比表（动态对齐全部 sample_sizes 列）
    println!(
        "\n{:=^135}",
        " 各长度样本稳健中位数校准 Latency 横向对比与最终推荐表 (Run 50%, Size 30%, Build 20%) "
    );
    print!("{:<15} | {:<8}", "Crypto Name", "Old Lat");
    for sz in sample_sizes {
        print!(" | Len {:<5}", sz);
    }
    println!(" | \x1b[1;32m{:<9}\x1b[0m", "Final Rec");
    println!("{:-^135}", "");

    for (c_idx, crypto) in CRYPTOS.iter().enumerate() {
        print!("{:<15} | {:<8}", crypto.name, crypto.latency);
        let mut sum_score = 0.0;
        let mut valid_samples = 0;

        for s_idx in 0..num_samples {
            if let Some(stat) = &agg_matrix[c_idx][s_idx] {
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

const ANGR_RUST_TEMPLATE: &str = r#"
#![no_std]
#![no_main]
#![allow(unsafe_code)]
#![allow(unused)]

use core::arch::asm;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    sys_exit(101)
}

#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

#[inline(always)]
unsafe fn syscall1(n: usize, a1: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            in("rax") n,
            in("rdi") a1,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack)
        );
    }
    ret
}

#[inline(always)]
unsafe fn syscall3(n: usize, a1: usize, a2: usize, a3: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            in("rax") n,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack)
        );
    }
    ret
}

fn sys_write(buf: &[u8]) {
    unsafe { syscall3(1, 1, buf.as_ptr() as usize, buf.len()); }
}

fn sys_read(buf: &mut [u8]) -> isize {
    unsafe { syscall3(0, 0, buf.as_mut_ptr() as usize, buf.len()) }
}

fn sys_exit(code: usize) -> ! {
    unsafe { syscall1(60, code); }
    loop {}
}

fn env_found(envp: *const *const u8, name: &[u8]) -> bool {
    let mut i = 0;
    loop {
        let entry = unsafe { *envp.add(i) };
        if entry.is_null() { return false; }
        let mut matched = true;
        for (k, &b) in name.iter().enumerate() {
            if unsafe { *entry.add(k) } != b { matched = false; break; }
        }
        if matched && unsafe { *entry.add(name.len()) } == b'=' { return true; }
        i += 1;
    }
}

fn print_hex(mut v: usize) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut buf = [0u8; 16];
    let mut i = 16;
    loop {
        if i == 0 { break; }
        i -= 1;
        buf[i] = HEX[v & 0xf];
        v >>= 4;
        if v == 0 { break; }
    }
    sys_write(b"0x");
    sys_write(&buf[i..16]);
    sys_write(b"\n");
}

#[cold]
#[inline(never)]
fn ok() -> ! {
    sys_write(b"ok\n");
    sys_exit(0);
}

#[cold]
#[inline(never)]
fn err() -> ! {
    sys_write(b"err\n");
    sys_exit(1);
}

fn main(sp: *const usize) -> ! {
    let base = _start as usize;
    print_hex((ok as usize).wrapping_sub(base));
    print_hex((err as usize).wrapping_sub(base));

    let argc = unsafe { *sp };
    let envp = unsafe { sp.add(argc as usize + 2) } as *const *const u8;

    if env_found(envp, b"aaaaaaaaaaaa") {
        let mut input = [0u8; [[len]]];
        sys_read(&mut input);
        [[storage]]
        let k: u64 = [[key]];
        [[dec]]
        if &input[..] == [[ident]].as_slice() {
            ok();
        } else {
            err();
        }
    }
    sys_exit(0)
}

#[no_mangle]
#[unsafe(naked)]
extern "C" fn _start() -> ! {
    core::arch::naked_asm!(
        "xor rbp, rbp",
        "mov rdi, rsp",
        "and rsp, -16",
        "call {f}",
        "mov rdi, rax",
        "call {exit}",
        f = sym main,
        exit = sym sys_exit,
    )
}
"#;

const ANGR_PY_TEMPLATE: &str = r#"
import angr
import sys
import time

t0 = time.perf_counter()
project = angr.Project(r"[[bin]]", auto_load_libs=False)

base = project.entry
ok_addr = base + [[ok]]
err_addr = base + [[err]]

initial_state = project.factory.entry_state(
    env={"aaaaaaaaaaaa": "1111"},
)

simgr = project.factory.simgr(initial_state)
simgr.explore(find=ok_addr, avoid=err_addr)
elapsed = time.perf_counter() - t0

if simgr.found:
    found_state = simgr.found[0]
    stdin_sol = found_state.posix.dumps(0)
    print(f"__ANGR_SEC:{elapsed:.6f}")
    print(f"__FOUND_HEX:{stdin_sol.hex()}")
    sys.exit(0)
else:
    print("NOT_FOUND")
    sys.exit(1)
"#;

fn run_py_with_timeout(python: &str, code: &str, timeout: Duration) -> std::io::Result<Output> {
    let mut child = Command::new(python)
        .args(["-c", code])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("aaaaaaaaaaaa", "1111")
        .spawn()?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("无法获取子进程 stdout"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| std::io::Error::other("无法获取子进程 stderr"))?;

    let stdout_handle = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let stderr_handle = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        buf
    });

    let start = Instant::now();
    let mut timed_out = false;

    let status = loop {
        match child.try_wait()? {
            Some(status) => break status,
            None => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let status = child.wait()?;
                    timed_out = true;
                    break status;
                }
                thread::sleep(Duration::from_millis(50));
            }
        }
    };

    let mut stdout_buf = stdout_handle.join().unwrap_or_default();
    let stderr_buf = stderr_handle.join().unwrap_or_default();

    if timed_out {
        if !stdout_buf.is_empty() && !stdout_buf.ends_with(b"\n") {
            stdout_buf.push(b'\n');
        }
        stdout_buf.extend_from_slice(b"TIMEOUT");
    }

    Ok(Output {
        status,
        stdout: stdout_buf,
        stderr: stderr_buf,
    })
}

fn parse_offsets(stdout: &str) -> (String, String) {
    let lines: Vec<&str> = stdout
        .lines()
        .map(|s| s.trim())
        .filter(|s| s.starts_with("0x"))
        .collect();

    assert!(
        lines.len() >= 2,
        "未能从输出中解析出 ok/err 的偏移地址:\n{}",
        stdout
    );
    (lines[0].to_string(), lines[1].to_string())
}

/// 返回 `(是否被破解, 有效抗符号执行秒数)`
/// 若超时或求解失败（NOT_FOUND），秒数直接记为 `timeout_secs as f64`
fn eval_with_angr(
    python: &str,
    bin: &Path,
    ok_offset: &str,
    err_offset: &str,
    expected_hex: &str,
    timeout_secs: u64,
) -> (bool, f64) {
    let py_code = ANGR_PY_TEMPLATE
        .replace("[[bin]]", &bin.to_string_lossy())
        .replace("[[ok]]", ok_offset)
        .replace("[[err]]", err_offset);

    let start = Instant::now();
    let out = match run_py_with_timeout(python, &py_code, Duration::from_secs(timeout_secs)) {
        Ok(out) => out,
        Err(e) => panic!("调用 python angr 进程失败: {e}"),
    };
    let wall_secs = start.elapsed().as_secs_f64().min(timeout_secs as f64);

    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut py_secs: Option<f64> = None;
    let mut found_hex: Option<String> = None;

    for line in stdout.lines() {
        let line = line.trim();
        if let Some(s) = line.strip_prefix("__ANGR_SEC:") {
            py_secs = s.parse::<f64>().ok();
        } else if let Some(h) = line.strip_prefix("__FOUND_HEX:") {
            found_hex = Some(h.to_string());
        }
    }

    if out.status.success()
        && let Some(actual_hex) = found_hex
        && actual_hex.starts_with(expected_hex)
    {
        let secs = py_secs.unwrap_or(wall_secs).min(timeout_secs as f64);
        (true, secs)
    } else {
        // 超时或未能正确求解出目标明文，视为防御成功，计满超时时间
        (false, timeout_secs as f64)
    }
}

#[derive(Clone, Copy)]
struct SecurityRecord {
    cracked: bool,
    angr_secs: f64,
}

struct SecurityAggStat {
    cracked_count: usize,
    total_runs: usize,
    med_angr_secs: f64,
    calibrated_score: f64,
}

#[test]
#[ignore = "需要python angr环境，执行CRYPTOS的security校准测试"]
#[allow(clippy::needless_range_loop)]
fn test_cryptos_security() {
    let python = if let Ok(py) = env::var("ANGR_PYTHONPATH") {
        let out = match run_py_with_timeout(
            &py,
            "import angr; print(ord('哈') + (1 + 3/5))",
            Duration::from_secs(15),
        ) {
            Ok(out) => out,
            Err(e) => panic!("无法启动 ANGR_PYTHONPATH 指定的解释器: {e}"),
        };
        if !out.status.success() {
            panic!(
                "angr 环境自检失败:\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
        } else if String::from_utf8_lossy(&out.stdout).trim() != "21705.6" {
            panic!(
                "angr 环境自检输出异常:\n{}",
                String::from_utf8_lossy(&out.stdout)
            );
        }
        py
    } else {
        panic!("需要设置含 angr 的 ANGR_PYTHONPATH 环境变量");
    };

    let sample_sizes: &[usize] = &[1, 3, 5, 16, 43, 128];
    assert!(
        sample_sizes.windows(2).all(|w| w[0] <= w[1]),
        "sample_sizes 必须按升序排列，以保证超时单调性剪枝的正确性"
    );

    const ANGR_TIMEOUT_SECS: u64 = 30;
    const ROUNDS: usize = 3;

    // 权重配置：angr时间 35%, cognitive_load 27%, attack_complexity 20%, latency 18%
    const WEIGHT_ANGR: f64 = 0.35;
    const WEIGHT_COG: f64 = 0.27;
    const WEIGHT_ATT: f64 = 0.20;
    const WEIGHT_LAT: f64 = 0.18;

    // 扣除 Python/angr 固定启动底噪 (0.15s)
    const ANGR_BASE_OVERHEAD: f64 = 0.15;
    // 最终整体缩放系数，预留编排层多原语叠加空间
    const COEFFICIENT: f64 = 0.60;

    let mut runner = DnyRun::new(
        &ANGR_RUST_TEMPLATE
            .replace("[[len]]", "1")
            .replace("[[storage]]", "let d = [0u8; 1];")
            .replace("[[key]]", "0")
            .replace("[[dec]]", "")
            .replace("[[ident]]", "d"),
        &obfstr2_dep(true),
    );
    runner.release(true);
    runner.cargo_config(
        r#"
[build]
target = "x86_64-unknown-none"

[target.x86_64-unknown-none]
rustflags = [
    "-C", "panic=abort",
    "-C", "opt-level=z",
    "-C", "relocation-model=static",
    "-C", "link-arg=--gc-sections",
]
"#,
    );

    let init_ret = runner.build(Some(Duration::from_secs(20)));
    assert!(
        init_ret.ok,
        "初始化 x86_64-unknown-none 构建测试环境失败:\n{init_ret}"
    );

    let num_cryptos = CRYPTOS.len();
    let num_samples = sample_sizes.len();

    let mut raw_data: Vec<Vec<Vec<SecurityRecord>>> =
        vec![vec![Vec::with_capacity(ROUNDS); num_samples]; num_cryptos];

    println!(
        "\n>>> 开始执行 {} 轮 Security 校准压测 (angr 超时: {}s, 样本长度: {:?})...",
        ROUNDS, ANGR_TIMEOUT_SECS, sample_sizes
    );

    for round in 1..=ROUNDS {
        println!(
            ">>> [Round {:02}/{:02}] 正在采集全量原语符号执行与静态指标数据...",
            round, ROUNDS
        );

        // 记录本轮在小样本下已达到 ANGR_TIMEOUT_SECS 的原语（记录触发时的样本长度）
        let mut timed_out_at_len: Vec<Option<usize>> = vec![None; num_cryptos];

        for (sample_idx, &sample_size) in sample_sizes.iter().enumerate() {
            let mut data = vec![0u8; sample_size];
            if sample_size > 0 {
                fill_bytes(data.as_mut());
            }
            let expected_hex: String = data.iter().map(|b| format!("{:02x}", b)).collect();

            let crypto_indices: Vec<usize> = shuffle((0..num_cryptos).collect());

            'c: for &c_idx in &crypto_indices {
                let crypto = &CRYPTOS[c_idx];
                if !(crypto.support)(sample_size) {
                    continue 'c;
                }

                // 智能跳过：若本轮该原语在更短样本下已超时，则更大样本必然超时
                if let Some(prev_len) = timed_out_at_len[c_idx] {
                    println!(
                        "    [Len {:>2}] {:<15} -> {:<8} ({:.2}s, 跳过: Len {} 已超时)",
                        sample_size, crypto.name, "DEFENDED", ANGR_TIMEOUT_SECS as f64, prev_len
                    );
                    raw_data[c_idx][sample_idx].push(SecurityRecord {
                        cracked: false,
                        angr_secs: ANGR_TIMEOUT_SECS as f64,
                    });
                    continue 'c;
                }

                let mut d = data.clone();
                let key = random::<u64>();
                let iv = random::<u8>();

                let mut ctx = LayerCtx {
                    type_path: quote!(::obfstr2::types::buffer::StackBytes),
                    key,
                    key_ident: format_ident!("k"),
                    iv,
                    in_size: d.len(),
                    out_size: d.len(),
                    ident_tag: "0".to_string(),
                };
                d = (crypto.enc)(&mut ctx, d);
                assert_eq!(d.len(), ctx.out_size);
                assert_eq!(data.len(), ctx.in_size);

                let storage = (STORAGES[0].ast)(
                    &format_ident!("d"),
                    &quote!(::obfstr2::types::buffer::StackBytes),
                    &d,
                )
                .to_string();
                let (dec, ident) = (crypto.dec)(&ctx, format_ident!("d"));

                let code = ANGR_RUST_TEMPLATE
                    .replace("[[len]]", &sample_size.to_string())
                    .replace("[[storage]]", &storage)
                    .replace("[[key]]", &key.to_string())
                    .replace("[[dec]]", &dec.to_string())
                    .replace("[[ident]]", &ident.to_string());

                runner.reset_main_code(&code);
                let ret = runner.run(Some(Duration::from_secs(18)));
                assert!(
                    ret.ok,
                    "原语裸机编译/偏移提取失败: {} (len={})\n{}",
                    crypto.name, sample_size, ret
                );

                let (ok_off, err_off) = parse_offsets(&ret.stdout);
                let bin_path = runner.bin_path();

                let (cracked, angr_secs) = eval_with_angr(
                    &python,
                    &bin_path,
                    &ok_off,
                    &err_off,
                    &expected_hex,
                    ANGR_TIMEOUT_SECS,
                );

                if !cracked && angr_secs >= ANGR_TIMEOUT_SECS as f64 {
                    timed_out_at_len[c_idx] = Some(sample_size);
                }

                println!(
                    "    [Len {:>2}] {:<15} -> {:<8} ({:.2}s)",
                    sample_size,
                    crypto.name,
                    if cracked { "CRACKED" } else { "DEFENDED" },
                    angr_secs
                );

                raw_data[c_idx][sample_idx].push(SecurityRecord { cracked, angr_secs });
            }
        }
    }

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

    // 扣除 0.15s 底噪后进行对数变换，提升低耗时区间的分辨率
    let angr_log = |secs: f64| -> f64 { ((secs - ANGR_BASE_OVERHEAD).max(0.0) + 1.0).ln() };

    // 阶段 1：组内聚合
    let mut agg_matrix: Vec<Vec<Option<SecurityAggStat>>> = Vec::with_capacity(num_cryptos);
    for c_idx in 0..num_cryptos {
        let mut sample_stats = Vec::with_capacity(num_samples);
        for s_idx in 0..num_samples {
            let runs = &raw_data[c_idx][s_idx];
            if runs.is_empty() {
                sample_stats.push(None);
            } else {
                let cracked_count = runs.iter().filter(|r| r.cracked).count();
                sample_stats.push(Some(SecurityAggStat {
                    cracked_count,
                    total_runs: runs.len(),
                    med_angr_secs: median(runs.iter().map(|r| r.angr_secs).collect()),
                    calibrated_score: 0.0,
                }));
            }
        }
        agg_matrix.push(sample_stats);
    }

    // 阶段 2：跨原语归一化与加权合成
    for s_idx in 0..num_samples {
        let valid_indices: Vec<usize> = (0..num_cryptos)
            .filter(|&c| agg_matrix[c][s_idx].is_some())
            .collect();

        if valid_indices.is_empty() {
            continue;
        }

        let min_angr_log = valid_indices
            .iter()
            .filter_map(|&c| agg_matrix[c][s_idx].as_ref())
            .map(|s| angr_log(s.med_angr_secs))
            .fold(f64::INFINITY, f64::min);
        let max_angr_log = valid_indices
            .iter()
            .filter_map(|&c| agg_matrix[c][s_idx].as_ref())
            .map(|s| angr_log(s.med_angr_secs))
            .fold(f64::NEG_INFINITY, f64::max);

        let min_cog = valid_indices
            .iter()
            .map(|&c| CRYPTOS[c].metrics.cognitive_load as f64)
            .fold(f64::INFINITY, f64::min);
        let max_cog = valid_indices
            .iter()
            .map(|&c| CRYPTOS[c].metrics.cognitive_load as f64)
            .fold(f64::NEG_INFINITY, f64::max);

        let min_att = valid_indices
            .iter()
            .map(|&c| CRYPTOS[c].metrics.attack_complexity as f64)
            .fold(f64::INFINITY, f64::min);
        let max_att = valid_indices
            .iter()
            .map(|&c| CRYPTOS[c].metrics.attack_complexity as f64)
            .fold(f64::NEG_INFINITY, f64::max);

        let min_lat = valid_indices
            .iter()
            .map(|&c| CRYPTOS[c].latency as f64)
            .fold(f64::INFINITY, f64::min);
        let max_lat = valid_indices
            .iter()
            .map(|&c| CRYPTOS[c].latency as f64)
            .fold(f64::NEG_INFINITY, f64::max);

        for c_idx in 0..num_cryptos {
            if let Some(stat) = &mut agg_matrix[c_idx][s_idx] {
                let crypto = &CRYPTOS[c_idx];
                let m = &crypto.metrics;

                let n_angr = if (max_angr_log - min_angr_log).abs() <= f64::EPSILON
                    && stat.med_angr_secs >= ANGR_TIMEOUT_SECS as f64
                {
                    1.0
                } else {
                    norm(angr_log(stat.med_angr_secs), min_angr_log, max_angr_log)
                };
                let n_cog = norm(m.cognitive_load as f64, min_cog, max_cog);
                let n_att = norm(m.attack_complexity as f64, min_att, max_att);
                let n_lat = norm(crypto.latency as f64, min_lat, max_lat);

                let composite_score = n_angr * WEIGHT_ANGR
                    + n_cog * WEIGHT_COG
                    + n_att * WEIGHT_ATT
                    + n_lat * WEIGHT_LAT;

                stat.calibrated_score = composite_score * 99.0 + 1.0;
            }
        }
    }

    // 输出按样本长度细分的校准报告
    for (sample_idx, &sample_size) in sample_sizes.iter().enumerate() {
        let header = format!(
            " Security 样本 #{idx} (长度: {len} 字节, angr超时: {timeout}s, 采样: {rounds} 轮) 校准报告 ",
            idx = sample_idx + 1,
            len = sample_size,
            timeout = ANGR_TIMEOUT_SECS,
            rounds = ROUNDS
        );
        println!("\n{:=^110}", header);
        println!(
            "{:<15} | {:<9} | {:<9} | {:<8} | {:<8} | {:<8} | {:<10} | {:<15}",
            "Crypto Name",
            "Cracked",
            "Angr(s)",
            "CogLoad",
            "AttComp",
            "Latency",
            "Old Sec",
            "Calibrated Sec"
        );
        println!("{:-^110}", "");

        for (c_idx, crypto) in CRYPTOS.iter().enumerate() {
            if let Some(stat) = &agg_matrix[c_idx][sample_idx] {
                let cracked_str = format!("{}/{}", stat.cracked_count, stat.total_runs);
                println!(
                    "{:<15} | {:<9} | {:<9.2} | {:<8} | {:<8} | {:<8} | {:<10} | \x1b[1;32m{:<5.1}\x1b[0m",
                    crypto.name,
                    cracked_str,
                    stat.med_angr_secs,
                    crypto.metrics.cognitive_load,
                    crypto.metrics.attack_complexity,
                    crypto.latency,
                    crypto.security,
                    stat.calibrated_score
                );
            } else {
                println!(
                    "{:<15} | {:<9} | {:<9} | {:<8} | {:<8} | {:<8} | {:<10} | \x1b[90m{:<15}\x1b[0m",
                    crypto.name,
                    "N/A",
                    "N/A",
                    crypto.metrics.cognitive_load,
                    crypto.metrics.attack_complexity,
                    crypto.latency,
                    crypto.security,
                    "Unsupported"
                );
            }
        }
        println!("{:=^110}\n", "");
    }

    // 输出最终全量横向推荐对比表（统一通过 COEFFICIENT 进行最终整体缩放）
    println!(
        "\n{:=^115}",
        format!(
            " 各长度样本校准 Security 横向对比与最终推荐表 (Angr {:.1}%, Cog {:.1}%, Att {:.1}%, Lat {:.1}% COEFFICIENT({COEFFICIENT:.4})) ",
            WEIGHT_ANGR * 100.0,
            WEIGHT_COG * 100.0,
            WEIGHT_ATT * 100.0,
            WEIGHT_LAT * 100.0
        ),
    );
    print!("{:<15} | {:<8}", "Crypto Name", "Old Sec");
    for sz in sample_sizes {
        print!(" | Len {:<5}", sz);
    }
    println!(" | \x1b[1;32m{:<9}\x1b[0m", "Final Rec");
    println!("{:-^115}", "");

    for (c_idx, crypto) in CRYPTOS.iter().enumerate() {
        print!("{:<15} | {:<8}", crypto.name, crypto.security);
        let mut sum_score = 0.0;
        let mut valid_samples = 0;

        for s_idx in 0..num_samples {
            if let Some(stat) = &agg_matrix[c_idx][s_idx] {
                let sc = stat.calibrated_score * COEFFICIENT;
                sum_score += sc;
                valid_samples += 1;
                print!(" | {:<9.1}", sc);
            } else {
                print!(" | \x1b[90m{:<9}\x1b[0m", "N/A");
            }
        }

        if valid_samples > 0 {
            let final_rec = (sum_score / valid_samples as f64).round().clamp(1.0, 100.0) as u8;
            println!(" | \x1b[1;32m{:<9}\x1b[0m", final_rec);
        } else {
            println!(" | \x1b[90m{:<9}\x1b[0m", "N/A");
        }
    }
    println!("{:=^115}\n", "");
}
