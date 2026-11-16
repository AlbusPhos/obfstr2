// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 非法输入测试：越界元素 / 缺失文件 / 非字面量 / 未闭合括号均报编译错误。
//!
//! 由原 `obfstr2-macros/src/lib.rs::tests` 迁移而来。每个用例独立
//! `dny_run`（目录唯一），无共享 `RUNNER`、无 `clear_dny_project`。
//! 未引入 `trybuild`：其快照 rudiment 在离线环境不可用，且此处仅需断言
//! `stderr` 关键信息，`dny_run` 足够。
use lib_unknown::dyntest::dny_run;

fn obfstr2_dep() -> String {
    format!("obfstr2 = {{ path = {:?} }}", env!("CARGO_MANIFEST_DIR"))
}

fn assert_compile_fail(code: &str, needle: &str, tag: &str) {
    let ret = dny_run(code, &obfstr2_dep(), None, false);
    assert!(!ret.ok, "[{tag}] 非法输入应当编译失败");
    assert!(
        ret.stderr.contains(needle),
        "[{tag}] 错误信息应包含 {needle:?}，实际 stderr:\n{}",
        ret.stderr
    );
}

#[test]
fn bytes_array_out_of_range_rejected() {
    assert_compile_fail(
        r#"fn main() { let _b = obfstr2::b2!([0x61, 300]); }"#,
        "0..=255",
        "b2-range",
    );
}

#[test]
fn user_unknown_field_rejected() {
    assert_compile_fail(
        r#"fn main() { let _s = obfstr2::s_user!({ unknown_field: 1 }, "hi"); }"#,
        "未知的 TierParams 字段",
        "s-user-unknown",
    );
}

#[test]
fn user_duplicate_field_rejected() {
    assert_compile_fail(
        r#"fn main() { let _s = obfstr2::s_user!({ wrap_pct: 10, wrap_pct: 20 }, "hi"); }"#,
        "重复配置字段",
        "s-user-dup",
    );
}

#[test]
fn user_min_security_deadloop_rejected() {
    assert_compile_fail(
        r#"fn main() { let _n = obfstr2::i_user!({ min_security: 100 }, 42u32); }"#,
        "min_security 不能 >= 100",
        "i-user-deadloop",
    );
}

#[test]
fn user_inverted_range_rejected() {
    assert_compile_fail(
        r#"fn main() { let _f = obfstr2::fl_user!({ anti_disasm_count_range: 5..=1 }, 3.14); }"#,
        "不能大于上界",
        "fl-user-range",
    );
}

#[test]
fn user_missing_comma_rejected() {
    assert_compile_fail(
        r#"fn main() { let _s = obfstr2::s_user!({} "hi"); }"#,
        "后缺少逗号",
        "s-user-comma",
    );
}

#[test]
fn user_brace_required_rejected() {
    assert_compile_fail(
        r#"fn main() { let _s = obfstr2::s_user!("hi"); }"#,
        "第一个参数必须是花括号配置块",
        "s-user-brace",
    );
}

#[test]
fn user_payload_rejects_expression() {
    // 与 sN! 一致：s 系载荷经 syn::LitStr 直接解析，非字面量报 syn 的标准文案
    assert_compile_fail(
        r#"fn main() { let x = String::from("hi"); let _s = obfstr2::s_user!({}, x); }"#,
        "expected string literal",
        "s-user-expr",
    );
}

#[test]
fn bytes_array_non_literal_rejected() {
    assert_compile_fail(
        r#"fn main() { let x = 1u8; let _b = obfstr2::b2!([x]); }"#,
        "整数字面量",
        "b2-lit",
    );
}

#[test]
fn bytes_string_literal_rejected() {
    assert_compile_fail(
        r#"fn main() { let _b = obfstr2::b2!("hi"); }"#,
        "只接受字节串字面量",
        "b2-str",
    );
}

#[test]
fn missing_file_rejected() {
    assert_compile_fail(
        r#"fn main() { let _b = obfstr2::f2!("不存在的文件.bin"); }"#,
        "无法读取文件",
        "f2-missing",
    );
}

#[test]
fn int_out_of_range_rejected() {
    assert_compile_fail(
        r#"fn main() { let _x = obfstr2::i2!(256u8); }"#,
        "超出 u8 范围",
        "i2-range",
    );
}

#[test]
fn int_negative_for_unsigned_rejected() {
    assert_compile_fail(
        r#"fn main() { let _x = obfstr2::i2!(-1u8); }"#,
        "负数不能用于无符号",
        "i2-neg-unsigned",
    );
}

#[test]
fn int_non_literal_rejected() {
    assert_compile_fail(
        r#"fn main() { let _x = obfstr2::i2!(u64::MAX); }"#,
        "只接受整数字面量",
        "i2-lit",
    );
}

#[test]
fn int_bad_suffix_rejected() {
    assert_compile_fail(
        r#"fn main() { let _x = obfstr2::i2!(1u256); }"#,
        "后缀",
        "i2-suffix",
    );
}

#[test]
fn float_overflow_to_inf_rejected() {
    assert_compile_fail(
        r#"fn main() { let _x = obfstr2::fl2!(1e999f32); }"#,
        "无穷",
        "fl2-inf",
    );
}

#[test]
fn float_non_literal_rejected() {
    assert_compile_fail(
        r#"fn main() { let _x = obfstr2::fl2!(f64::MAX); }"#,
        "只接受浮点字面量",
        "fl2-lit",
    );
}

#[test]
fn float_bad_suffix_rejected() {
    assert_compile_fail(
        r#"fn main() { let _x = obfstr2::fl2!(1.0f16); }"#,
        "后缀",
        "fl2-suffix",
    );
}

#[test]
fn cstr_interior_nul_rejected() {
    assert_compile_fail(
        r#"fn main() { let _s = obfstr2::cs2!("a\0b"); }"#,
        "内部 NUL",
        "cs2-nul",
    );
}

#[test]
fn cstr_byte_str_nul_rejected() {
    assert_compile_fail(
        r#"fn main() { let _s = obfstr2::cs2!(b"hi\0"); }"#,
        "内部 NUL",
        "cs2-bytenul",
    );
}

#[test]
fn cstr_non_literal_rejected() {
    assert_compile_fail(
        r#"fn main() { let e = "x"; let _s = obfstr2::cs2!(e); }"#,
        "只接受字符串",
        "cs2-lit",
    );
}

#[test]
fn cwstr_interior_nul_rejected() {
    assert_compile_fail(
        r#"fn main() { let _s = obfstr2::cw2!("a\0b"); }"#,
        "内部 NUL",
        "cw2-nul",
    );
}

#[test]
fn cwstr_non_literal_rejected() {
    assert_compile_fail(
        r#"fn main() { let _s = obfstr2::cw2!(concat!("a", "b")); }"#,
        "只接受字符串字面量",
        "cw2-lit",
    );
}

#[test]
fn cwstr_byte_str_rejected() {
    assert_compile_fail(
        r#"fn main() { let _s = obfstr2::cw2!(b"hi"); }"#,
        "只接受字符串字面量",
        "cw2-bytes",
    );
}

#[test]
fn wstr_non_literal_rejected() {
    assert_compile_fail(
        r#"fn main() { let _s = obfstr2::w2!(concat!("a", "b")); }"#,
        "只接受字符串字面量",
        "w2-lit",
    );
}

#[test]
fn wstr_byte_str_rejected() {
    assert_compile_fail(
        r#"fn main() { let _s = obfstr2::w2!(b"hi"); }"#,
        "只接受字符串字面量",
        "w2-bytes",
    );
}

#[test]
fn s_fmt_non_literal_rejected() {
    assert_compile_fail(
        r#"fn main() { let f = "x"; print!("{}", obfstr2::s_fmt1!(f)) }"#,
        "必须是字符串字面量",
        "s_fmt-lit",
    );
}

#[test]
fn s_fmt_unclosed_brace_rejected() {
    assert_compile_fail(
        r#"fn main() { print!("{}", obfstr2::s_fmt1!("a{b")) }"#,
        "未闭合",
        "s_fmt-brace",
    );
}
