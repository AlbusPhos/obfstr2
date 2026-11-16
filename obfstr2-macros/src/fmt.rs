// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `s_fmt!` 的格式串切分与重组逻辑（入口见 crate 根 [`s_fmt`](crate::s_fmt)）。
use lib_unknown::rand::core::random;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{LitStr, Token};

pub struct FormatArgs {
    pub fmt_lit: LitStr,
    pub rest: TokenStream2,
}

impl Parse for FormatArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let fmt_lit: LitStr = input
            .parse()
            .map_err(|e| syn::Error::new(e.span(), "s_fmt! 的第一个参数必须是字符串字面量"))?;

        let rest = if input.peek(Token![,]) {
            let _comma: Token![,] = input.parse()?;
            input.parse()?
        } else {
            TokenStream2::new()
        };

        Ok(FormatArgs { fmt_lit, rest })
    }
}

/// 拆分后的格式化字符串块
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum Chunk {
    Text(String),
    Placeholder(String),
}

/// 稳健地解析格式化字符串。
///
/// - `{{` / `}}` 视为转义字符；
/// - 其余 `{...}` 视为占位符，占位符内部支持：
///   - 嵌套花括号（按深度匹配，而非遇到第一个 `}` 就截断）；
///   - 单引号/双引号字符串、字符字面量内部的 `{` `}`（含转义）不会影响边界判断。
///
/// 这样可以正确处理诸如 `"{foo('}')}"`、`"{x:foo}"` 等复杂占位符，
/// 而不会在扫描时崩溃或产生“未配对”的误报。
pub fn parse_format_string(s: &str) -> Result<Vec<Chunk>, String> {
    let mut chunks = Vec::new();
    let mut chars = s.chars().peekable();
    let mut text = String::new();

    while let Some(c) = chars.next() {
        if c == '{' {
            if chars.peek() == Some(&'{') {
                chars.next();
                text.push_str("{{");
            } else {
                if !text.is_empty() {
                    chunks.push(Chunk::Text(std::mem::take(&mut text)));
                }
                let ph = parse_placeholder(&mut chars)?;
                chunks.push(Chunk::Placeholder(ph));
            }
        } else if c == '}' {
            if chars.peek() == Some(&'}') {
                chars.next();
                text.push_str("}}");
            } else {
                return Err("格式串中 '}' 未配对".into());
            }
        } else {
            text.push(c);
        }
    }
    if !text.is_empty() {
        chunks.push(Chunk::Text(text));
    }
    Ok(chunks)
}

/// 解析一个已消费掉起始 `{` 的占位符，返回包含首尾花括号的完整文本。
///
/// 采用花括号深度计数 + 字符串/字符字面量状态机：
/// - 进入 `'` 或 `"` 后视为进入字面量模式，期间的 `{` `}` 被忽略，
///   直到遇到未被 `\` 转义的同类引号才退出；
/// - 不在字面量模式时，`{` 增加深度、`}` 减少深度，深度归零即闭合。
fn parse_placeholder(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<String, String> {
    let mut ph = String::from("{");
    let mut depth: i32 = 1;
    let mut in_string: Option<char> = None;
    let mut escaped = false;
    let mut closed = false;

    for pc in chars.by_ref() {
        ph.push(pc);

        if let Some(quote_char) = in_string {
            if escaped {
                escaped = false;
            } else if pc == '\\' {
                escaped = true;
            } else if pc == quote_char {
                in_string = None;
            }
            continue;
        }

        match pc {
            '\'' | '"' => in_string = Some(pc),
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    closed = true;
                    break;
                }
            }
            _ => {}
        }
    }

    if !closed {
        return if in_string.is_some() {
            Err("格式串占位符中的字符串/字符字面量未闭合".into())
        } else {
            Err("格式串中 '{' 未闭合".into())
        };
    }

    Ok(ph)
}

/// 按档位混淆格式串中的字面量部分，展开为 `alloc::format!` 调用（返回 `String`，需 `alloc`，`no_std` 可用）。
///
/// `build` 为字面量片段的档位入口（`s1` / `s2` / `s3` / `s_user`）；调用方（`s_fmt1/2/3`
/// 与 `s_fmt_user` 本体）传入对应档位。`s_fmt_user` 以闭包捕获自定义 `TierParams`，
/// 故 `build` 为 `impl Fn` 而非函数指针。
pub fn sfmt(
    input: TokenStream2,
    build: impl Fn(String) -> Result<TokenStream2, String>,
) -> TokenStream2 {
    let args = match syn::parse2::<FormatArgs>(input) {
        Ok(args) => args,
        Err(err) => return err.to_compile_error(),
    };
    let fmt_str = args.fmt_lit.value();

    let chunks = match parse_format_string(&fmt_str) {
        Ok(c) => c,
        Err(err) => {
            return syn::Error::new(args.fmt_lit.span(), err).to_compile_error();
        }
    };

    // 每次宏展开生成一个随机后缀，降低生成的注入参数标识符
    // 与调用方传入参数（或其他宏展开）同名冲突的概率。
    let suffix: u64 = random();

    let mut new_fmt = String::new();
    let mut obf_injections = Vec::new();
    let mut obf_idx = 0usize;

    for chunk in chunks {
        match chunk {
            Chunk::Text(text) => {
                let unescaped_text = text.replace("{{", "{").replace("}}", "}");
                let arg_name = quote::format_ident!("__s_fmt_chunk_{}_{}__", obf_idx, suffix);

                new_fmt.push_str(&format!("{{{arg_name}}}"));

                let ts = match build(unescaped_text) {
                    Ok(ts) => ts,
                    Err(e) => {
                        return syn::Error::new(args.fmt_lit.span(), e).to_compile_error();
                    }
                };
                obf_injections.push(quote! { #arg_name = #ts });

                obf_idx += 1;
            }
            Chunk::Placeholder(ph) => {
                new_fmt.push_str(&ph);
            }
        }
    }

    let new_fmt_lit = LitStr::new(&new_fmt, args.fmt_lit.span());
    let mut final_args = args.rest;

    let has_trailing_comma = final_args
        .clone()
        .into_iter()
        .last()
        .is_some_and(|tt| matches!(tt, proc_macro2::TokenTree::Punct(p) if p.as_char() == ','));

    if !final_args.is_empty() && !obf_injections.is_empty() && !has_trailing_comma {
        final_args.extend(quote! { , });
    }

    if !obf_injections.is_empty() {
        let inj_tokens = quote! { #(#obf_injections),* };
        final_args.extend(inj_tokens);
    }

    quote! {
        {
            extern crate alloc;
            alloc::format!(#new_fmt_lit, #final_args)
        }
    }
}
