// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `s_fmt!` 与标准 `format!` 的等价性测试（进程内断言）。
use obfstr2::{s_fmt1, s_fmt2, s_fmt3};

#[test]
fn plain_text() {
    assert_eq!(s_fmt1!("hello world"), "hello world");
}

#[test]
fn escaped_braces() {
    assert_eq!(s_fmt1!("{{a}} b"), "{a} b");
}

#[test]
fn positional_and_named_args() {
    assert_eq!(
        s_fmt1!("{0} {name}", 1, name = "x"),
        format!("{0} {name}", 1, name = "x")
    );
}

#[test]
fn format_spec_width_precision() {
    assert_eq!(
        s_fmt1!("{x:>8.2}", x = 1.23456789_f64),
        format!("{x:>8.2}", x = 1.23456789_f64)
    );
}

#[test]
fn debug_pretty_spec_multiline() {
    let v = vec![1, 2, 3];
    assert_eq!(s_fmt1!("{v:#?}", v = v.clone()), format!("{v:#?}", v = v));
}

#[test]
fn mixed_text_and_placeholder_unicode() {
    assert_eq!(
        s_fmt1!("你好，{name}！今天是{day}号", name = "张三", day = 5),
        format!("你好，{name}！今天是{day}号", name = "张三", day = 5),
    );
}

#[test]
fn trailing_comma_in_args() {
    assert_eq!(s_fmt1!("{}", 42,), format!("{}", 42,));
}

#[test]
fn empty_and_braces_only() {
    assert_eq!(s_fmt1!(""), String::new());
    assert_eq!(s_fmt1!("{{}}"), "{}");
}

#[test]
fn all_tiers_agree_with_format() {
    let name = "world";
    let expected = format!("hello, {name}! length={}", 5);
    assert_eq!(s_fmt1!("hello, {name}! length={}", 5), expected);
    assert_eq!(s_fmt2!("hello, {name}! length={}", 5), expected);
    assert_eq!(s_fmt3!("hello, {name}! length={}", 5), expected);
    assert_eq!(s_fmt2!("plain text"), "plain text");
    assert_eq!(s_fmt3!("{{}}"), "{}");
}
