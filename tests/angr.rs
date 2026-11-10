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

const TEMPLATE: &str = r#"
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

    // 栈布局：argc | argv[0..argc] | NULL | envp[0..] | NULL
    let argc = unsafe { *sp };
    let envp = unsafe { sp.add(argc as usize + 2) } as *const *const u8;

    if env_found(envp, b"aaaaaaaaaaaa") {
        let mut input = [0u8; 8];
        sys_read(&mut input);
        if &input[..] == [[obfstr]]!("1234567\n").as_bytes() {
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
        "mov rdi, rsp",   // 保留原始 sp（argc 起点），再对齐 rsp
        "and rsp, -16",
        "call {f}",
        "mov rdi, rax",
        "call {exit}",
        f = sym main,
        exit = sym sys_exit,
    )
}
"#;

const TEMPLATE2: &str = r#"
import angr
import sys

project = angr.Project(r"[[bin]]", auto_load_libs=False)

base = project.entry
ok_addr = base + [[ok]]
err_addr = base + [[err]]


initial_state = project.factory.entry_state(
    env={"aaaaaaaaaaaa": "1111"},
)

simgr = project.factory.simgr(initial_state)
simgr.explore(find=ok_addr, avoid=err_addr)

if simgr.found:
    found_state = simgr.found[0]
    stdin_sol = found_state.posix.dumps(0)
    print(f"FOUND: {stdin_sol}")
    sys.exit(0)
else:
    print("NOT_FOUND")
    sys.exit(1)
"#;

fn run_with_timeout(python: &str, code: &str, timeout: Duration) -> std::io::Result<Output> {
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

fn run(python: &str, code: &str) -> std::io::Result<Output> {
    run_with_timeout(python, code, Duration::from_secs(10))
}

fn parse_offsets(stdout: &str) -> (String, String) {
    let lines: Vec<&str> = stdout
        .lines()
        .map(|s| s.trim())
        .filter(|s| s.starts_with("0x"))
        .collect();

    assert!(
        lines.len() >= 2,
        "未能从输出中解析出 ok/err 的偏移地址: \n{}",
        stdout
    );
    (lines[0].to_string(), lines[1].to_string())
}

fn test_with_angr(
    python: &str,
    bin: &Path,
    ok_offset: &str,
    err_offset: &str,
    timeout_secs: usize,
) -> Result<(bool, String), Box<dyn Error>> {
    let py_code = TEMPLATE2
        .replace("[[bin]]", &bin.to_string_lossy())
        .replace("[[ok]]", ok_offset)
        .replace("[[err]]", err_offset);

    let out = run_with_timeout(python, &py_code, Duration::from_secs(timeout_secs as u64))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let is_cracked = out.status.success() && stdout.contains("FOUND");
    Ok((is_cracked, stdout))
}

// running 1 test
// [1/2] 正在对 obfstr 进行符号执行求解...
// obfstr 求解结果: FOUND: b'1234567\n'
// [2/2] 正在对 obfstr2 进行符号执行求解（预期应超时/失败）...
// test main has been running for over 60 seconds
// obfstr2 求解结果: TIMEOUT
// 验证通过：obfstr2 成功防御了 angr 的符号执行分析！
// test main ... ok
//
// test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 67.86s
#[test]
#[ignore = "需要python angr环境"]
fn main() -> Result<(), Box<dyn Error>> {
    let python = if let Ok(py) = env::var("ANGR_PYTHONPATH") {
        let out = run(&py, "import angr; print(ord('哈') + (1 + 3/5))")?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).into());
        } else if String::from_utf8_lossy(&out.stdout).trim() != "21705.6" {
            return Err(String::from_utf8_lossy(&out.stdout).into());
        }
        py
    } else {
        return Err("需要设置含angr的ANGR_PYTHONPATH环境变量".into());
    };

    let deps = format!(
        "obfstr2 = {{ path = {:?}, default-features = false }}
obfstr = \"0.4\"
",
        env!("CARGO_MANIFEST_DIR")
    );

    let mut runner = DnyRun::new(&TEMPLATE.replace("[[obfstr]]", "obfstr::obfstr"), &deps);
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

    let ret_base = runner.run(Some(Duration::from_secs(18)));
    assert!(ret_base.ok, "基线构建/运行失败:\n{ret_base}");
    assert!(
        ret_base.stderr.is_empty(),
        "基线构建 stderr 非空:\n{ret_base}"
    );

    let bin_path: PathBuf = runner.bin_path();
    let (ok_base, err_base) = parse_offsets(&ret_base.stdout);

    println!("[1/2] 正在对 obfstr 进行符号执行求解...");
    let (base_cracked, base_out) = test_with_angr(&python, &bin_path, &ok_base, &err_base, 15)?;
    println!("obfstr 求解结果: {}", base_out.trim());

    assert!(
        base_cracked,
        "基准测试失败：angr 应该能够轻松逆向常规的 obfstr 字符串混淆！"
    );

    runner.reset_main_code(&TEMPLATE.replace("[[obfstr]]", "&*obfstr2::s3"));
    let ret_obf = runner.run(Some(Duration::from_secs(18)));
    assert!(ret_obf.ok, "防御构建/运行失败:\n{ret_obf}");
    assert!(
        ret_obf.stderr.is_empty(),
        "防御构建 stderr 非空:\n{ret_obf}"
    );

    let (ok_obf, err_obf) = parse_offsets(&ret_obf.stdout);

    println!("[2/2] 正在对 obfstr2 进行符号执行求解（预期应超时/失败）...");
    let (obf_cracked, obf_out) = test_with_angr(&python, &bin_path, &ok_obf, &err_obf, 60)?;
    println!("obfstr2 求解结果: {}", obf_out.trim());

    assert!(
        !obf_cracked,
        "防御测试失败：angr 成功破解了 obfstr2！说明未起到抗符号执行作用。"
    );

    println!("验证通过：obfstr2 成功防御了 angr 的符号执行分析！");
    Ok(())
}
