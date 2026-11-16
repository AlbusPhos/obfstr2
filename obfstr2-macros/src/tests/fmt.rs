// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `crate::fmt` 格式串切分单测（自 `fmt.rs` 内联 `parse_tests` 模块迁移）：
//! 占位符嵌套花括号、字符串/字符字面量内部花括号、转义序列与边界行为。

#![allow(clippy::expect_used)]

use crate::fmt::{Chunk, parse_format_string};

fn ok(s: &str) -> Vec<Chunk> {
    parse_format_string(s).unwrap_or_else(|e| panic!("解析 {s:?} 失败: {e}"))
}

fn err(s: &str) -> String {
    match parse_format_string(s) {
        Ok(v) => panic!("{s:?} 应当解析失败，实际得到 {v:?}"),
        Err(e) => e,
    }
}

#[test]
fn plain_text() {
    assert_eq!(ok("hello world"), vec![Chunk::Text("hello world".into())]);
}

#[test]
fn empty_input() {
    assert_eq!(ok(""), Vec::<Chunk>::new());
}

#[test]
fn escaped_braces_only() {
    assert_eq!(ok("{{a}}"), vec![Chunk::Text("{{a}}".into())]);
}

#[test]
fn simple_and_named_placeholder() {
    assert_eq!(
        ok("{} {name}"),
        vec![
            Chunk::Placeholder("{}".into()),
            Chunk::Text(" ".into()),
            Chunk::Placeholder("{name}".into()),
        ]
    );
}

#[test]
fn placeholder_with_format_spec() {
    // {x:foo} —— 冒号后的内容是否合法由最终 alloc::format! 校验，
    // 这里只验证切分阶段把整体当成一个占位符。
    assert_eq!(ok("{x:foo}"), vec![Chunk::Placeholder("{x:foo}".into())]);
}

#[test]
fn placeholder_with_nested_braces() {
    assert_eq!(
        ok("{foo({bar})}"),
        vec![Chunk::Placeholder("{foo({bar})}".into())]
    );
}

#[test]
fn placeholder_with_char_literal_containing_brace() {
    // '}' 内部的 } 不应被当成占位符结束 —— 这是本次修复的核心场景
    assert_eq!(
        ok("{foo('}')}"),
        vec![Chunk::Placeholder("{foo('}')}".into())]
    );
}

#[test]
fn placeholder_with_string_literal_containing_brace() {
    assert_eq!(
        ok(r#"{foo("}")}"#),
        vec![Chunk::Placeholder(r#"{foo("}")}"#.into())]
    );
}

#[test]
fn placeholder_with_escaped_quote_in_string() {
    // 字符串内部出现被转义的引号 \" ，不应提前结束字符串模式
    assert_eq!(
        ok(r#"{foo("a\"}b")}"#),
        vec![Chunk::Placeholder(r#"{foo("a\"}b")}"#.into())]
    );
}

#[test]
fn placeholder_with_escaped_backslash_before_quote() {
    // 边界：\\" 中的 \\ 是转义自己，后面的 " 仍然应该正常结束字符串
    // r#"{foo("a\\")}"# 对应真实内容：{foo("a\")}
    assert_eq!(
        ok(r#"{foo("a\\")}"#),
        vec![Chunk::Placeholder(r#"{foo("a\\")}"#.into())]
    );
}

#[test]
fn mixed_text_and_placeholders() {
    assert_eq!(
        ok("pre{a}mid{b:?}post"),
        vec![
            Chunk::Text("pre".into()),
            Chunk::Placeholder("{a}".into()),
            Chunk::Text("mid".into()),
            Chunk::Placeholder("{b:?}".into()),
            Chunk::Text("post".into()),
        ]
    );
}

#[test]
fn consecutive_placeholders() {
    assert_eq!(
        ok("{a}{b}"),
        vec![
            Chunk::Placeholder("{a}".into()),
            Chunk::Placeholder("{b}".into()),
        ]
    );
}

#[test]
fn escape_adjacent_to_placeholder() {
    // {{ 紧跟占位符，验证两者不会相互吞噬
    assert_eq!(
        ok("{{{a}}}"),
        vec![
            Chunk::Text("{{".into()),
            Chunk::Placeholder("{a}".into()),
            Chunk::Text("}}".into()),
        ]
    );
}

#[test]
fn unmatched_closing_brace_errors() {
    let e = err("a}b");
    assert!(e.contains("未配对"), "错误信息: {e}");
}

#[test]
fn unterminated_placeholder_errors() {
    let e = err("{abc");
    assert!(e.contains("未闭合"), "错误信息: {e}");
}

#[test]
fn unterminated_string_in_placeholder_errors() {
    let e = err(r#"{foo("abc}"#);
    assert!(e.contains("字符串/字符字面量未闭合"), "错误信息: {e}");
}

#[test]
fn deeply_nested_braces_does_not_overflow_or_panic() {
    // 构造较深的嵌套，确认不会栈溢出/死循环（depth 计数不会越界）
    let depth = 1000;
    let mut s = String::from("{");
    s.push_str(&"{".repeat(depth));
    s.push_str(&"}".repeat(depth));
    s.push('}');
    assert!(parse_format_string(&s).is_ok());
}

/// 已知限制：raw string 前缀（r"..."/r#"..."#/b"..."）未被状态机识别。
/// 这里显式记录"当前行为"，而不是断言它是"正确"的——
/// 一旦未来实现 raw string 支持，这个测试需要同步更新（最好改成断言正确切分）。
#[test]
fn known_limitation_raw_string_inside_placeholder() {
    let input = r###"{foo(r#"a}b"#)}"###;
    // 当前只保证“不 panic”，不保证切分结果语义正确。
    let _ = parse_format_string(input);
}
