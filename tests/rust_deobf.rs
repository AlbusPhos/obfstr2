// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![cfg(unix)]
use lib_unknown::dyntest::DnyRun;
use std::env;
use std::error::Error;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// 栽种 5 类带明确 IOC 标签的测试载荷（对齐 rust-deobf 的 scatter 评测基准）：
/// 涵盖 [url]、[ip]、[mutex]、[path] 与 [registry]，
const SECRETS: &[&str] = &[
    "https://api.example.com/v1/beacon",
    "10.66.12.7:8443",
    "Global\\OrbitNotes-5c1e9a",
    "C:\\ProgramData\\OrbitSvc\\state.db",
    "Software\\OrbitNotes\\Agent\\Settings",
];

const TEMPLATE: &str = r#"
#![no_std]
#![no_main]
#![allow(unsafe_code)]
#![allow(unused)]
use core::arch::asm;
use core::hint::black_box;
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
fn sys_exit(code: usize) -> ! {
    unsafe { syscall1(60, code); }
    loop {}
}
/// 非内联接收函数：将 `&str` 的 `(ptr, len)` 暴露在标准 ABI 调用边界（rdi, rsi），
/// 为 rust-deobf 的 CALL/RET 钩子提供最有利的捕获条件。
#[inline(never)]
fn consume_str(s: &str) -> usize {
    let bytes = black_box(s.as_bytes());
    let mut acc = bytes.len();
    for &b in bytes {
        acc = acc.wrapping_mul(131).wrapping_add(b as usize);
    }
    black_box(acc)
}
#[inline(never)]
fn probe_endpoint() -> usize {
    consume_str([[obfstr]]!("https://api.example.com/v1/beacon"))
}
#[inline(never)]
fn probe_address() -> usize {
    consume_str([[obfstr]]!("10.66.12.7:8443"))
}
#[inline(never)]
fn probe_mutex() -> usize {
    consume_str([[obfstr]]!("Global\\OrbitNotes-5c1e9a"))
}
#[inline(never)]
fn probe_path() -> usize {
    consume_str([[obfstr]]!("C:\\ProgramData\\OrbitSvc\\state.db"))
}
#[inline(never)]
fn probe_registry() -> usize {
    consume_str([[obfstr]]!("Software\\OrbitNotes\\Agent\\Settings"))
}
fn main() -> ! {
    let mut tag = 0usize;
    tag ^= probe_endpoint();
    tag ^= probe_address();
    tag ^= probe_mutex();
    tag ^= probe_path();
    tag ^= probe_registry();
    black_box(tag);
    sys_exit(0)
}
#[no_mangle]
#[unsafe(naked)]
extern "C" fn _start() -> ! {
    core::arch::naked_asm!(
        "xor rbp, rbp",
        "and rsp, -16",
        "call {f}",
        "mov rdi, rax",
        "call {exit}",
        f = sym main,
        exit = sym sys_exit,
    )
}
"#;

fn run_with_timeout(program: &Path, args: &[&str], timeout: Duration) -> std::io::Result<Output> {
    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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

/// 调用 `rust-deobf recover <bin>` 并统计被成功还原的载荷列表。
fn test_with_rust_deobf(
    deobf_bin: &Path,
    target_bin: &Path,
    timeout_secs: u64,
) -> Result<(Vec<&'static str>, String), Box<dyn Error>> {
    let bin_str = target_bin.to_string_lossy();
    let out = run_with_timeout(
        deobf_bin,
        &["recover", &bin_str],
        Duration::from_secs(timeout_secs),
    )?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("rust-deobf 执行异常 (status={}):\n{}", out.status, stderr).into());
    }
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let recovered: Vec<&'static str> = SECRETS
        .iter()
        .copied()
        .filter(|&secret| stdout.contains(secret))
        .collect();
    Ok((recovered, stdout))
}

// running 1 test
// [1/2] 正在使用 rust-deobf 分析 obfstr::obfstr 基线产物...
// obfstr 分析输出:
// binary: dyn_test_1795511522210229705_834893268   format=ELF   arch=x86_64
//   static x86-64 (recover_callsite): 0 strings  decoders=0  seconds=0.1
//   emulation (recover_emulate): 5 strings  functions=14/14  timed_out=False  workers=1  seconds=0.2
//   compressed strings (recover_blobs): 0 strings  streams=0  seconds=0.0
//   static stack strings (recover_stackstrings): 0 strings  seconds=0.0
//   Rust literals (recover_rust_strings): 0 strings  unexplained=1  seconds=0.0
// 
// == decoded strings ==
//   10.66.12.7:8443   [ip]
//       function 0x201874  (emulation)
//   C:\ProgramData\OrbitSvc\state.dbC:\ProgramData\OrbitSvc\state.dbO   [path]
//       function 0x201874  (emulation)
//   Software\OrbitNotes\Agent\Settings   [registry]
//       function 0x201874  (emulation)
//   https://api.example.com/v1/beacon   [url]
//       function 0x201874  (emulation)
//   Global\OrbitNotes-5c1e9a   [mutex]
//       function 0x201637  (emulation)
// [2/2] 正在使用 rust-deobf 分析 &*obfstr2::s2 防御产物（预期 0 解出）...
// obfstr2 分析输出:
// binary: dyn_test_1795511522210229705_834893268   format=ELF   arch=x86_64
//   static x86-64 (recover_callsite): 0 strings  decoders=0  seconds=0.1
//   emulation (recover_emulate): 6 strings  functions=473/473  timed_out=False  workers=1  seconds=12.3
//   compressed strings (recover_blobs): 0 strings  streams=0  seconds=0.0
//   static stack strings (recover_stackstrings): 0 strings  seconds=0.3
//   Rust literals (recover_rust_strings): 30 strings  unexplained=70  seconds=0.1
// 
// == decoded strings ==
//   `abcdefghijkl
//       function 0x2059e6  (emulation)
//   BCDEFGHIJKLMNOPQRSTU
//       function 0x204fb2  (emulation)
//   defghijklmnopqrstuv
//       function 0x20764c  (emulation)
//   cdefghijklmno
//       function 0x207c8c  (emulation)
//   =>?@ABCDEFGHIJKLMNOPQR
//       function 0x211e9c  (emulation)
//   _`abcdefghijk
//       function 0x21081b  (emulation)
// 验证通过：obfstr 被还原 5/5，而 obfstr2 实现 0/5 解出，成功免疫 rust-deobf！
// test main ... ok
#[test]
#[ignore = "需要python rust-deobf环境"]
fn main() -> Result<(), Box<dyn Error>> {
    let deobf_bin = if let Ok(venv) = env::var("VIRTUAL_ENV") {
        let bin = Path::new(&venv).join("bin").join("rust-deobf");
        let out = run_with_timeout(&bin, &["--version"], Duration::from_secs(10))?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).into());
        }
        bin
    } else {
        return Err("需要设置含rust-deobf的VIRTUAL_ENV环境变量".into());
    };
    // 启用 anti-disassembly（支持 no_std），验证完整抗逆向特征
    let deps = format!(
        "obfstr2 = {{ path = {:?}, default-features = false, features = [\"anti-disassembly\"] }}\n\
         obfstr = \"0.4\"\n",
        env!("CARGO_MANIFEST_DIR")
    );
    let obf_macro = "obfstr::obfstr";

    let mut runner = DnyRun::new(&TEMPLATE.replace("[[obfstr]]", obf_macro), &deps);
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
    // ── 阶段 1：基线对照（验证 rust-deobf 能够 5/5 全破传统 obfstr）──────────
    let ret_base = runner.run(Some(Duration::from_secs(18)));
    assert!(ret_base.ok, "基线构建/运行失败:\n{ret_base}");
    assert!(
        ret_base.stderr.is_empty(),
        "基线构建 stderr 非空:\n{ret_base}"
    );
    let bin_path: PathBuf = runner.bin_path();
    println!("[1/2] 正在使用 rust-deobf 分析 {obf_macro} 基线产物...");
    let (base_recovered, base_out) = test_with_rust_deobf(&deobf_bin, &bin_path, 60)?;
    println!("obfstr 分析输出:\n{}", base_out.trim());
    assert_eq!(
        base_recovered.len(),
        SECRETS.len(),
        "基准测试失败：rust-deobf 应当能全量还原 obfstr 的 {} 个字符串，实际仅还原 {:?}！",
        SECRETS.len(),
        base_recovered
    );
    // ── 阶段 2：防御验证（验证 obfstr2 在均衡档 s2! 下实现 0 解出）──────────
    let obf_macro = "&*obfstr2::s2";
    runner.reset_main_code(&TEMPLATE.replace("[[obfstr]]", obf_macro));
    let ret_obf = runner.run(Some(Duration::from_secs(18)));
    assert!(ret_obf.ok, "防御构建/运行失败:\n{ret_obf}");
    assert!(
        ret_obf.stderr.is_empty(),
        "防御构建 stderr 非空:\n{ret_obf}"
    );
    println!("[2/2] 正在使用 rust-deobf 分析 {obf_macro} 防御产物（预期 0 解出）...");
    let (obf_recovered, obf_out) = test_with_rust_deobf(&deobf_bin, &bin_path, 90)?;
    println!("obfstr2 分析输出:\n{}", obf_out.trim());
    assert!(
        obf_recovered.is_empty(),
        "防御测试失败：rust-deobf 成功还原了 obfstr2 的敏感载荷: {:?}！",
        obf_recovered
    );
    println!(
        "验证通过：obfstr 被还原 {}/{}，而 obfstr2 实现 0/{} 解出，成功免疫 rust-deobf！",
        base_recovered.len(),
        SECRETS.len(),
        SECRETS.len()
    );
    Ok(())
}
