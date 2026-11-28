// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 非法输入测试：越界元素 / 缺失文件 / 非字面量 / 未闭合括号均报编译错误。
//!
//! 使用 `run_batch_use_cache` 预热一次 `obfstr2` 依赖并复用编译缓存。

use lib_unknown::compiletest::{Task, run_batch_use_cache};

fn obfstr2_dep() -> String {
    format!("obfstr2 = {{ path = {:?} }}", env!("CARGO_MANIFEST_DIR"))
}

/// (tag, 期望包含的错误关键字 needle, 待测代码 code)
const COMPILE_FAIL_CASES: &[(&str, &str, &str)] = &[
    (
        "b2-range",
        "0..=255",
        r#"fn main() { let _b = obfstr2::b2!([0x61, 300]); }"#,
    ),
    (
        "d2-non-array",
        "只接受 u32 整数字面量数组",
        r#"fn main() { let x = [1u32, 2]; let _d = obfstr2::d2!(x); }"#,
    ),
    (
        "d2-neg",
        "数组元素只接受整数字面量",
        r#"fn main() { let _d = obfstr2::d2!([-1]); }"#,
    ),
    (
        "d2-range",
        "超出 u32 范围",
        r#"fn main() { let _d = obfstr2::d2!([4294967296u64]); }"#,
    ),
    (
        "d2-float",
        "数组元素只接受整数字面量",
        r#"fn main() { let _d = obfstr2::d2!([1.0f32]); }"#,
    ),
    (
        "q2-neg",
        "数组元素只接受整数字面量",
        r#"fn main() { let _q = obfstr2::q2!([-1]); }"#,
    ),
    (
        "q2-range",
        "超出 u64 范围",
        r#"fn main() { let _q = obfstr2::q2!([18446744073709551616u128]); }"#,
    ),
    (
        "s-user-unknown",
        "未知的 TierParams 字段",
        r#"fn main() { let _s = obfstr2::s_user!({ unknown_field: 1 }, "hi"); }"#,
    ),
    (
        "s-user-dup",
        "重复配置字段",
        r#"fn main() { let _s = obfstr2::s_user!({ wrap_pct: 10, wrap_pct: 20 }, "hi"); }"#,
    ),
    (
        "i-user-deadloop",
        "min_security 不能 >= 100",
        r#"fn main() { let _n = obfstr2::i_user!({ min_security: 100 }, 42u32); }"#,
    ),
    (
        "fl-user-range",
        "不能大于上界",
        r#"fn main() { let _f = obfstr2::fl_user!({ anti_disasm_count_range: 5..=1 }, 3.14); }"#,
    ),
    (
        "s-user-comma",
        "后缺少逗号",
        r#"fn main() { let _s = obfstr2::s_user!({} "hi"); }"#,
    ),
    (
        "s-user-brace",
        "第一个参数必须是花括号配置块",
        r#"fn main() { let _s = obfstr2::s_user!("hi"); }"#,
    ),
    (
        "s-user-expr",
        "expected string literal",
        r#"fn main() { let x = String::from("hi"); let _s = obfstr2::s_user!({}, x); }"#,
    ),
    (
        "b2-lit",
        "整数字面量",
        r#"fn main() { let x = 1u8; let _b = obfstr2::b2!([x]); }"#,
    ),
    (
        "b2-str",
        "只接受字节串字面量",
        r#"fn main() { let _b = obfstr2::b2!("hi"); }"#,
    ),
    (
        "f2-missing",
        "无法读取文件",
        r#"fn main() { let _b = obfstr2::f2!("不存在的文件.bin"); }"#,
    ),
    (
        "i2-range",
        "超出 u8 范围",
        r#"fn main() { let _x = obfstr2::i2!(256u8); }"#,
    ),
    (
        "i2-neg-unsigned",
        "负数不能用于无符号",
        r#"fn main() { let _x = obfstr2::i2!(-1u8); }"#,
    ),
    (
        "i2-lit",
        "只接受整数字面量",
        r#"fn main() { let _x = obfstr2::i2!(u64::MAX); }"#,
    ),
    (
        "i2-suffix",
        "后缀",
        r#"fn main() { let _x = obfstr2::i2!(1u256); }"#,
    ),
    (
        "fl2-inf",
        "无穷",
        r#"fn main() { let _x = obfstr2::fl2!(1e999f32); }"#,
    ),
    (
        "fl2-lit",
        "只接受浮点字面量",
        r#"fn main() { let _x = obfstr2::fl2!(f64::MAX); }"#,
    ),
    (
        "fl2-suffix",
        "后缀",
        r#"fn main() { let _x = obfstr2::fl2!(1.0f16); }"#,
    ),
    (
        "cs2-nul",
        "内部 NUL",
        r#"fn main() { let _s = obfstr2::cs2!("a\0b"); }"#,
    ),
    (
        "cs2-bytenul",
        "内部 NUL",
        r#"fn main() { let _s = obfstr2::cs2!(b"hi\0"); }"#,
    ),
    (
        "cs2-lit",
        "只接受字符串",
        r#"fn main() { let e = "x"; let _s = obfstr2::cs2!(e); }"#,
    ),
    (
        "cw2-nul",
        "内部 NUL",
        r#"fn main() { let _s = obfstr2::cw2!("a\0b"); }"#,
    ),
    (
        "cw2-lit",
        "只接受字符串字面量",
        r#"fn main() { let _s = obfstr2::cw2!(concat!("a", "b")); }"#,
    ),
    (
        "cw2-bytes",
        "只接受字符串字面量",
        r#"fn main() { let _s = obfstr2::cw2!(b"hi"); }"#,
    ),
    (
        "w2-lit",
        "只接受字符串字面量",
        r#"fn main() { let _s = obfstr2::w2!(concat!("a", "b")); }"#,
    ),
    (
        "w2-bytes",
        "只接受字符串字面量",
        r#"fn main() { let _s = obfstr2::w2!(b"hi"); }"#,
    ),
    (
        "s_fmt-lit",
        "必须是字符串字面量",
        r#"fn main() { let f = "x"; print!("{}", obfstr2::s_fmt1!(f)) }"#,
    ),
    (
        "s_fmt-brace",
        "未闭合",
        r#"fn main() { print!("{}", obfstr2::s_fmt1!("a{b")) }"#,
    ),
    // ── 动态密钥档：密钥配置校验 ──────────────────────────────────────────
    // 以下用例故意使用硬编码密钥 / 编译期字面量，用于验证编译期拒绝逻辑，
    // 不属于正常 *_key! 使用方式；运行期 roundtrip 见 tests/smoke.rs（统一读取
    // OBFSTR2_TEST_KEY 环境变量）。
    (
        "s-key-run-lit",
        "不能为字面量",
        r#"fn main() { let _s = obfstr2::s_key!({ build_key: "k", run_key: "k" }, "hi"); }"#,
    ),
    (
        "b-key-run-lit",
        "不能为字面量",
        r#"fn main() { let _s = obfstr2::b_key!({ build_key: "k", run_key: b"k" }, b"hi"); }"#,
    ),
    (
        "s-key-run-asbytes",
        "不能为字面量",
        r#"fn main() { let _s = obfstr2::s_key!({ build_key: "k", run_key: "k".as_bytes() }, "hi"); }"#,
    ),
    (
        "s-key-run-array",
        "不能为字面量",
        r#"fn main() { let _s = obfstr2::s_key!({ build_key: "k", run_key: &[0x6b] }, "hi"); }"#,
    ),
    (
        "s-key-no-build",
        "缺少必填字段 `build_key`",
        r#"fn main() { let k: &[u8] = b"k"; let _s = obfstr2::s_key!({ run_key: k }, "hi"); }"#,
    ),
    (
        "s-key-no-run",
        "缺少必填字段 `run_key`",
        r#"fn main() { let _s = obfstr2::s_key!({ build_key: "k" }, "hi"); }"#,
    ),
    (
        "s-key-unknown",
        "未知的密钥配置字段",
        r#"fn main() { let k: &[u8] = b"k"; let _s = obfstr2::s_key!({ build_key: "k", run_key: k, foo: 1 }, "hi"); }"#,
    ),
    (
        "s-key-dup",
        "重复配置字段",
        r#"fn main() { let k: &[u8] = b"k"; let _s = obfstr2::s_key!({ build_key: "k", build_key: "k2", run_key: k }, "hi"); }"#,
    ),
    (
        "s-key-empty",
        "`build_key` 密钥内容不能为空",
        r#"fn main() { let k: &[u8] = b"k"; let _s = obfstr2::s_key!({ build_key: "", run_key: k }, "hi"); }"#,
    ),
    (
        "s-key-invalid",
        "`build_key` 只接受",
        r#"fn main() { let k: &[u8] = b"k"; let _s = obfstr2::s_key!({ build_key: 42, run_key: k }, "hi"); }"#,
    ),
    (
        "s-key-env-missing",
        "编译期未找到环境变量",
        r#"fn main() { let k: &[u8] = b"k"; let _s = obfstr2::s_key!({ build_key: env!("OBFSTR2_TEST_MISSING_KEY_QUERY"), run_key: k }, "hi"); }"#,
    ),
    (
        "s-key-comma",
        "后缺少逗号",
        r#"fn main() { let k: &[u8] = b"k"; let _s = obfstr2::s_key!({ build_key: "k", run_key: k } "hi"); }"#,
    ),
    (
        "s-key-user-cfg",
        "第二个参数必须是花括号配置块",
        r#"fn main() { let k: &[u8] = b"k"; let _s = obfstr2::s_key_user!({ build_key: "k", run_key: k }, "hi"); }"#,
    ),
];

#[test]
fn all_invalid_inputs_rejected() {
    let dep = obfstr2_dep();
    let tasks = COMPILE_FAIL_CASES
        .iter()
        .map(|&(tag, _, code)| Task::new(tag, code).with_deps(&dep).with_release(false));

    // only_build = Some(true)：仅编译不运行；启用首任务依赖预热缓存
    let results = run_batch_use_cache(tasks, None, Some(true), None);

    // 1. 校验首任务依赖缓存预热是否成功
    let (init_tag, init_ret) = &results[0];
    assert_eq!(init_tag, "init_cache");
    assert!(
        init_ret.ok,
        "[init_cache] 预热依赖编译失败，stderr:\n{}",
        init_ret.stderr
    );

    // 2. 逐个校验编译失败用例（汇总所有错误，避免单个失败掩盖其余用例）
    let mut failures = Vec::new();
    for (&(expected_tag, needle, _), (actual_tag, ret)) in
        COMPILE_FAIL_CASES.iter().zip(results.iter().skip(1))
    {
        assert_eq!(expected_tag, actual_tag);
        if ret.ok {
            failures.push(format!("[{actual_tag}] 非法输入应当编译失败，但编译成功了"));
        } else if !ret.stderr.contains(needle) {
            failures.push(format!(
                "[{actual_tag}] 错误信息应包含 {needle:?}，实际 stderr:\n{}",
                ret.stderr
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "以下 {} 个编译失败测试未通过：\n\n{}",
        failures.len(),
        failures.join("\n----------------------------------------\n")
    );
}
