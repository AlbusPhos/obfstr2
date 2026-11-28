// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 性能对比测试：全尺度 Payload 覆盖 (8B ~ 10MB) + 单次缓存构建耗时 + 每次去混淆耗时 + 产物体积 + expand 字符数。

use lib_unknown::compiletest::Runner;
use std::time::Duration;

/// 压测覆盖的数据尺寸阶梯：8B, 32B, 256B, 1KB, 256KB, 1MB, 10MB
const SIZES: &[usize] = &[8, 32, 256, 1024, 1024 * 256, 1024 * 1024, 1024 * 1024 * 10];
// const SIZES: &[usize] = &[8, 32, 256, 1024, 1024 * 256];

const TIMEOUT_SECS: u64 = 300;

/// 每个尺寸下的用例数（明文基线 + CasualX + obfstr2 b1/b2/b3）。
const PER_CHUNK: usize = 5;

fn timeout() -> Option<Duration> {
    Some(Duration::from_secs(TIMEOUT_SECS))
}

/// 根据 Payload 长度自适应调节解密循环次数，避免超大块压测超时
fn get_loops(len: usize) -> u32 {
    match len {
        0..=1024 => 512,
        1025..=262_144 => 64,      // 256 KB
        262_145..=1_048_576 => 16, // 1 MB
        _ => 2,                    // 10 MB: 2 轮解密 (累计处理 20MB 数据)
    }
}

fn payload(len: usize) -> String {
    "a".repeat(len)
}

fn obfstr2_dep() -> String {
    format!("obfstr2 = {{ path = {:?} }}", env!("CARGO_MANIFEST_DIR"))
}

fn casual_dep() -> String {
    "obfstr = \"0.4\"".to_string()
}

fn guest(body: &str, loops: u32) -> String {
    format!(
        r#"
#[allow(long_running_const_eval)]
fn main() {{
    let handle = std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {{
            let mut sum: u64 = 0;
            for _ in 0..std::hint::black_box({loops}) {{
                {body}
            }}
            println!("{{sum}}");
        }})
        .unwrap();
    handle.join().unwrap();
}}"#
    )
}

fn case_defs(p: &str, len: usize, loops: u32) -> [(&'static str, String, String); 5] {
    let base_code = format!(
        "static SECRET: &[u8; {len}] = b\"{p}\";\n{}",
        guest(
            "for &b in SECRET.iter() { sum = sum.wrapping_add(b as u64); }\n        std::hint::black_box(sum);",
            loops
        )
    );

    let casual_bytes = guest(
        &format!(
            "for &b in std::hint::black_box(obfstr::obfbytes!(b\"{p}\")).iter() {{ sum = sum.wrapping_add(b as u64); }}\n        std::hint::black_box(sum);"
        ),
        loops,
    );

    let obf = |m: &str| {
        guest(
            &format!(
                "let v = std::hint::black_box(obfstr2::{m}!(b\"{p}\"));\n        for &b in (&*v).iter() {{ sum = sum.wrapping_add(b as u64); }}"
            ),
            loops,
        )
    };

    [
        ("明文基线", base_code, String::new()),
        ("CasualX obfbytes!", casual_bytes, casual_dep()),
        ("obfstr2 b1!", obf("b1"), obfstr2_dep()),
        ("obfstr2 b2!", obf("b2"), obfstr2_dep()),
        ("obfstr2 b3!", obf("b3"), obfstr2_dep()),
    ]
}

struct Sample {
    cached_build: Duration,
    run: Duration,
    expand_chars: Option<usize>,
    bin_bytes: Option<u64>,
}

fn measure_single(runner: &mut Runner, name: &str, expected_sum: u64) -> Sample {
    let ret = runner.run(timeout());
    assert!(ret.ok, "[{name}] 缓存构建或运行失败:\n{ret}");

    let got: u64 = ret
        .stdout
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("[{name}] 校验和非数字，输出:\n{}", ret.stdout));
    assert_eq!(got, expected_sum, "[{name}] 解密校验和错误");

    let bin_bytes = std::fs::metadata(runner.bin_path()).map(|m| m.len()).ok();
    let expand_res = runner.cargo(&["expand"], timeout());
    let expand_chars = expand_res.ok.then_some(expand_res.stdout.len());

    Sample {
        cached_build: ret.build_duration,
        run: ret.run_duration,
        expand_chars,
        bin_bytes,
    }
}

struct Row {
    size: usize,
    loops: u32,
    name: String,
    cached_build: Duration,
    run: Duration,
    expand_chars: Option<usize>,
    bin_bytes: Option<u64>,
}

fn per_decrypt(row_run: Duration, base_run: Duration, loops: u32) -> f64 {
    (row_run.as_secs_f64() - base_run.as_secs_f64()).max(0.0) / loops as f64 * 1e6
}

fn pct(v: f64, base: f64) -> Option<String> {
    if base == 0.0 {
        return None;
    }
    Some(format!("{:.0}%", v / base * 100.0 - 100.0))
}

fn cell(abs: String, ratio: Option<String>) -> String {
    match ratio {
        Some(p) => format!("{abs}（{p}）"),
        None => abs,
    }
}

fn format_bytes(bytes: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    if bytes >= MB {
        format!("{:.2} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{:.0} B", bytes)
    }
}

fn format_duration_us(us: f64) -> String {
    if us >= 1e6 {
        format!("{:.2} s", us / 1e6)
    } else if us >= 1e3 {
        format!("{:.2} ms", us / 1e3)
    } else {
        format!("{:.2} µs", us)
    }
}

fn fmt_dur(d: Duration) -> String {
    format_duration_us(d.as_secs_f64() * 1e6)
}

fn render_report(rows: &[Row]) {
    eprintln!(
        "| Payload | 用例 | 每次去混淆耗时 | 缓存构建耗时 | 产物二进制（未 strip） | expand 字符数 |"
    );
    eprintln!("| ---: | :--- | ---: | ---: | ---: | ---: |");

    for chunk in rows.chunks(PER_CHUNK) {
        let base = &chunk[0];
        for (j, r) in chunk.iter().enumerate() {
            let is_base = j == 0;

            let size_cell = if j == 0 {
                format_bytes(r.size as f64)
            } else {
                "〃".to_string()
            };

            let per = if is_base {
                "—".to_string()
            } else {
                format_duration_us(per_decrypt(r.run, base.run, r.loops))
            };

            let p = |v: f64, b: f64| if is_base { None } else { pct(v, b) };
            let po = |v: Option<u64>, b: Option<u64>| {
                if is_base {
                    return None;
                }
                match (v, b) {
                    (Some(v), Some(b)) => pct(v as f64, b as f64),
                    _ => None,
                }
            };

            let build = cell(
                fmt_dur(r.cached_build),
                p(
                    r.cached_build.as_secs_f64(),
                    base.cached_build.as_secs_f64(),
                ),
            );
            let bin = cell(
                r.bin_bytes
                    .map_or("n/a".to_string(), |n| format_bytes(n as f64)),
                po(r.bin_bytes, base.bin_bytes),
            );
            let expand = cell(
                r.expand_chars.map_or("n/a".to_string(), |n| n.to_string()),
                po(
                    r.expand_chars.map(|v| v as u64),
                    base.expand_chars.map(|v| v as u64),
                ),
            );

            eprintln!(
                "| {:<9} | {:<18} | {:>13} | {} | {} | {} |",
                size_cell, r.name, per, build, bin, expand
            );
        }
    }
    eprintln!("{:=^118}\n", "");
}

#[test]
#[ignore = "全尺度覆盖压测，耗时约 2~3 分钟"]
fn perf_report_release() {
    let combined_deps = format!("{}\n{}", casual_dep(), obfstr2_dep());

    eprintln!("\n>>> 正在进行全局依赖预热构建 (Warm-up)...");
    let mut runner = Runner::new("fn main() {}", &combined_deps);
    runner.release(true);
    let warm = runner.build(timeout());
    assert!(warm.ok, "依赖预热构建失败:\n{warm}");
    eprintln!(">>> 预热完成");

    let mut all_rows: Vec<Row> = Vec::with_capacity(SIZES.len() * PER_CHUNK);

    for &size in SIZES {
        let loops = get_loops(size);
        let p = payload(size);
        let cases = case_defs(&p, size, loops);
        let expected_sum = size as u64 * b'a' as u64 * loops as u64;

        for (name, code, _) in &cases {
            runner.reset_main_code(code);
            let s = measure_single(&mut runner, name, expected_sum);
            all_rows.push(Row {
                size,
                loops,
                name: name.to_string(),
                cached_build: s.cached_build,
                run: s.run,
                expand_chars: s.expand_chars,
                bin_bytes: s.bin_bytes,
            });
        }
    }

    render_report(&all_rows);
}
