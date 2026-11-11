// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 多态密文存储策略注册表（`Storage` + `STORAGES`）。
//!

use lib_unknown::encoding::base64::base64_encode_vec;
use lib_unknown::rand::core::{fill_bytes, random, random_range};
use proc_macro2::{
    Ident as Ident2, Literal as Literal2, Span as Span2, TokenStream as TokenStream2,
};
use quote::{format_ident, quote};
use std::sync::LazyLock;

#[derive(Clone)]
pub(crate) struct Storage {
    #[allow(dead_code)]
    pub(crate) name: &'static str,
    /// 完整的变量声明与数据填充 AST。
    /// - `ident`: 输出变量标识符
    /// - `family`: 裸容器家族路径（如 `StackBytes` / `HeapBytes`，不含泛型参数）
    /// - `data`: 密文原始字节；
    pub(crate) ast: fn(&Ident2, &TokenStream2, &[u8]) -> TokenStream2,
    // 判断当前 chunk 长度是否适合此策略
    pub(crate) support: fn(usize) -> bool,
    // 抗分析能力（如伪装度，打断静态扫描的能力） 0~100
    #[allow(dead_code)]
    pub(crate) security: u8,
    // 运行时恢复数据的性能开销 0~100
    pub(crate) latency: u8,
}

/// 十六进制字符串存储（MAC/UUID/IPv6）的运行时解码片段。
fn hex_str_decode(
    ident: &Ident2,
    family: &TokenStream2,
    size: usize,
    const_name: &Ident2,
) -> TokenStream2 {
    quote! {
        let mut #ident = #family::<#size>::new();
        {
            let mut _written = 0usize;
            let mut _buf = #family::<16>::new();
            unsafe {
                _buf.extend_from_slice(&[0u8; 16]).unwrap_unchecked();
            }

            for &s in ::core::hint::black_box(#const_name) {
                if _written >= #size { break; }
                let _rem = #size - _written;
                let _take = if _rem < 16 { _rem } else { 16 };
                let (_, _n) = ::obfstr2::encoding::hex::hex_decode(
                    ::core::hint::black_box(s.as_bytes()),
                    &mut _buf[.._take],
                );
                unsafe {
                    #ident.extend_from_slice(&_buf[.._n]).unwrap_unchecked();
                }
                _written += _n;
            }
        }
    }
}

/// 整型数组存储策略生成宏（U64/U128 共用）。
macro_rules! int_array_storage {
    ($tag:ident, $ty:ty, $suffixed:ident, $width:expr) => {
        Storage {
            name: stringify!($tag),
            ast: |ident, family, data| {
                let size = data.len();
                let mut ints = Vec::new();
                for chunk in data.chunks($width) {
                    let mut buf = [0u8; $width];
                    if chunk.len() < $width {
                        fill_bytes(&mut buf);
                    }
                    buf[..chunk.len()].copy_from_slice(chunk);
                    ints.push(Literal2::$suffixed(<$ty>::from_le_bytes(buf)));
                }
                let const_name = Ident2::new(
                    &format!("__STATIC_{}_{}", stringify!($tag), random::<u32>()),
                    Span2::call_site(),
                );
                quote! {
                    static #const_name: &[$ty] = &[#(#ints),*];
                    let mut #ident = #family::<#size>::new();
                    let mut _current_len = 0usize;
                    for &val in ::core::hint::black_box(#const_name) {
                        if _current_len >= #size { break; }
                        let b = ::core::hint::black_box(val).to_le_bytes();
                        let remaining = #size - _current_len;
                        let copy_len = if remaining > $width { $width } else { remaining };
                        unsafe { #ident.extend_from_slice(&b[..copy_len]).unwrap_unchecked(); }
                        _current_len += copy_len;
                    }
                }
            },
            support: |size| (128..=4096).contains(&size),
            security: 3,
            latency: 10,
        }
    };
}

struct PemProfile {
    tag: &'static str,
    min_bytes: usize,
    max_bytes: usize,
}
const PEM_PROFILES: &[PemProfile] = &[
    PemProfile {
        tag: "CERTIFICATE",
        min_bytes: 800,
        max_bytes: 1400,
    },
    PemProfile {
        tag: "X509 CERTIFICATE",
        min_bytes: 800,
        max_bytes: 1400,
    },
    PemProfile {
        tag: "CERTIFICATE REQUEST",
        min_bytes: 600,
        max_bytes: 1000,
    },
    PemProfile {
        tag: "NEW CERTIFICATE REQUEST",
        min_bytes: 600,
        max_bytes: 1000,
    },
    PemProfile {
        tag: "RSA PRIVATE KEY",
        min_bytes: 1100,
        max_bytes: 1700,
    },
    PemProfile {
        tag: "PRIVATE KEY",
        min_bytes: 700,
        max_bytes: 1200,
    },
    PemProfile {
        tag: "EC PRIVATE KEY",
        min_bytes: 180,
        max_bytes: 320,
    },
    PemProfile {
        tag: "PUBLIC KEY",
        min_bytes: 200,
        max_bytes: 400,
    },
    PemProfile {
        tag: "RSA PUBLIC KEY",
        min_bytes: 250,
        max_bytes: 450,
    },
];

pub(crate) static STORAGES: LazyLock<Vec<Storage>> = LazyLock::new(|| {
    vec![
        // 原始字节切片策略
        Storage {
            name: "bytes",
            ast: |ident, family, data| {
                let size = data.len();
                let enc_literal = Literal2::byte_string(data);
                let enc_ident = format_ident!("__STATIC_B_{}", random::<u32>());
                quote! {
                    static #enc_ident: &[u8] = #enc_literal;
                    let mut #ident = #family::<#size>::new();
                    unsafe {
                        #ident
                            .extend_from_slice(::core::hint::black_box(#enc_ident))
                            .unwrap_unchecked();
                    }
                }
            },
            support: |_| true,
            security: 1,
            latency: 1,
        },
        int_array_storage!(U64, u64, u64_suffixed, 8),
        int_array_storage!(U128, u128, u128_suffixed, 16),
        // MAC 地址隐写存储策略
        Storage {
            name: "mac",
            ast: |ident, family, data| {
                let size = data.len();
                let mut macs = Vec::new();
                for chunk in data.chunks(6) {
                    let mut buf = [0u8; 6];
                    if chunk.len() < 6 {
                        fill_bytes(&mut buf);
                    }
                    buf[..chunk.len()].copy_from_slice(chunk);
                    let mac_str = format!(
                        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}\0",
                        buf[0], buf[1], buf[2], buf[3], buf[4], buf[5]
                    );
                    macs.push(mac_str);
                }
                let const_name = format_ident!("__STATIC_MAC_{}", random::<u32>());
                let decode = hex_str_decode(ident, family, size, &const_name);
                quote! {
                    static #const_name: &[&str] = &[#(#macs),*];
                    #decode
                }
            },
            support: |size| (6..=386).contains(&size),
            security: 30,
            latency: 10,
        },
        // UUID 隐写存储策略
        Storage {
            name: "uuid",
            ast: |ident, family, data| {
                let size = data.len();
                let mut uuids = Vec::new();
                for chunk in data.chunks(16) {
                    let mut buf = [0u8; 16];
                    if chunk.len() < 16 {
                        fill_bytes(&mut buf);
                    }
                    buf[..chunk.len()].copy_from_slice(chunk);
                    let uuid_str = format!(
                        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}\0",
                        buf[0],
                        buf[1],
                        buf[2],
                        buf[3],
                        buf[4],
                        buf[5],
                        buf[6],
                        buf[7],
                        buf[8],
                        buf[9],
                        buf[10],
                        buf[11],
                        buf[12],
                        buf[13],
                        buf[14],
                        buf[15]
                    );
                    uuids.push(uuid_str);
                }
                let const_name = format_ident!("__STATIC_UUID_{}", random::<u32>());
                let decode = hex_str_decode(ident, family, size, &const_name);
                quote! {
                    static #const_name: &[&str] = &[#(#uuids),*];
                    #decode
                }
            },
            support: |size| (8..=512).contains(&size),
            security: 30,
            latency: 10,
        },
        // IPv6 隐写存储策略
        Storage {
            name: "ipv6",
            ast: |ident, family, data| {
                let size = data.len();
                let mut ipv6s = Vec::new();
                for chunk in data.chunks(16) {
                    let mut buf = [0u8; 16];
                    if chunk.len() < 16 {
                        fill_bytes(&mut buf);
                    }
                    buf[..chunk.len()].copy_from_slice(chunk);
                    let ipv6_str = format!(
                        "{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}\0",
                        buf[0],
                        buf[1],
                        buf[2],
                        buf[3],
                        buf[4],
                        buf[5],
                        buf[6],
                        buf[7],
                        buf[8],
                        buf[9],
                        buf[10],
                        buf[11],
                        buf[12],
                        buf[13],
                        buf[14],
                        buf[15]
                    );
                    ipv6s.push(ipv6_str);
                }
                let const_name = format_ident!("__STATIC_IPV6_{}", random::<u32>());
                let decode = hex_str_decode(ident, family, size, &const_name);
                quote! {
                    static #const_name: &[&str] = &[#(#ipv6s),*];
                    #decode
                }
            },
            support: |size| (8..=512).contains(&size),
            security: 30,
            latency: 10,
        },
        // PEM 格式拟真存储策略（单连续字符串 + 抗 LLVM DCE 优化）
        Storage {
            name: "pem",
            ast: |ident, family, data| {
                let size = data.len();

                // 1. 随机选取一个 PEM Profile
                let profile_idx = random_range(0..PEM_PROFILES.len());
                let profile = &PEM_PROFILES[profile_idx];

                // 2. 根据 profile 决定总长度
                let min_target = profile.min_bytes.max(size);
                let max_target = profile.max_bytes.max(min_target + 64);
                let mut target_len = random_range(min_target..=max_target);

                // 3. 保证总长度是对齐 3 的倍数，避免生成 '=' padding 字符
                if target_len % 3 != 0 {
                    target_len += 3 - (target_len % 3);
                }

                // 4. 前半部分真实数据，后半部分随机噪声填充
                let mut padded = Vec::with_capacity(target_len);
                padded.extend_from_slice(data);

                let pad_len = target_len - size;
                if pad_len > 0 {
                    let mut dummy_bytes = vec![0u8; pad_len];
                    fill_bytes(&mut dummy_bytes);
                    padded.extend_from_slice(&dummy_bytes);
                }

                // 5. 进行标准 Base64 编码
                let b64 = base64_encode_vec(&padded, false);

                // 6. 拼接为完整的单字符串 PEM 文本（保证内存中绝对连续）
                let tag = profile.tag;
                let mut pem_str = String::with_capacity(b64.len() + 128);
                pem_str.push_str(&format!("-----BEGIN {}-----\n", tag));
                for chunk in b64.chunks(64) {
                    pem_str.push_str(&String::from_utf8_lossy(chunk));
                    pem_str.push('\n');
                }
                pem_str.push_str(&format!("-----END {}-----\n", tag));

                let const_name = format_ident!("__STATIC_PEM_{}", random::<u32>());

                quote! {
                    // 单一连续字符串常量，强制分配在一段连续的 .rodata 空间内
                    static #const_name: &str = #pem_str;
                    let mut #ident = #family::<#size>::new();
                    {
                        let mut _written = 0usize;
                        let mut _buf = #family::<48>::new();
                        unsafe {
                            _buf.extend_from_slice(&[0u8; 48]).unwrap_unchecked();
                        }

                        // 将整个连续 PEM 切片通过 black_box 保护
                        let _pem_bytes = ::core::hint::black_box(#const_name.as_bytes());
                        let mut _pos = 0usize;

                        'pem_outer: while _pos < _pem_bytes.len() && _written < #size {
                            // 查找下一行行尾 (\n)
                            let mut _end = _pos;
                            while _end < _pem_bytes.len() && _pem_bytes[_end] != b'\n' {
                                _end += 1;
                            }
                            let _line = &_pem_bytes[_pos.._end];
                            _pos = _end + 1;

                            // 如果遇到空行或头尾标识（以 '-' 开头）：
                            // 使用 black_box 标记引用，向 LLVM 表明头尾数据已被实际访问，不可剔除
                            if _line.is_empty() || _line[0] == b'-' {
                                ::core::hint::black_box(_line);
                                continue;
                            }

                            let _rem = #size - _written;
                            let _take = if _rem < 48 { _rem } else { 48 };

                            let (_, _n) = ::obfstr2::encoding::base64::base64_decode(
                                ::core::hint::black_box(_line),
                                &mut _buf[.._take],
                            );
                            unsafe {
                                #ident.extend_from_slice(&_buf[.._n]).unwrap_unchecked();
                            }
                            _written += _n;
                            if _written >= #size { break 'pem_outer; }
                        }
                    }
                }
            },
            support: |size| (128..=10240).contains(&size),
            security: 45,
            latency: 12,
        },
    ]
});

#[cfg(test)]
mod tests {
    use super::*;
    use lib_unknown::dyntest::DnyRun;
    use lib_unknown::rand::core::fill_bytes;
    use quote::{format_ident, quote};
    use std::fs;
    use std::time::Duration;

    fn obfstr2_dep() -> String {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
        format!("obfstr2 = {{ path = {:?} }}", root)
    }

    #[test]
    #[ignore = "需构建 dyntest 临时 guest 工程（含依赖拉取），约数秒"]
    fn test_storages() {
        let mut runner = DnyRun::new("fn main() {}", &obfstr2_dep());
        runner.release(true);
        let ret = runner.build(Some(Duration::from_secs(10)));
        if !ret.ok {
            println!("{}", ret);
            panic!();
        }

        let mut data = vec![0u8; 1024];
        fill_bytes(data.as_mut());

        let template = r#"
fn main() {
[[d]]
for b in &*d {
    print!("{:02x}", b);
}
for _ in 0..core::hint::black_box(100000) {
[[d]]
core::hint::black_box(d);
}
}"#;
        let code = |f: fn(&Ident2, &TokenStream2, &[u8]) -> TokenStream2, d| {
            f(
                &format_ident!("d"),
                &quote!(::obfstr2::types::buffer::StackBytes),
                d,
            )
            .to_string()
        };
        for storage in &*STORAGES {
            let main = template.replace("[[d]]", &code(storage.ast, &data));
            runner.reset_main_code(&main);
            let ret = runner.run(Some(Duration::from_secs(3)));
            let expected_hex: String = data.iter().map(|b| format!("{:02x}", b)).collect();
            assert_eq!(ret.stdout.trim(), expected_hex, "!!!!\n{ret}");
            println!(
                "[[{}]] {} kb",
                storage.name,
                fs::metadata(runner.bin_path())
                    .map(|m| m.len() / 1024)
                    .unwrap_or(0)
            );
            println!("{}", ret);
        }
    }
}
