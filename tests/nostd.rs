// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 裸机链路测试：`x86_64-unknown-none` 下解密并 `write(1)` 输出。
//!
//! 按 `default-features` 拆分为两个 `#[test]` 批次入口，
//! 各自通过 `run_batch_use_cache` 预热并复用编译缓存。
//!
//! 运行含 `*_key!` 的批次前需设置 `OBFSTR2_TEST_KEY`；裸机目标无运行时环境变量
//! 接口，`run_key` 由测试宿主读取后以函数返回值形式注入生成源码。
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

use lib_unknown::compiletest::{Task, run_batch_use_cache};
use std::time::Duration;

const NOSTD_CARGO_CONFIG: &str = r#"
[build]
target = "x86_64-unknown-none"

[target.x86_64-unknown-none]
rustflags = [
    "-C", "panic=abort",
    "-C", "opt-level=z",
    "-C", "relocation-model=static",
    "-C", "link-arg=--gc-sections",
]
"#;

const NOSTD_TEMPLATE: &str = r#"
#![no_std]
#![no_main]
#![allow(binary_asm_labels)]
#![allow(unused)]
#![allow(unsafe_code)]
#![cfg(not(test))]

use core::arch::asm;
use core::panic;

extern crate alloc;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

// 极简 bump 分配器：部分宏可能随机展开为堆容器（HeapBytes / HeapU16），裸机需提供全局分配器。
// 128 码元载荷下 64KiB 绰绰有余；不回收（测试进程一次运行）。
struct Bump;
struct BumpBuf(UnsafeCell<[u8; 65536]>);
unsafe impl Sync for BumpBuf {}
static BUMP_BUF: BumpBuf = BumpBuf(UnsafeCell::new([0; 65536]));
static BUMP_POS: AtomicUsize = AtomicUsize::new(0);
unsafe impl core::alloc::GlobalAlloc for Bump {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();
        let mut pos = BUMP_POS.load(Ordering::Relaxed);
        loop {
            let aligned = (pos + align - 1) & !(align - 1);
            let next = aligned + size;
            if next > 65536 {
                return core::ptr::null_mut();
            }
            match BUMP_POS.compare_exchange(pos, next, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => return unsafe { (*BUMP_BUF.0.get()).as_mut_ptr().add(aligned) },
                Err(cur) => pos = cur,
            }
        }
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}
#[global_allocator]
static GLOBAL: Bump = Bump;

#[panic_handler]
fn panic(_: &panic::PanicInfo) -> ! {
    sys_exit(101)
}

pub mod constants {
    pub const WRITE: usize = 1;
    pub const EXIT: usize  = 60;
}

#[inline(always)]
pub unsafe fn syscall1(n: usize, a1: usize) -> isize {
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
pub unsafe fn syscall3(n: usize, a1: usize, a2: usize, a3: usize) -> isize {
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

#[inline(always)]
pub fn sys_exit(status: usize) -> ! {
    unsafe { syscall1(constants::EXIT, status); }
    loop {}
}

#[inline(always)]
pub fn sys_write(fd: usize, buf: &[u8]) -> isize {
    unsafe { syscall3(constants::WRITE, fd, buf.as_ptr() as usize, buf.len()) }
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub extern "C" fn _start() -> ! {
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

#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}

fn main() -> usize {
    [[main_body]]
    0
}
"#;

fn obfstr2_dep(no_default: bool) -> String {
    if no_default {
        format!(
            "obfstr2 = {{ path = {:?}, default-features = false }}",
            env!("CARGO_MANIFEST_DIR")
        )
    } else {
        format!("obfstr2 = {{ path = {:?} }}", env!("CARGO_MANIFEST_DIR"))
    }
}

/// 对应原 `run_nostd`：`&*bytes` 解引用输出
fn body_deref_bytes(expr: &str) -> String {
    format!("let bytes = {expr};\n    sys_write(1, &*bytes);")
}

/// 对应原 `run_nostd_bytes`：`Vec<u8>` 直接输出
fn body_vec_u8(expr: &str) -> String {
    format!("let v: alloc::vec::Vec<u8> = {expr};\n    sys_write(1, &v);")
}

/// 对应原 `run_nostd_int`：`u32` 小端 4 字节输出
fn body_u32_le(expr: &str) -> String {
    format!("let v: u32 = {expr};\n    sys_write(1, &v.to_le_bytes());")
}

/// 定宽数组容器：直接输出底层小端字节流（无 alloc）。
fn body_word_bytes(expr: &str) -> String {
    format!("let words = {expr};\n    sys_write(1, words.as_byte_slice());")
}

/// 对应原 `run_nostd_float`：`f32` 小端位字节转 8 字节 hex ASCII 输出（避免非 UTF-8 被 lossy 改写）
fn body_f32_hex(expr: &str) -> String {
    format!(
        r#"let v: f32 = {expr};
    let b = v.to_bits().to_le_bytes();
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = [0u8; 8];
    let mut k = 0;
    while k < 4 {{
        out[2 * k] = HEX[(b[k] >> 4) as usize];
        out[2 * k + 1] = HEX[(b[k] & 15) as usize];
        k += 1;
    }}
    sys_write(1, &out);"#
    )
}

/// 批量构建并运行同一 `deps` 配置下的所有裸机任务，复用首任务缓存。
fn run_nostd_batch(deps: &str, cases: &[(&str, String, Vec<u8>)]) {
    let tasks = cases.iter().map(|(tag, main_body, _)| {
        let code = NOSTD_TEMPLATE.replace("[[main_body]]", main_body);
        Task::new(*tag, code)
            .with_deps(deps)
            .with_cargo_config(NOSTD_CARGO_CONFIG)
    });

    let results = run_batch_use_cache(tasks, Some(Duration::from_secs(180)), None, None);
    assert_eq!(results[0].0, "init_cache");

    for ((expected_tag, _, expected_bytes), (actual_tag, ret)) in
        cases.iter().zip(results.iter().skip(1))
    {
        assert_eq!(expected_tag, actual_tag);
        assert!(ret.ok, "[{actual_tag}] 裸机构建或运行失败:\n{ret}");
        assert!(ret.stderr.is_empty(), "[{actual_tag}] 编译失败:\n{ret}");
        assert_eq!(
            ret.stdout.as_bytes(),
            expected_bytes.as_slice(),
            "[{actual_tag}] 解码数据错误"
        );
    }
}

#[test]
fn nostd_no_alloc_batch() {
    let lit = "a".repeat(128);
    let expected_a128 = vec![b'a'; 128];

    let mut expected_w128 = Vec::with_capacity(256);
    for u in lit.encode_utf16() {
        expected_w128.extend_from_slice(&u.to_le_bytes());
    }
    let mut expected_cw128 = expected_w128.clone();
    expected_cw128.extend_from_slice(&[0, 0]);

    let expected_fl1_hex: Vec<u8> = 1.5f32
        .to_bits()
        .to_le_bytes()
        .iter()
        .flat_map(|b| format!("{b:02x}").into_bytes())
        .collect();
    let expected_dwords: Vec<u8> = [0x61616161u32, 0x62626262, 0]
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();
    let expected_qwords: Vec<u8> = [0x0102_0304_0506_0708u64, 0]
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();

    let cases = vec![
        // 1. b1: 128 × 'a'，纯栈、无堆
        (
            "b1_nostd",
            body_deref_bytes(&format!(r#"obfstr2::b1!(b"{lit}")"#)),
            expected_a128.clone(),
        ),
        // 2. cw1: 128 × 'a' 宽 C 字符串（UTF-16LE + 结尾 0u16）
        (
            "cw1_nostd",
            body_vec_u8(&format!(
                r#"{{ let w = obfstr2::cw1!("{lit}"); w.as_byte_slice().to_vec() }}"#
            )),
            expected_cw128,
        ),
        // 3. w1: 128 × 'a' 宽字符串（UTF-16LE，无结尾 NUL）
        (
            "w1_nostd",
            body_vec_u8(&format!(
                r#"{{ let w = obfstr2::w1!("{lit}"); w.as_byte_slice().to_vec() }}"#
            )),
            expected_w128,
        ),
        // 4. cs1: 128 × 'a' C 字符串
        (
            "cs1_nostd",
            body_deref_bytes(&format!(
                r#"{{ let s = obfstr2::cs1!(c"{lit}"); obfstr2::types::cstr::CStr::as_bytes(&s).to_vec() }}"#
            )),
            expected_a128.clone(),
        ),
        // 5. i1: 0x61616161u32 小端即 4 × 'a'
        (
            "i1_nostd",
            body_u32_le("obfstr2::i1!(0x61616161u32)"),
            b"aaaa".to_vec(),
        ),
        // 6. b_user: 自定义档字节串
        (
            "b_user_nostd",
            body_deref_bytes(&format!(
                r#"obfstr2::b_user!({{ prefer_stack: true, max_latency: 40, min_security: 60 }}, b"{lit}")"#
            )),
            expected_a128,
        ),
        // 7. i_user: 自定义档整数链路
        (
            "i_user_nostd",
            body_u32_le("obfstr2::i_user!({ prefer_stack: true, max_latency: 40 }, 0x61616161u32)"),
            b"aaaa".to_vec(),
        ),
        // 8. fl1: 1.5f32 小端位字节 hex（"0000c03f"）
        (
            "fl1_nostd",
            body_f32_hex("obfstr2::fl1!(1.5f32)"),
            expected_fl1_hex,
        ),
        // 9. d1: u32 数组小端字节流
        (
            "d1_nostd",
            body_word_bytes("obfstr2::d1!([0x61616161u32, 0x62626262, 0])"),
            expected_dwords,
        ),
        // 10. q1: u64 数组小端字节流
        (
            "q1_nostd",
            body_word_bytes("obfstr2::q1!([0x0102_0304_0506_0708u64, 0])"),
            expected_qwords,
        ),
    ];

    run_nostd_batch(&obfstr2_dep(true), &cases);
}

#[test]
fn nostd_default_features_batch() {
    let lit = "a".repeat(128);
    let expected_a128 = vec![b'a'; 128];

    // 裸机目标没有可读取环境变量的运行时接口：
    // - `build_key` 仍由临时工程编译期的 `env!` 从 OBFSTR2_TEST_KEY 读取；
    // - `run_key` 由测试宿主在运行期读取同名环境变量后，以函数返回值形式注入生成源码，
    //   用于验证“运行期密钥来源”链路，而不是把密钥直接写进宏调用参数。
    // 运行本测试前必须设置 OBFSTR2_TEST_KEY，且两次读写必须一致。
    let env_key = std::env::var("OBFSTR2_TEST_KEY")
        .expect("运行 *_key! 测试前请设置环境变量 `OBFSTR2_TEST_KEY`");
    let env_key_bytes = format!("{:?}", env_key.as_bytes());

    let mut expected_w128 = Vec::with_capacity(256);
    for u in lit.encode_utf16() {
        expected_w128.extend_from_slice(&u.to_le_bytes());
    }
    let mut expected_cw128 = expected_w128.clone();
    expected_cw128.extend_from_slice(&[0, 0]);

    let expected_dwords: Vec<u8> = [0x61616161u32, 0x62626262, 0]
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();
    let expected_qwords: Vec<u8> = [0x0102_0304_0506_0708u64, 0]
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();

    let cases = vec![
        // 1. b2: 默认特性下可能随机展开为 HeapBytes
        (
            "b2_nostd",
            body_deref_bytes(&format!(r#"obfstr2::b2!(b"{lit}")"#)),
            expected_a128.clone(),
        ),
        // 2. cw2: 默认特性下外层可能随机选择 HeapU16
        (
            "cw2_nostd",
            body_vec_u8(&format!(
                r#"{{ let w = obfstr2::cw2!("{lit}"); w.as_byte_slice().to_vec() }}"#
            )),
            expected_cw128.clone(),
        ),
        // 3. cw3: 高强度档在 alloc 下主容器恒为 HeapU16
        (
            "cw3_nostd",
            body_vec_u8(&format!(
                r#"{{ let w = obfstr2::cw3!("{lit}"); w.as_byte_slice().to_vec() }}"#
            )),
            expected_cw128,
        ),
        // 4. w3: 高强度档无 NUL 宽字符串裸机堆链路
        (
            "w3_nostd",
            body_vec_u8(&format!(
                r#"{{ let w = obfstr2::w3!("{lit}"); w.as_byte_slice().to_vec() }}"#
            )),
            expected_w128,
        ),
        // 5. s_key: 动态密钥（编译期 env! build_key + 运行期来源 run_key），默认特性（含堆）
        (
            "s_key_nostd",
            body_deref_bytes(&format!(
                r#"{{ fn runtime_key() -> &'static [u8] {{ &{env_key_bytes} }}
                let s = obfstr2::s_key!({{ build_key: env!("OBFSTR2_TEST_KEY"), run_key: runtime_key() }}, "{lit}");
                obfstr2::types::str::Str::as_str(&s).as_bytes().to_vec() }}"#
            )),
            expected_a128,
        ),
        // 6. d2: 默认特性下可能随机展开为 HeapU32
        (
            "d2_nostd",
            body_word_bytes("obfstr2::d2!([0x61616161u32, 0x62626262, 0])"),
            expected_dwords,
        ),
        // 7. q2: 默认特性下可能随机展开为 HeapU64
        (
            "q2_nostd",
            body_word_bytes("obfstr2::q2!([0x0102_0304_0506_0708u64, 0])"),
            expected_qwords,
        ),
    ];

    run_nostd_batch(&obfstr2_dep(false), &cases);
}
