// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 多态密文存储策略注册表（`Storage` + `STORAGES`）。
//!

use lib_unknown::encoding::base64::base64_encode_vec;
use lib_unknown::encoding::hb2::{DEFAULT_TABLE as HB2_TABLE, hb2_encode_to_vec};
use lib_unknown::rand::core::{fill_bytes, random, random_range, shuffle};
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
        let mut #ident = unsafe { #family::<#size>::new_zeroed(#size).unwrap_unchecked() };
        {
            let mut _written = 0usize;
            // 直接解码到目标容器的字节视图：省掉 scratch 容器与其拷贝 / 擦除。
            for &s in ::core::hint::black_box(#const_name) {
                if _written >= #size { break; }
                let _rem = #size - _written;
                let _take = if _rem < 16 { _rem } else { 16 };
                let (_, _n) = ::obfstr2::encoding::hex::hex_decode(
                    ::core::hint::black_box(s.as_bytes()),
                    &mut #ident.as_mut_byte_slice()[_written.._written + _take],
                );
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

/// HB2 空白隐写的伪特征字符串表。
///
/// 来源为 OpenSSL / 常见系统库与程序中的可打印片段，统一裁剪到 ≤30 字节；
/// 不得包含 HB2 符号表字节（默认 `\0` / `\n`），否则会被解码器误读为比特。
const HB2_DECOYS: &[&str] = &[
    "type=%d",
    "TLS %s extension \"%s\"",
    "TLS ext \"%s\" (id=%d)",
    "error setting random cookie",
    "Error calculating HMAC-SHA1",
    "%s: %zu-byte buffer too large",
    "matched peer raw public key",
    "No peer cert or raw pubkey",
    "Call to SSL_CONF_cmd failed",
    "# SSL/TLS secrets log file",
    "No %s certificate CA names",
    "Acceptable %s CA names",
    "Secure Renegotiation IS%s",
    "TLS forbids renegotiation",
    "ALPN negotiation",
    "bad_cert_status_response",
    "BIO_ADDRINFO assertion failed",
    "Can't bind %s address",
    "SRP param N and g unknown",
    "SRP parameters set",
    "Failed to init SRP verifier",
    "Cannot init SRP verifier file",
    "0123456789ABCDEF",
    "save_free_certs",
    "handle_opt_geninfo",
    "set_fallback_pubkey",
    "setup_request_ctx",
    "setup_protection_ctx",
    "setup_verification_ctx",
    "write_PKIMESSAGE",
    "read_PKIMESSAGE",
    "read_write_req_resp",
    "setup_client_ctx",
    "./0123456789ABCDEFGHIJKLMNOPQR",
    "Lets print some clear text",
    "GET %s HTTP/1.0",
    "This is a key...",
    "app_http_get_asn1",
    "asn1=SEQUENCE:LDAPMessage",
    "messageID=INTEGER:1",
    "extendedReq=EXPLICIT:23A",
    "9049f5913b5bd5f8730cc1ce",
    ".note.gnu.build-id",
    ".note.gnu.property",
    "libvlc",
    "vlc_interface",
    "SSL_CTX_new",
    "ERR_clear_error",
];

/// 把密文编码为 HB2 空白符号串，并随机插入伪特征字符串（约为编码数据的 10%）。
///
/// 插入内容全部为表外可打印字节，`hb2_decode` 的宽松模式会自动跳过；
/// 为保证极短载荷也有伪装文本，预算不足一条字符串时仍至少插入一条。
fn hb2_storage_text(data: &[u8]) -> String {
    debug_assert!(
        HB2_DECOYS
            .iter()
            .all(|d| d.len() <= 30 && !d.as_bytes().iter().any(|b| HB2_TABLE.contains(b))),
        "HB2 decoys must be <=30 bytes and must not contain table symbols"
    );

    let mut out = hb2_encode_to_vec(data, HB2_TABLE);
    let budget = out.len() / 10;
    let mut inserted = 0usize;

    let mut order: Vec<usize> = (0..HB2_DECOYS.len()).collect();
    order = shuffle(order);

    let mut cursor = 0usize;
    while inserted < budget {
        let decoy = HB2_DECOYS[order[cursor % order.len()]];
        cursor += 1;

        let cost = decoy.len() + 1; // + 一个空格分隔符
        if inserted > 0 && inserted + cost > budget {
            break;
        }

        let pos = random_range(0..=out.len());
        out.splice(pos..pos, decoy.bytes().chain(::core::iter::once(b' ')));
        inserted += cost;
    }

    if inserted == 0 {
        let decoy = HB2_DECOYS[random_range(0..HB2_DECOYS.len())];
        let pos = random_range(0..=out.len());
        out.splice(pos..pos, decoy.bytes().chain(::core::iter::once(b' ')));
    }

    // HB2 符号与挑选出的伪特征串均为 ASCII，因此必定是合法 UTF-8。
    String::from_utf8(out).unwrap_or_else(|_| unreachable!("HB2 symbols and decoys are ASCII"))
}

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
            latency: 1,
        },
        int_array_storage!(U64, u64, u64_suffixed, 8),
        int_array_storage!(U128, u128, u128_suffixed, 16),
        // HB2 空白字符隐写：每个密文字节展开为 8 个 \0/\n 符号，随机夹入伪特征字符串（熵稀释专用）
        Storage {
            name: "hb2",
            ast: |ident, family, data| {
                let size = data.len();
                let text = hb2_storage_text(data);
                let encoded_literal = Literal2::string(&text);
                let const_name = format_ident!("__STATIC_HB2_{}", random::<u32>());
                quote! {
                    static #const_name: &str = #encoded_literal;
                    let mut #ident = unsafe { #family::<#size>::new_zeroed(#size).unwrap_unchecked() };
                    let (_, _hb2_written) = ::obfstr2::encoding::hb2::hb2_decode(
                        ::core::hint::black_box(#const_name.as_bytes()),
                        #ident.as_mut_byte_slice(),
                        ::obfstr2::encoding::hb2::DEFAULT_TABLE,
                    );
                    ::core::hint::black_box(_hb2_written);
                }
            },
            support: |size| (8..=8192).contains(&size),
            latency: 85,
        },
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
            latency: 80,
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
            latency: 68,
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
            latency: 71,
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
                    let mut #ident = unsafe { #family::<#size>::new_zeroed(#size).unwrap_unchecked() };
                    {
                        let mut _written = 0usize;

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

                            // 直接解码到目标容器的字节视图：省掉 scratch 容器与其拷贝 / 擦除。
                            let (_, _n) = ::obfstr2::encoding::base64::base64_decode(
                                ::core::hint::black_box(_line),
                                &mut #ident.as_mut_byte_slice()[_written.._written + _take],
                            );
                            _written += _n;
                            if _written >= #size { break 'pem_outer; }
                        }
                    }
                }
            },
            support: |size| (128..=8192).contains(&size),
            latency: 66,
        },
    ]
});
