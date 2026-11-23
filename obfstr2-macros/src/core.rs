// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 混淆编排核心：分块 → 加密 → 存储 → 发射（`build_obfuscated_bytes`）。

use crate::crypto::CRYPTOS;
use crate::crypto::{Crypto, LayerCtx};
use crate::parse::KeyPair;
use crate::storage::{STORAGES, Storage};
use lib_unknown::crypto::core::inv_mul::inv_mul64;
use lib_unknown::hash::LightHasher;
use lib_unknown::rand::core::{random, random_bool, random_range, shuffle};
use proc_macro2::{Ident as Ident2, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use std::ops::RangeInclusive;

/// 纯栈内存预算上限（字节），控制主容器与 chunk 中间容器走栈 / 走堆的阈值。
///
/// 由 `budget-*` feature 配置（`budget-1k` / `budget-4k` / `budget-16k` / `budget-64k` /
/// `budget-256k` / `budget-512k` / `budget-1m` / `budget-4m` / `budget-8m`）：同时启用
/// 多个时取最大档；未指定时默认 4 KiB。
pub(crate) const STACK_BUDGET: usize = if cfg!(feature = "budget-8m") {
    8 * 1024 * 1024
} else if cfg!(feature = "budget-4m") {
    4 * 1024 * 1024
} else if cfg!(feature = "budget-1m") {
    1024 * 1024
} else if cfg!(feature = "budget-512k") {
    512 * 1024
} else if cfg!(feature = "budget-256k") {
    256 * 1024
} else if cfg!(feature = "budget-64k") {
    64 * 1024
} else if cfg!(feature = "budget-16k") {
    16 * 1024
} else if cfg!(feature = "budget-4k") {
    4 * 1024
} else if cfg!(feature = "budget-1k") {
    1024
} else {
    4 * 1024 // 默认值放在最后
};

const _: () = const {
    assert!(
        cfg!(feature = "force_heap") as u8 + cfg!(feature = "force_stack") as u8 <= 1,
        "不能够同时启用force_heap和force_stack特性"
    );
};

/// 主容器（`is_main = true`）或 chunk 中间容器（`false`）的堆 / 栈统一判定。
///
/// 主容器优先级：`force_heap (TierParams)` > `feature (force_heap / force_stack)` > 默认启发式。
pub(crate) fn use_heap(
    size: usize,
    is_main: bool,
    prefer_stack: bool,
    force_heap: Option<bool>,
) -> Result<bool, String> {
    let budget = STACK_BUDGET / if is_main { 1 } else { 2 };

    // 仅对主容器（is_main = true）解析强制堆栈策略
    let effective_force_heap: Option<bool> = if is_main {
        match force_heap {
            Some(v) => Some(v), // 1. TierParams (_user 档) 优先级最高
            None => {
                if cfg!(feature = "force_heap") {
                    Some(true)
                } else if cfg!(feature = "force_stack") {
                    Some(false)
                } else {
                    None
                }
            }
        }
    } else {
        None
    };

    // 若命中强制策略，直接裁决
    if let Some(force) = effective_force_heap {
        return if force {
            if !cfg!(feature = "alloc") {
                Err("主容器强制走堆（force_heap = true），但当前未启用 `alloc` 特性".to_string())
            } else {
                Ok(true) // 无视 ≤16B 恒栈规则与随机掷币，强制走堆
            }
        } else {
            // 强制走栈：仍受 STACK_BUDGET 保护以防栈溢出（如需超预算走栈可调大 budget-* 特性）
            if size >= budget {
                Err(format!(
                    "主容器需要 {size}B，超过栈预算 {budget}B（总上限 {STACK_BUDGET}B），无法强制走栈"
                ))
            } else {
                Ok(false)
            }
        };
    }

    // 3. 默认启发式逻辑
    if cfg!(feature = "alloc") {
        if size <= 16 {
            Ok(false)
        } else if size >= budget {
            Ok(true)
        } else if random_bool(if prefer_stack { 0.76 } else { 0.4 }) {
            Ok(false)
        } else {
            Ok(true)
        }
    } else if size >= budget {
        Err(format!(
            "容器需要 {size}B，超过栈预算 {budget}B（总上限 {STACK_BUDGET}B）且无堆可用"
        ))
    } else {
        Ok(false)
    }
}

/// 强度档位公共参数（延迟上限 / 组合安全度下限 / 逐层非内联包装概率 / 短块预填充 / 堆栈偏好 / 反反汇编注入参数）。
#[derive(Clone)]
pub(crate) struct TierParams {
    pub(crate) force_heap: Option<bool>,
    pub(crate) prefer_stack: bool,
    pub(crate) max_latency: u8,
    pub(crate) min_security: u8,
    /// 单层安全度缩放系数（默认 1.0）：
    /// - `< 1.0`：折减单层安全度贡献，迫使选层循环叠加更多加密层才达到 `min_security`；
    /// - `> 1.0`：放大单层安全度贡献，更快达到 `min_security`，减少加密层数。
    pub(crate) security_factor: f64,
    /// 包装概率 0..=100（`build_dec_layers` 按层抛硬币）。
    pub(crate) wrap_pct: u8,
    /// 垃圾块填充概率（0~100 百分比；按 chunk 掷骰；超出 100 视为恒插）。
    pub(crate) junk_pct: u8,
    pub(crate) cold_pct: u8,
    /// 短块强制预填充：初始块长 < 16B 时是否前置 `pkcs7_16` 填充（不计入 op_count 与组合安全度）。
    pub(crate) pad_short: bool,

    pub(crate) anti_disasm_pct: u8,
    pub(crate) anti_disasm_count_range: RangeInclusive<u8>,
    pub(crate) anti_disasm_garbage_range: RangeInclusive<u8>,
}

impl TierParams {
    /// 校验自定义档位（`*_user!` 配置块）的取值与组合约束，违规以字符串错误返回。
    ///
    /// - `min_security` 必须 `<= 99`：组合安全度按 `100 * (1 - Π(1 - s/100))` 乘法累积，
    ///   数学上永远无法达到 100，`>= 100` 会使选层循环永不退出（编译死循环）。
    /// - `max_latency` 必须 `>= 2`：原语注册表的最低单层延迟为 2（`cbc_xor` / `sbox_static`），
    ///   更小的值会使首层可用原语池为空（深层报 `no crypto for layer`）。
    /// - 四个百分比字段必须 `<= 100`：编排端对超限值有静默钳位语义（如 `junk_pct` 超过
    ///   100 视为恒插），对用户 API 显式报错优于静默钳位。
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.min_security >= 100 {
            return Err(format!(
                "min_security 不能 >= 100（组合安全度按乘法累积，数学上无法达到 100，会导致编译死循环），实际 {}",
                self.min_security
            ));
        }
        if !self.security_factor.is_finite() || self.security_factor <= 0.0 {
            return Err(format!(
                "security_factor 必须是 > 0.0 的有限浮点数（<= 0 会导致组合安全度无法增长而陷入编译死循环），实际 {}",
                self.security_factor
            ));
        }
        if self.max_latency < 2 {
            return Err(format!(
                "max_latency 不能 < 2（原语注册表最低单层延迟为 2，更小的值会导致无可用加密原语），实际 {}",
                self.max_latency
            ));
        }
        for (name, pct) in [
            ("wrap_pct", self.wrap_pct),
            ("junk_pct", self.junk_pct),
            ("cold_pct", self.cold_pct),
        ] {
            if pct > 100 {
                return Err(format!("{name} 必须在 0..=100 范围内，实际 {pct}"));
            }
        }
        if self.anti_disasm_pct > 100 {
            return Err(format!(
                "anti_disasm_pct 必须在 0..=100 范围内，实际 {}",
                self.anti_disasm_pct
            ));
        }
        Ok(())
    }
}

/// 低延迟档（`s1` / `b1` / `i1` / `fl1` / `cs1`）：栈偏好（`alloc` 下掷币 76% 栈），`no_std` 可用。
pub(crate) const TIER_LOW: TierParams = TierParams {
    force_heap: None,
    prefer_stack: true,
    max_latency: 30,
    min_security: 7,
    security_factor: 1.0,
    wrap_pct: 8,
    junk_pct: 2,
    cold_pct: 10,
    pad_short: false,
    anti_disasm_pct: 5,
    anti_disasm_count_range: 1..=1,
    anti_disasm_garbage_range: 1..=5,
};

/// 均衡档（`s2` / `b2` / `i2` / `fl2` / `cs2`）：堆栈随机（`alloc` 下 >16B 载荷 60% 走堆）。
pub(crate) const TIER_BALANCED: TierParams = TierParams {
    force_heap: None,
    prefer_stack: false,
    max_latency: 80,
    min_security: 63,
    security_factor: 1.0,
    wrap_pct: 50,
    junk_pct: 18,
    cold_pct: 20,
    pad_short: true,
    anti_disasm_pct: 40,
    anti_disasm_count_range: 1..=2,
    anti_disasm_garbage_range: 4..=10,
};

/// 高强度档（`s3` / `b3` / `i3` / `fl3` / `cs3`）：`alloc` 下主容器堆偏好（60% 走堆，≤16B 恒栈）。
pub(crate) const TIER_HIGH: TierParams = TierParams {
    force_heap: None,
    prefer_stack: false,
    max_latency: 100,
    min_security: 93,
    security_factor: 1.0,
    wrap_pct: 85,
    junk_pct: 35,
    cold_pct: 50,
    pad_short: true,
    anti_disasm_pct: 80,
    anti_disasm_count_range: 2..=3,
    anti_disasm_garbage_range: 8..=15,
};

fn available_crypto(chunk_size: usize, max_latency: u8) -> Vec<Crypto> {
    shuffle(
        CRYPTOS
            .iter()
            .filter(|op| (op.support)(chunk_size) && op.latency <= max_latency)
            .cloned()
            .collect(),
    )
}

fn available_storage(chunk_size: usize, max_latency: u8) -> Vec<Storage> {
    shuffle(
        STORAGES
            .iter()
            .filter(|s| (s.support)(chunk_size) && s.latency <= max_latency)
            .cloned()
            .collect(),
    )
}

/// 单层快照：`enc` 执行后立即冻结关键字段，供逆序 `dec` 使用。
pub(crate) struct LayerCtxRecord {
    pub(crate) crypto: Crypto,
    pub(crate) key: u64,
    pub(crate) iv: u8,
    pub(crate) pt_len: usize,
    pub(crate) ct_len: usize,
}

/// 执行单个原语的编译期正向变换并冻结层快照。
fn apply_single_layer(
    crypto: Crypto,
    data: Vec<u8>,
    key: u64,
    chunk_tag: &str,
    layer_idx: usize,
) -> Result<(Vec<u8>, LayerCtxRecord), String> {
    let in_len = data.len();
    let iv = random::<u8>();
    // enc 期间 type_path/ident_tag/key_ident 无意义（enc 从不读它们），填占位符即可；
    // dec 时由 build_dec_layers 重填家族与确定性 Ident。
    let mut ctx = LayerCtx {
        type_path: TokenStream2::new(),
        key,
        key_ident: format_ident!("__unused_key_enc"),
        iv,
        in_size: in_len,
        out_size: in_len,
        ident_tag: String::new(),
    };
    let data = (crypto.enc)(&mut ctx, data);
    if ctx.out_size != data.len() {
        return Err(format!(
            "{chunk_tag} 层#{layer_idx} 原语({})自报告长度 {}B 与实际输出 {}B 不一致",
            crypto.name,
            ctx.out_size,
            data.len()
        ));
    }
    let out_len = data.len();
    Ok((
        data,
        LayerCtxRecord {
            crypto,
            key,
            iv,
            pt_len: in_len,
            ct_len: out_len,
        },
    ))
}

/// 加密选层：随机 `1..3` 个基础层起步，组合安全度（`1 - Π(1 - security)`）
/// 达到 `tier.min_security` 即停；真 chunk 与垃圾块共用，保证过滤口径一致。
///
/// 整个 chunk 仅生成一个根密钥种子，第 `i (i >= 1)` 层在编译期通过
/// `key.wrapping_mul(inv_mul64((last_dec as u64) | 1)).wrapping_sub(ghost_state)` 逆向派生密钥，
/// 使运行期逆序解密时可通过 `key.wrapping_add(ghost_state).wrapping_mul((last_dec as u64) | 1)`
/// 正向还原上一层密钥。
///
/// 若 `tier.pad_short` 开启且初始长度 `< 16`，强制前置一层 `pkcs7_16`（不计入 `op_count` 与安全度）。
/// 返回 `(密文, 层快照)`；原语自报告长度与实际输出不一致时返回 `Err`。
/// `chunk_tag` 仅用于错误定位（如 `"chunk#3"` / `"junk#3"`）。
pub(crate) fn encrypt_layers(
    data: Vec<u8>,
    tier: &TierParams,
    ghost_state: u64,
    chunk_tag: &str,
) -> Result<(Vec<u8>, Vec<LayerCtxRecord>), String> {
    let mut data = data;
    let mut layers: Vec<LayerCtxRecord> = Vec::new();
    let mut key = random::<u64>();

    let mut push_layer = |crypto: Crypto,
                          data: Vec<u8>,
                          layers: &mut Vec<LayerCtxRecord>|
     -> Result<Vec<u8>, String> {
        if !layers.is_empty()
            && let Some(&last_dec) = data.last()
        {
            key = key
                .wrapping_mul(inv_mul64((last_dec as u64) | 1))
                .wrapping_sub(ghost_state);
        }
        let (next_data, record) = apply_single_layer(crypto, data, key, chunk_tag, layers.len())?;
        layers.push(record);
        Ok(next_data)
    };

    // 短块强制前置 pkcs7_16 填充：将不足 16B 的块补齐到 16B，不计入后续层数与安全度预算
    if tier.pad_short && data.len() < 16 {
        let pkcs7 = CRYPTOS
            .iter()
            .find(|op| op.name == "pkcs7_16" && (op.support)(data.len()))
            .cloned()
            .ok_or_else(|| format!("{chunk_tag} 缺少可用的 pkcs7_16 预填充原语"))?;
        data = push_layer(pkcs7, data, &mut layers)?;
    }

    let op_count = random_range(1..3usize);
    let target_layers = layers.len() + op_count;
    let mut combined_vulnerability = 1.0_f64;
    let mut current_combined_security = 0_u8;

    while layers.len() < target_layers || current_combined_security < tier.min_security {
        let mut pool = available_crypto(data.len(), tier.max_latency);
        let Some(crypto) = pool.pop() else {
            return Err(format!(
                "no crypto for layer {}, max_latency {}",
                layers.len(),
                tier.max_latency
            ));
        };
        let security = crypto.security;
        data = push_layer(crypto, data, &mut layers)?;

        // 单层安全度按档位系数缩放，并钳位到 [0.0, 100.0]（防止系数过大时 vulnerability 变为负数）
        let scaled_security = (security as f64 * tier.security_factor).clamp(0.0, 100.0);
        let vulnerability = (100.0 - scaled_security) / 100.0;
        combined_vulnerability *= vulnerability;
        current_combined_security = (100.0 * (1.0 - combined_vulnerability)) as u8;
    }
    Ok((data, layers))
}

pub(crate) fn effective_wrap_pct(base: u8, inline_count: u32) -> u8 {
    if base == 0 {
        return 0;
    }
    if inline_count >= 64 {
        return 100;
    }
    (base as f64 * (1.0 + base as f64 / 100.0).powi(inline_count as i32)).min(100.0) as u8
}

/// 为整个 chunk 的初始 8 字节密钥生成随机 Storage 存储与运行时解码/即时擦除 AST。
/// 编译期以 `sim_ghost` 异或掩码，运行期结合 `#ghost_ident` 还原，确保单层块也强依赖 `ghost_state`。
/// 返回语句定义 `let mut #key_ident: u64 = { ... };`。
fn gen_key_storage(
    key: u64,
    sim_ghost: u64,
    ghost_ident: &Ident2,
    key_ident: &Ident2,
    tag: &str,
    max_latency: u8,
) -> TokenStream2 {
    let key_bytes = (key ^ sim_ghost).to_le_bytes();
    // 8 字节天然支持 mac, uuid, ipv6, bytes
    let mut pool = available_storage(8, max_latency);
    let storage = pool.pop().unwrap_or_else(|| {
        STORAGES
            .iter()
            .find(|s| s.name == "bytes")
            .cloned()
            .unwrap_or_else(|| STORAGES[0].clone())
    });

    let key_buf_ident = format_ident!("__key_buf_{tag}");
    let stack_family = quote!(::obfstr2::types::buffer::StackBytes);
    let storage_ast = (storage.ast)(&key_buf_ident, &stack_family, &key_bytes);

    quote! {
        #[allow(unused_variables, unused_mut)]
        let mut #key_ident: u64 = {
            #storage_ast
            u64::from_le_bytes(unsafe { #key_buf_ident.as_slice().try_into().unwrap_unchecked() })
                ^ ::core::hint::black_box(#ghost_ident)
        };
    }
}

/// 生成反反汇编宏调用 AST
fn gen_anti_disasm_ast(tier: &TierParams) -> TokenStream2 {
    if !cfg!(feature = "anti-disassembly") || tier.anti_disasm_pct == 0 {
        return TokenStream2::new();
    }

    if random_range(0..100u8) >= tier.anti_disasm_pct {
        return TokenStream2::new();
    }

    let count = random_range(tier.anti_disasm_count_range.clone());
    let garbage = random_range(tier.anti_disasm_garbage_range.clone());

    quote! {
        ::obfstr2::anti_disassembly!(count = #count, garbage = #garbage);
    }
}

/// 逆序解密 fold：`(AST, Ident_out) = dec(ctx, Ident_in)` 串联。
/// 每个 chunk 仅声明一个初始密钥（经 `ghost_ident` 解掩码），每层解密后混入 `ghost_ident` 并通过
/// `.wrapping_add(ghost_state).wrapping_mul((last_dec as u64) | 1)` 推进。
#[allow(clippy::too_many_arguments)]
fn build_dec_layers(
    layers: &[LayerCtxRecord],
    family: &TokenStream2,
    entry: Ident2,
    ghost_ident: &Ident2,
    sim_ghost: u64,
    tag_prefix: &str,
    tier: &TierParams,
) -> (TokenStream2, TokenStream2, Ident2) {
    let mut prelude = TokenStream2::new();
    let mut body = TokenStream2::new();
    let mut inline_count = 0;
    let mut cur = entry;

    let Some(entry_layer) = layers.last() else {
        return (prelude, body, cur);
    };

    // 为整个 chunk 构建唯一的初始 key_ident 与存储加载 AST（对应加密链末层密钥）
    let key_ident = format_ident!("__key_{tag_prefix}");
    body.extend(gen_key_storage(
        entry_layer.key,
        sim_ghost,
        ghost_ident,
        &key_ident,
        tag_prefix,
        tier.max_latency,
    ));

    for (rev_pos, layer) in layers.iter().rev().enumerate() {
        let enc_idx = layers.len() - 1 - rev_pos;
        let tag = format!("{tag_prefix}_{enc_idx}");
        let ct = layer.ct_len;
        let pt = layer.pt_len;
        let anti_disasm = gen_anti_disasm_ast(tier);

        // 档位控制的非内联概率 + 单调累积：0=全内联，>=100=全包装。
        let wrap = random_range(0..100u8) < effective_wrap_pct(tier.wrap_pct, inline_count);
        if wrap {
            let in_ty = quote!(#family<#ct>);
            let out_ty = quote!(#family<#pt>);
            let param = format_ident!("enc");
            let key_ref = format_ident!("__key_ref");
            let ctx = LayerCtx {
                type_path: family.clone(),
                key: layer.key,
                key_ident: key_ident.clone(),
                iv: layer.iv,
                in_size: layer.pt_len,
                out_size: layer.ct_len,
                ident_tag: tag.clone(),
            };

            let (frag, next_ident) = (layer.crypto.dec)(&ctx, param.clone());
            let advance_key_ast = if enc_idx > 0 && pt > 0 {
                quote! {
                    let _last_dec = ::core::hint::black_box(
                        unsafe { *#next_ident.as_slice().last().unwrap_unchecked() }
                    );
                    *#key_ref = ::core::hint::black_box(
                        #key_ident
                            .wrapping_add(::core::hint::black_box(#ghost_ident))
                            .wrapping_mul((_last_dec as u64) | 1)
                    );
                }
            } else {
                quote!()
            };

            let cold_attr = if random_range(0..100u8) < tier.cold_pct {
                quote!(#[cold])
            } else {
                quote!()
            };

            // 包装壳子形态再抛一次硬币：具名函数 / 闭包+泛型蹦床，各占 50% 概率。
            let use_closure = random::<bool>();
            if use_closure {
                let trampoline_fn = format_ident!("__dec_trampoline_{tag}");
                let closure_ident = format_ident!("__dec_closure_{tag}");
                let captured_ident = format_ident!("__dec_captured_{tag}");
                let val = format_ident!("__v_{tag}");

                prelude.extend(quote! {
                    #cold_attr
                    #[inline(never)]
                    #[allow(unused_mut)]
                    fn #trampoline_fn(
                        f: &mut dyn ::core::ops::FnMut(&mut u64, u64) -> #out_ty,
                        k: &mut u64,
                        g: u64,
                    ) -> #out_ty {
                        #anti_disasm
                        ::core::hint::black_box((::core::hint::black_box(f))(
                            ::core::hint::black_box(k),
                            ::core::hint::black_box(g),
                        ))
                    }
                });
                body.extend(quote! {
                    #[allow(unused_mut)]
                    let mut #captured_ident: ::core::option::Option<#in_ty> =
                        ::core::option::Option::Some(#cur);
                    let mut #closure_ident = move |
                        #key_ref: &mut u64,
                        #[allow(unused_variables)] #ghost_ident: u64,
                    | -> #out_ty {
                        #[allow(unused_mut)]
                        let mut #param: #in_ty = unsafe {
                            #captured_ident.take().unwrap_unchecked()
                        };
                        #[allow(unused_variables)]
                        let #key_ident: u64 = *#key_ref;
                        #frag
                        #advance_key_ast
                        ::core::hint::black_box(#next_ident)
                    };
                    let mut #val = #trampoline_fn(
                        ::core::hint::black_box(&mut #closure_ident),
                        ::core::hint::black_box(&mut #key_ident),
                        ::core::hint::black_box(#ghost_ident),
                    );
                });
                cur = val;
            } else {
                let dec_fn = format_ident!("__dec_layer_{tag}");
                let val = format_ident!("__v_{tag}");

                prelude.extend(quote! {
                    #cold_attr
                    #[inline(never)]
                    #[allow(unused_mut)]
                    fn #dec_fn(
                        mut enc: #in_ty,
                        #key_ref: &mut u64,
                        #[allow(unused_variables)] #ghost_ident: u64,
                    ) -> #out_ty {
                        #anti_disasm
                        #[allow(unused_variables)]
                        let #key_ident: u64 = *#key_ref;
                        #frag
                        #advance_key_ast
                        ::core::hint::black_box(#next_ident)
                    }
                });
                body.extend(quote! {
                    #[allow(function_casts_as_integer)]
                    let _ = ::core::hint::black_box(#dec_fn) as usize;
                    let mut #val = #dec_fn(
                        #cur,
                        ::core::hint::black_box(&mut #key_ident),
                        ::core::hint::black_box(#ghost_ident),
                    );
                });
                cur = val;
            }
        } else {
            let ctx = LayerCtx {
                type_path: family.clone(),
                key: layer.key,
                key_ident: key_ident.clone(),
                iv: layer.iv,
                in_size: layer.pt_len,
                out_size: layer.ct_len,
                ident_tag: tag,
            };
            let (frag, next_ident) = (layer.crypto.dec)(&ctx, cur);
            let advance_key_ast = if enc_idx > 0 && pt > 0 {
                quote! {
                    let _last_dec = ::core::hint::black_box(
                        unsafe { *#next_ident.as_slice().last().unwrap_unchecked() }
                    );
                    #key_ident = ::core::hint::black_box(
                        #key_ident
                            .wrapping_add(::core::hint::black_box(#ghost_ident))
                            .wrapping_mul((_last_dec as u64) | 1)
                    );
                }
            } else {
                quote!()
            };
            body.extend(quote! {
                #anti_disasm
                #frag
                #advance_key_ast
            });
            cur = next_ident;
            inline_count += 1;
        }
    }
    (prelude, body, cur)
}

/// 生成数学不透明谓词。
fn gen_integrity_check(
    is_junk: bool,
    final_ident: &Ident2,
    ghost_ident: &Ident2,
    sim_ghost: u64,
) -> TokenStream2 {
    let seed: u64 = random::<u64>() | 1; // 保证为奇数，在 Z_{2^64} 上可逆
    let c_lhs: u64 = random();
    let diff: u64 = random::<u64>() | 1; // 非零奇数偏移，用于构造精确不等式
    let use_eq: bool = random();

    // 当 (is_junk && use_eq) 或 (!is_junk && !use_eq) 时，运行期两侧应相等
    let should_equal = is_junk == use_eq;
    let cmp_op = if use_eq { quote!(==) } else { quote!(!=) };

    let mode = random_range(0..4_u8);
    match mode {
        // Mode 0: 非线性 MBA 恒等式 + Z_{2^64} 乘法逆元对偶 (inv_mul64)
        // 恒等式: ((x ^ y) + 2*(x & y)) * k * k^{-1} == (x | y) + (x & y)
        0 => {
            let rot = random_range(5..59_u32);
            let k_inv = inv_mul64(seed);
            let ct_ctx = sim_ghost.rotate_left(rot).wrapping_mul(seed);
            let c_rhs = if should_equal {
                ct_ctx.wrapping_add(c_lhs)
            } else {
                ct_ctx.wrapping_add(c_lhs).wrapping_add(diff)
            };

            quote! {
                let _x = ::core::hint::black_box(#ghost_ident) ^ #c_lhs;
                let _y = (#final_ident.as_slice().as_ptr() as usize) as u64;
                let _a = (_x ^ _y)
                    .wrapping_add((_x & _y).wrapping_shl(1))
                    .wrapping_mul(#seed);
                let _b = (_x | _y).wrapping_add(_x & _y);
                let _ctx = ::core::hint::black_box(#ghost_ident)
                    .rotate_left(#rot)
                    .wrapping_mul(#seed);
                let _lhs = _a.wrapping_mul(#k_inv).wrapping_add(_ctx).wrapping_add(#c_lhs);
                let _rhs = _b.wrapping_add(#c_rhs);
                if ::core::hint::black_box(_lhs #cmp_op _rhs) {
                    #final_ident.wipe_data();
                }
            }
        }

        // Mode 1: 汉明重量容斥原理 (popcnt / count_ones) + 正交位拆分 (SMT 求解器毒药)
        // 恒等式 1: popcnt(x | y) + popcnt(x & y) == popcnt(x) + popcnt(y)
        // 恒等式 2: ((x & !y) + (x & y)) ^ seed == x ^ seed == ghost_ident
        1 => {
            let ct_x = sim_ghost ^ seed;
            let ct_ctx = (ct_x.count_ones() as u64)
                .wrapping_mul(seed)
                .wrapping_add(sim_ghost);
            let c_rhs = if should_equal {
                ct_ctx.wrapping_add(c_lhs)
            } else {
                ct_ctx.wrapping_add(c_lhs).wrapping_add(diff)
            };
            let odd_c = c_lhs | 1;

            quote! {
                let _x = ::core::hint::black_box(#ghost_ident) ^ #seed;
                let _y = ((#final_ident.as_slice().as_ptr() as usize) as u64).wrapping_mul(#seed);
                let _pop_a = ((_x | _y).count_ones().wrapping_add((_x & _y).count_ones())) as u64;
                let _pop_b = (_x.count_ones().wrapping_add(_y.count_ones())) as u64;
                let _re_ghost = (_x & !_y).wrapping_add(_x & _y) ^ #seed;
                let _ctx = (_x.count_ones() as u64)
                    .wrapping_mul(#seed)
                    .wrapping_add(_re_ghost);
                let _lhs = _pop_a.wrapping_mul(#odd_c).wrapping_add(_ctx).wrapping_add(#c_lhs);
                let _rhs = _pop_b.wrapping_mul(#odd_c).wrapping_add(#c_rhs);
                if ::core::hint::black_box(_lhs #cmp_op _rhs) {
                    #final_ident.wipe_data();
                }
            }
        }

        // Mode 2: 2-adic 环 Hensel 奇数幂不动点 + 异或奇偶校验 (Parity)
        // 性质 1: 任意奇数 v，v^2 mod 8 == 1（故 v^2 & !7 == v^2 - 1），且 v^4 mod 16 == 1
        // 性质 2: (popcnt(x ^ y) ^ popcnt(x) ^ popcnt(y)) & 1 == 0
        2 => {
            let ct_ctx = (sim_ghost.wrapping_mul(seed) ^ 1).wrapping_add(c_lhs);
            let c_rhs = if should_equal {
                ct_ctx
            } else {
                ct_ctx.wrapping_add(diff)
            };

            quote! {
                let _x = ::core::hint::black_box(#ghost_ident);
                let _y = (#final_ident.as_slice().as_ptr() as usize) as u64;
                let _odd = (_x ^ _y ^ #seed) | 1;
                let _sq = _odd.wrapping_mul(_odd);
                let _parity = (((_x ^ _y).count_ones()
                    ^ _x.count_ones()
                    ^ _y.count_ones()) & 1) as u64;
                let _unit = (_sq.wrapping_mul(_sq) & 15) ^ _parity;
                let _lhs = (_x.wrapping_mul(#seed) ^ _unit)
                    .wrapping_add(_sq & !7u64)
                    .wrapping_add(#c_lhs);
                let _rhs = _sq.wrapping_sub(1).wrapping_add(#c_rhs);
                if ::core::hint::black_box(_lhs #cmp_op _rhs) {
                    #final_ident.wipe_data();
                }
            }
        }

        // Mode 3: 德摩根对称差重组的双侧 MBA + 棘轮线性同余校验
        // 恒等式: (x | y) + (x & y) == ((x | y) ^ (x & y)) + 2*(x & y)
        _ => {
            let k_inv = inv_mul64(seed);
            let ct_ctx = sim_ghost.wrapping_add(c_lhs).wrapping_mul(seed);
            let c_rhs = if should_equal {
                ct_ctx
            } else {
                ct_ctx.wrapping_add(diff)
            };

            quote! {
                let _x = ::core::hint::black_box(#ghost_ident);
                let _y = ((#final_ident.as_slice().as_ptr() as usize) as u64) ^ #seed;
                let _mba_a = (_x | _y).wrapping_add(_x & _y);
                let _mba_b = ((_x | _y) ^ (_x & _y)).wrapping_add((_x & _y).wrapping_shl(1));
                let _ctx = _x.wrapping_add(#c_lhs).wrapping_mul(#seed);
                let _lhs = (_mba_a ^ #k_inv).wrapping_add(_ctx);
                let _rhs = (_mba_b ^ #k_inv).wrapping_add(#c_rhs);
                if ::core::hint::black_box(_lhs #cmp_op _rhs) {
                    #final_ident.wipe_data();
                }
            }
        }
    }
}

/// 单个折叠组的最小子节点数。
const FOLD_GROUP_MIN: usize = 4;
/// 单个折叠组的最大子节点数。
pub(crate) const FOLD_GROUP_MAX: usize = 12;

/// 一个待发射的块节点。
///
/// `stmt` 既可以在根作用域执行，也可以被搬进任意层级的折叠函数：
/// 两个作用域都提供同名的 `&mut MainTy` 与 `mut u64` ghost 变量，
/// 因此块语句本身无需任何改写。
pub(crate) struct ChunkNode {
    pub(crate) stmt: TokenStream2,
}

/// 把一组相邻块节点折叠为独立函数，返回只含一条调用的父级节点。
#[allow(clippy::too_many_arguments)]
fn seal_chunk_group(
    group: Vec<ChunkNode>,
    id: u64,
    depth: usize,
    idx: usize,
    main_ty: &TokenStream2,
    bytes: &Ident2,
    ghost: &Ident2,
    prelude: &mut TokenStream2,
    tier: &TierParams,
) -> ChunkNode {
    let fold_fn = format_ident!("__fold_{id}_{depth}_{idx}");
    let stmts = group.iter().map(|node| &node.stmt);
    let cold_attr = if random_range(0..100u8) < tier.cold_pct {
        quote!(#[cold])
    } else {
        quote!()
    };
    let anti_disasm = gen_anti_disasm_ast(tier);

    prelude.extend(quote! {
        #cold_attr
        #[inline(never)]
        #[allow(unused_mut)]
        fn #fold_fn(#bytes: &mut #main_ty, mut #ghost: u64) -> u64 {
            #anti_disasm
            #(#stmts)*
            ::core::hint::black_box(#ghost)
        }
    });

    ChunkNode {
        stmt: quote! {
            #[allow(function_casts_as_integer)]
            let _ = ::core::hint::black_box(#fold_fn) as usize;
            #ghost = #fold_fn(#bytes, ::core::hint::black_box(#ghost));
        },
    }
}

/// 自底向上把 O(M) 个块节点折叠成多级调用树，直到根节点数 <= [`FOLD_GROUP_MAX`]。
///
/// 每轮除至多一个尾部单节点晋升外，每个组至少合并 2 个节点，
/// 因此节点数严格下降，折叠必然终止；根函数与单个折叠函数都只承载
/// 有限个（<= [`FOLD_GROUP_MAX`]）块节点，不再随块数线性膨胀。
#[allow(clippy::too_many_arguments)]
pub(crate) fn fold_chunk_nodes(
    mut nodes: Vec<ChunkNode>,
    id: u64,
    main_ty: &TokenStream2,
    bytes: &Ident2,
    ghost: &Ident2,
    prelude: &mut TokenStream2,
    tier: &TierParams,
) -> Vec<ChunkNode> {
    let mut depth = 0usize;
    while nodes.len() > FOLD_GROUP_MAX {
        let mut next = Vec::with_capacity(nodes.len() / 2 + 1);
        let mut groups = nodes.into_iter();
        let mut idx = 0usize;

        loop {
            let cap = random_range(FOLD_GROUP_MIN..=FOLD_GROUP_MAX);
            let group: Vec<ChunkNode> = groups.by_ref().take(cap).collect();
            match group.len() {
                0 => break,
                1 => next.extend(group),
                _ => {
                    next.push(seal_chunk_group(
                        group, id, depth, idx, main_ty, bytes, ghost, prelude, tier,
                    ));
                    idx += 1;
                }
            }
        }

        nodes = next;
        depth += 1;
    }
    nodes
}

/// 字节混淆编排的产物：展开 Token 流 + 主容器实际选择的堆 / 栈族。
///
/// `main_is_heap` 供 `cs2!` 等零拷贝消费主容器的外层包装复用：判定只在
/// `build_obfuscated_bytes` 内做一次，外层容器族与内层主容器族严格一致，
/// 避免两侧独立掷币导致 `from_inner_with_nul` 收到不匹配的容器类型。
pub(crate) struct ObfuscatedBytes {
    pub(crate) tokens: TokenStream2,
    pub(crate) main_is_heap: bool,
}

/// 字节混淆编排入口。
///
/// # 动态密钥与错误密钥（有意设计）
///
/// `key_pair` 为 `Some` 时，编译期以 `build_key` 派生初始 `sim_ghost`，运行期以
/// `run_key` 派生 `ghost_state` 解掩码块密钥。若运行期 `run_key` 与编译期
/// `build_key` 不一致，解密链将输出错误字节，并可能触发完整性谓词的
/// `wipe_data()` / 容器长度假设，最终表现为未定义行为（UB）。
///
/// 该行为是刻意保留的对抗设计：模拟执行、符号执行与切片执行无法仅凭控制流
/// 判定密钥是否匹配，错误密钥下的不可预测状态会污染分析状态空间。调用方必须
/// 保证 `run_key` 与 `build_key` 在运行期逐字节一致。
pub(crate) fn build_obfuscated_bytes(
    mut input: Vec<u8>,
    tier: TierParams,
    key_pair: Option<KeyPair>,
) -> Result<ObfuscatedBytes, String> {
    let size = input.len();
    let id: u64 = random();
    let ghost_init: u64 = random();

    let mut work_units: Vec<(Vec<u8>, bool)> = Vec::new();
    let mut i = 1;
    while !input.is_empty() {
        let base: usize = (i as f64).powf(1.4).ceil() as usize;
        let take_size = random_range({ 2 * base }..{ 8 * base }).min(input.len());
        let real_data: Vec<u8> = input.drain(..take_size).collect();
        let pt_len = real_data.len();
        work_units.push((real_data, false));

        if random_range(0..100_u8) < tier.junk_pct {
            let fake_lo = (pt_len / 10).max(1);
            let fake_len = if fake_lo >= pt_len {
                pt_len
            } else {
                random_range(fake_lo..pt_len + 1)
            };
            let fake_data: Vec<u8> = (0..fake_len).map(|_| random::<u8>()).collect();
            work_units.push((fake_data, true));
        }
        i += 1;
    }

    let raw_bytes = format_ident!("__raw_bytes_{id}");
    let bytes = format_ident!("__bytes_{id}");
    let ghost_state = format_ident!("__ghost_state_{id}");

    let chunk_type_path = |peak: usize| -> Result<TokenStream2, String> {
        if use_heap(peak, false, tier.prefer_stack, None)? {
            Ok(quote!(::obfstr2::types::buffer::HeapBytes))
        } else {
            Ok(quote!(::obfstr2::types::buffer::StackBytes))
        }
    };

    let main_is_heap = use_heap(size, true, tier.prefer_stack, tier.force_heap)?;
    let main_type_path = if main_is_heap {
        quote!(::obfstr2::types::buffer::HeapBytes)
    } else {
        quote!(::obfstr2::types::buffer::StackBytes)
    };
    let main_ty = quote!(#main_type_path::<#size>);

    let mut chunk_prelude = TokenStream2::new();
    let mut chunk_nodes: Vec<ChunkNode> = Vec::new();
    let mut chunk_inline_count: u32 = 0;

    // 编译期镜像追踪运行期的 ghost_state 与主容器末尾（最多 16 字节）
    // 若配置了动态密钥，编译期将 build_key 的 LightHasher 哈希混入初始 sim_ghost
    let mut sim_ghost: u64 = match &key_pair {
        Some(kp) => ghost_init ^ LightHasher::hash(kp.build_key.as_slice()),
        None => ghost_init,
    };
    let mut sim_tail: Vec<u8> = Vec::with_capacity(32);

    for (seq, (chunk_data, is_junk)) in (1..).zip(work_units) {
        let pt_len = chunk_data.len();
        // 仅在真块时提取当前块末尾最多 16 字节，用于后续更新编译期滑动窗口
        let chunk_tail: Vec<u8> = if !is_junk {
            chunk_data[pt_len.saturating_sub(16)..].to_vec()
        } else {
            Vec::new()
        };
        let tag = format!("{id}_{seq}");
        let bytes_chunk = format_ident!("__bytes_chunk_{tag}");

        // 加密选层（传入当前块入口处的 sim_ghost；垃圾块若在极端资源下无法加密则静默忽略）
        let (ciphertext, layers) =
            match encrypt_layers(chunk_data, &tier, sim_ghost, &format!("chunk#{seq}")) {
                Ok(res) => res,
                Err(e) => {
                    if is_junk {
                        continue;
                    } else {
                        return Err(e);
                    }
                }
            };
        let ct_len = ciphertext.len();

        let peak = layers
            .iter()
            .flat_map(|l| [l.pt_len, l.ct_len])
            .max()
            .unwrap_or(pt_len)
            .max(pt_len)
            .max(1);

        let chunk_type_path = match chunk_type_path(peak) {
            Ok(path) => path,
            // 垃圾块超栈预算时静默跳过
            Err(_) if is_junk => continue,
            Err(e) => return Err(e),
        };

        let mut pool = available_storage(ct_len, tier.max_latency);
        let Some(storage_strategy) = pool.pop() else {
            if is_junk {
                continue;
            }
            return Err(format!(
                "chunk#{seq} 密文 {ct_len}B 在延迟上限 {} 下无可用存储策略",
                tier.max_latency
            ));
        };

        let storage_ast = (storage_strategy.ast)(&bytes_chunk, &chunk_type_path, &ciphertext);

        let (dec_prelude, dec_body, final_ident) = build_dec_layers(
            &layers,
            &chunk_type_path,
            bytes_chunk.clone(),
            &ghost_state,
            sim_ghost,
            &tag,
            &tier,
        );

        let integrity_check_ast =
            gen_integrity_check(is_junk, &final_ident, &ghost_state, sim_ghost);
        let wrap_chunk =
            random_range(0..100u8) < effective_wrap_pct(tier.wrap_pct, chunk_inline_count);
        let chunk_ret_ty = quote!(#chunk_type_path<#pt_len>);

        let node_stmt = if wrap_chunk {
            let cold_attr = if random_range(0..100u8) < tier.cold_pct {
                quote!(#[cold])
            } else {
                quote!()
            };

            let use_closure = random::<bool>();
            if use_closure {
                let trampoline_fn = format_ident!("__chunk_trampoline_{tag}");
                let closure_ident = format_ident!("__chunk_closure_{tag}");
                let val_chunk = format_ident!("__chunk_val_{tag}");

                chunk_prelude.extend(quote! {
                    #cold_attr
                    #[inline(never)]
                    fn #trampoline_fn(
                        f: &mut dyn ::core::ops::FnMut(u64) -> #chunk_ret_ty,
                        g: u64,
                    ) -> #chunk_ret_ty {
                        ::core::hint::black_box((::core::hint::black_box(f))(
                            ::core::hint::black_box(g)
                        ))
                    }
                });

                quote! {
                    let mut #closure_ident = move |#[allow(unused_variables)] #ghost_state: u64| -> #chunk_ret_ty {
                        #storage_ast
                        #dec_prelude
                        #dec_body
                        #integrity_check_ast
                        ::core::hint::black_box(#final_ident)
                    };
                    let mut #val_chunk = #trampoline_fn(
                        ::core::hint::black_box(&mut #closure_ident),
                        ::core::hint::black_box(#ghost_state),
                    );
                    unsafe { #bytes.extend_from_slice(#val_chunk.as_slice()).unwrap_unchecked(); }
                    // 仅 drop 即可：Buffer 的 Drop 已对整段 capacity 执行 volatile 清零。
                    drop(#val_chunk);
                }
            } else {
                let chunk_fn = format_ident!("__chunk_fn_{tag}");
                let val_chunk = format_ident!("__chunk_val_{tag}");

                chunk_prelude.extend(quote! {
                    #cold_attr
                    #[inline(never)]
                    fn #chunk_fn(#[allow(unused_variables)] #ghost_state: u64) -> #chunk_ret_ty {
                        #storage_ast
                        #dec_prelude
                        #dec_body
                        #integrity_check_ast
                        ::core::hint::black_box(#final_ident)
                    }
                });

                quote! {
                    #[allow(function_casts_as_integer)]
                    let _ = ::core::hint::black_box(#chunk_fn) as usize;
                    let mut #val_chunk = #chunk_fn(::core::hint::black_box(#ghost_state));
                    unsafe { #bytes.extend_from_slice(#val_chunk.as_slice()).unwrap_unchecked(); }
                    // 仅 drop 即可：Buffer 的 Drop 已对整段 capacity 执行 volatile 清零。
                    drop(#val_chunk);
                }
            }
        } else {
            let val_chunk = format_ident!("__chunk_val_{tag}");
            chunk_inline_count += 1;
            quote! {
                {
                    #storage_ast
                    #dec_prelude
                    #dec_body
                    #integrity_check_ast
                    let mut #val_chunk = #final_ident;
                    unsafe { #bytes.extend_from_slice(#val_chunk.as_slice()).unwrap_unchecked(); }
                    // 仅 drop 即可：Buffer 的 Drop 已对整段 capacity 执行 volatile 清零。
                    drop(#val_chunk);
                }
            }
        };

        let mut node_stmt = node_stmt;
        node_stmt.extend(quote! {
            let _slice = #bytes.as_slice();
            let _tail = &_slice[_slice.len().saturating_sub(16)..];
            #ghost_state ^= ::obfstr2::hash::LightHasher::hash(_tail);
        });
        chunk_nodes.push(ChunkNode { stmt: node_stmt });

        // 同步推进编译期模拟状态：
        // 真块将明文拼入主容器，滑动保留末尾最多 16 字节；
        // 垃圾块在运行期被 wipe_data 清空为 0 长度，主容器末尾切片保持不变。
        if !is_junk {
            sim_tail.extend_from_slice(&chunk_tail);
            if sim_tail.len() > 16 {
                let drain_len = sim_tail.len() - 16;
                sim_tail.drain(..drain_len);
            }
        }
        sim_ghost ^= LightHasher::hash(sim_tail.as_slice());
    }

    let roots = fold_chunk_nodes(
        chunk_nodes,
        id,
        &main_ty,
        &bytes,
        &ghost_state,
        &mut chunk_prelude,
        &tier,
    );
    let mut chunk_body = TokenStream2::new();
    for node in roots {
        chunk_body.extend(node.stmt);
    }

    // 构造运行期 ghost_state 初始化语句（含 env! 依赖注册与 run_key 哈希混入）
    let ghost_init_ast = match &key_pair {
        Some(kp) => {
            let env_dep_ast = kp.build_key_env.as_ref().map(|var| {
                quote! {
                    const _: usize = ::core::env!(#var).len();
                }
            });
            let run_key_ident = format_ident!("__run_key_{id}");
            let run_key_expr = &kp.run_key;
            quote! {
                #env_dep_ast
                let #run_key_ident: &[u8] = #run_key_expr;
                #[allow(unused_mut)]
                let mut #ghost_state: u64 = #ghost_init
                    ^ ::obfstr2::hash::LightHasher::hash(::core::hint::black_box(#run_key_ident));
            }
        }
        None => quote! {
            #[allow(unused_mut)]
            let mut #ghost_state: u64 = #ghost_init;
        },
    };

    let anti_disasm1 = gen_anti_disasm_ast(&tier);
    let anti_disasm2 = gen_anti_disasm_ast(&tier);

    Ok(ObfuscatedBytes {
        tokens: quote! {
            {
                #chunk_prelude
                #anti_disasm1

                let mut #raw_bytes = #main_ty::new();
                #ghost_init_ast
                #[allow(unused_variables)]
                let #bytes: &mut #main_ty = &mut #raw_bytes;
                #chunk_body

                #anti_disasm2
                ::core::hint::black_box(#ghost_state);
                #raw_bytes
            }
        },
        main_is_heap,
    })
}
