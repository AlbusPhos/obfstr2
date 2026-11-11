// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 混淆编排核心：分块 → 加密 → 存储 → 发射（`build_obfuscated_bytes`）。

use crate::crypto::CRYPTOS;
use crate::crypto::{Crypto, LayerCtx};
use crate::storage::{STORAGES, Storage};
use lib_unknown::rand::core::{random, random_range, shuffle};
use proc_macro2::{Ident as Ident2, TokenStream as TokenStream2};
use quote::{format_ident, quote};

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

/// 强度档位公共参数（延迟上限 / 组合安全度下限 / 原语叠加倍率 / 逐层非内联包装概率 / 短块预填充）。
#[derive(Clone, Copy)]
pub(crate) struct TierParams {
    pub(crate) max_latency: u8,
    pub(crate) min_security: u8,
    pub(crate) magnification: u8,
    /// 包装概率 0..=100（`build_dec_layers` 按层抛硬币）。
    pub(crate) wrap_pct: u8,
    /// 垃圾块填充概率（0~100 百分比；按 chunk 掷骰；超出 100 视为恒插）。
    pub(crate) junk_pct: u8,
    pub(crate) cold_pct: u8,
    /// 短块强制预填充：初始块长 < 16B 时是否前置 `pkcs7_16` 填充（不计入 op_count 与组合安全度）。
    pub(crate) pad_short: bool,
}

/// 低延迟档（`s1` / `b1` / `i1` / `fl1` / `cs1`）：纯栈，`no_std` 可用。
pub(crate) const TIER_LOW: TierParams = TierParams {
    max_latency: 30,
    min_security: 7,
    magnification: 1,
    wrap_pct: 8,
    junk_pct: 2,
    cold_pct: 10,
    pad_short: false,
};

/// 均衡档（`s2` / `b2` / `i2` / `fl2` / `cs2`）：堆栈随机。
pub(crate) const TIER_BALANCED: TierParams = TierParams {
    max_latency: 80,
    min_security: 63,
    magnification: 1,
    wrap_pct: 50,
    junk_pct: 18,
    cold_pct: 20,
    pad_short: true,
};

/// 高强度档（`s3` / `b3` / `i3` / `fl3` / `cs3`）：`alloc` 下主容器走堆。
pub(crate) const TIER_HIGH: TierParams = TierParams {
    max_latency: 100,
    min_security: 93,
    magnification: 4,
    wrap_pct: 85,
    junk_pct: 35,
    cold_pct: 50,
    pad_short: true,
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
    chunk_tag: &str,
    layer_idx: usize,
) -> Result<(Vec<u8>, LayerCtxRecord), String> {
    let in_len = data.len();
    let (key, iv) = (random::<u64>(), random::<u8>());
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

/// 加密选层：随机 `1..3 × magnification` 个起步，组合安全度
/// （`1 - Π(1 - security)`）达标即停；真 chunk 与垃圾块共用，保证过滤口径一致。
///
/// 若 `tier.pad_short` 开启且初始长度 `< 16`，强制前置一层 `pkcs7_16`（不计入 `op_count` 与安全度）。
/// 返回 `(密文, 层快照)`；原语自报告长度与实际输出不一致时返回 `Err`。
/// `chunk_tag` 仅用于错误定位（如 `"chunk#3"` / `"junk#3"`）。
fn encrypt_layers(
    data: Vec<u8>,
    tier: TierParams,
    chunk_tag: &str,
) -> Result<(Vec<u8>, Vec<LayerCtxRecord>), String> {
    let mut data = data;
    let mut layers: Vec<LayerCtxRecord> = Vec::new();

    // 短块强制前置 pkcs7_16 填充：将不足 16B 的块补齐到 16B，不计入后续层数与安全度预算
    if tier.pad_short && data.len() < 16 {
        let pkcs7 = CRYPTOS
            .iter()
            .find(|op| op.name == "pkcs7_16" && (op.support)(data.len()))
            .cloned()
            .ok_or_else(|| format!("{chunk_tag} 缺少可用的 pkcs7_16 预填充原语"))?;
        let (padded_data, record) = apply_single_layer(pkcs7, data, chunk_tag, layers.len())?;
        data = padded_data;
        layers.push(record);
    }

    let op_count = random_range(1..3_u8).wrapping_mul(tier.magnification.max(1)) as usize;
    // 目标总层数 = 已前置的预填充层数（0 或 1）+ 正式加密所需最少层数 op_count
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
        let (next_data, record) = apply_single_layer(crypto, data, chunk_tag, layers.len())?;
        data = next_data;

        let vulnerability = (100.0 - security as f64) / 100.0;
        combined_vulnerability *= vulnerability;
        current_combined_security = (100.0 * (1.0 - combined_vulnerability)) as u8;
        layers.push(record);
    }
    Ok((data, layers))
}

fn effective_wrap_pct(base: u8, inline_count: u32) -> u8 {
    if base == 0 {
        return 0;
    }
    if inline_count >= 64 {
        return 100;
    }
    (base as f64 * (1.0 + base as f64 / 100.0).powi(inline_count as i32)).min(100.0) as u8
}

/// 为单层 8 字节密钥生成随机 Storage 存储与运行时解码/即时擦除 AST。
/// 返回语句定义 `let #key_ident: u64 = { ... };`。
fn gen_key_storage(key: u64, key_ident: &Ident2, tag: &str, max_latency: u8) -> TokenStream2 {
    let key_bytes = key.to_le_bytes();
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
        let #key_ident: u64 = {
            #storage_ast
            u64::from_le_bytes(unsafe {#key_buf_ident.as_slice().try_into().unwrap_unchecked() })
        };
    }
}

/// 逆序解密 fold：`(AST, Ident_out) = dec(ctx, Ident_in)` 串联。
fn build_dec_layers(
    layers: &[LayerCtxRecord],
    family: &TokenStream2,
    entry: Ident2,
    tag_prefix: &str,
    wrap_pct: u8,
    cold_pct: u8,
    max_latency: u8,
) -> (TokenStream2, TokenStream2, Ident2) {
    let mut prelude = TokenStream2::new();
    let mut body = TokenStream2::new();
    let mut inline_count = 0;
    let mut cur = entry;

    for (rev_pos, layer) in layers.iter().rev().enumerate() {
        let enc_idx = layers.len() - 1 - rev_pos;
        let tag = format!("{tag_prefix}_{enc_idx}");
        let ct = layer.ct_len;
        let pt = layer.pt_len;

        // 为本层构建专有 key_ident 与存储加载 AST
        let key_ident = format_ident!("__key_{tag}");
        let key_storage_ast = gen_key_storage(layer.key, &key_ident, &tag, max_latency);

        // 档位控制的非内联概率 + 单调累积：0=全内联，>=100=全包装。
        let wrap = random_range(0..100u8) < effective_wrap_pct(wrap_pct, inline_count);
        if wrap {
            let in_ty = quote!(#family<#ct>);
            let out_ty = quote!(#family<#pt>);
            let param = format_ident!("enc");
            let ctx = LayerCtx {
                type_path: family.clone(),
                key: layer.key,
                key_ident: key_ident.clone(),
                iv: layer.iv,
                in_size: layer.pt_len,
                out_size: layer.ct_len,
                ident_tag: tag.clone(),
            };

            let cold_attr = if random_range(0..100u8) < cold_pct {
                quote!(#[cold])
            } else {
                quote!()
            };

            // 包装壳子形态再抛一次硬币：具名函数 / 闭包+泛型蹦床，各占 50% 概率。
            let use_closure = random::<bool>();
            if use_closure {
                let (frag, next_ident) = (layer.crypto.dec)(&ctx, param.clone());
                let trampoline_fn = format_ident!("__dec_trampoline_{tag}");
                let closure_ident = format_ident!("__dec_closure_{tag}");
                let captured_ident = format_ident!("__dec_captured_{tag}");
                let val = format_ident!("__v_{tag}");

                prelude.extend(quote! {
                    #cold_attr
                    #[inline(never)]
                    #[allow(unused_mut)]
                    fn #trampoline_fn(f: &mut dyn ::core::ops::FnMut() -> #out_ty) -> #out_ty {
                        ::core::hint::black_box((::core::hint::black_box(f))())
                    }
                });
                body.extend(quote! {
                    #[allow(unused_mut)]
                    let mut #captured_ident: ::core::option::Option<#in_ty> =
                        ::core::option::Option::Some(#cur);
                    let mut #closure_ident = move || -> #out_ty {
                        #[allow(unused_mut)]
                        let mut #param: #in_ty = unsafe {
                            #captured_ident.take().unwrap_unchecked()
                        };
                        #key_storage_ast
                        #frag
                        ::core::hint::black_box(#next_ident)
                    };
                    let mut #val = #trampoline_fn(::core::hint::black_box(&mut #closure_ident));
                });
                cur = val;
            } else {
                let (frag, next_ident) = (layer.crypto.dec)(&ctx, param);
                let dec_fn = format_ident!("__dec_layer_{tag}");
                let val = format_ident!("__v_{tag}");

                prelude.extend(quote! {
                    #cold_attr
                    #[inline(never)]
                    #[allow(unused_mut)]
                    fn #dec_fn(mut enc: #in_ty) -> #out_ty {
                        #key_storage_ast
                        #frag
                        ::core::hint::black_box(#next_ident)
                    }
                });
                body.extend(quote! {
                    #[allow(function_casts_as_integer)]
                    let _ = ::core::hint::black_box(#dec_fn) as usize;
                    let mut #val = #dec_fn(#cur);
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
            body.extend(quote! {
                #key_storage_ast
                #frag
            });
            cur = next_ident;
            inline_count += 1;
        }
    }
    (prelude, body, cur)
}

/// 生成兼具真实感且不可折叠的“完整性校验”数学不透明谓词。
fn gen_integrity_check(is_junk: bool, final_ident: &Ident2) -> TokenStream2 {
    let seed: u64 = random();
    let mode = random_range(0..4_u8);
    match mode {
        0 => {
            let comp = if is_junk { quote!(!= 2) } else { quote!(== 2) };
            quote! {
                let _chk_val = ::core::hint::black_box(#seed as u64);
                if ::core::hint::black_box((_chk_val.wrapping_mul(_chk_val) & 3) #comp) {
                    #final_ident.wipe_data();
                }
            }
        }
        1 => {
            let comp = if is_junk { quote!(== 1) } else { quote!(== 0) };
            quote! {
                let _chk_val = ::core::hint::black_box(#seed as u64);
                if ::core::hint::black_box(((_chk_val | 1) & 1) #comp) {
                    #final_ident.wipe_data();
                }
            }
        }
        2 => {
            let comp = if is_junk { quote!(== 0) } else { quote!(== 1) };
            quote! {
                let _chk_val = ::core::hint::black_box(#seed as u64);
                if ::core::hint::black_box((_chk_val.wrapping_mul(2) & 1) #comp) {
                    #final_ident.wipe_data();
                }
            }
        }
        _ => {
            let comp = if is_junk {
                quote!(!= _chk_val)
            } else {
                quote!(== _chk_val)
            };
            quote! {
                let _chk_val = ::core::hint::black_box(#seed as u64);
                if ::core::hint::black_box(_chk_val.wrapping_add(1) #comp) {
                    #final_ident.wipe_data();
                }
            }
        }
    }
}

/// 单个折叠组的最小子节点数。
const FOLD_GROUP_MIN: usize = 4;
/// 单个折叠组的最大子节点数。
const FOLD_GROUP_MAX: usize = 12;

/// 一个待发射的块节点。
///
/// `stmt` 既可以在根作用域执行，也可以被搬进任意层级的折叠函数：
/// 两个作用域都提供同名的 `&mut MainTy` 与 `mut u8` ghost 变量，
/// 因此块语句本身无需任何改写。
struct ChunkNode {
    stmt: TokenStream2,
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
    cold_pct: u8,
    prelude: &mut TokenStream2,
) -> ChunkNode {
    let fold_fn = format_ident!("__fold_{id}_{depth}_{idx}");
    let stmts = group.iter().map(|node| &node.stmt);
    let cold_attr = if random_range(0..100u8) < cold_pct {
        quote!(#[cold])
    } else {
        quote!()
    };

    prelude.extend(quote! {
        #cold_attr
        #[inline(never)]
        #[allow(unused_mut)]
        fn #fold_fn(#bytes: &mut #main_ty, mut #ghost: u8) -> u8 {
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
fn fold_chunk_nodes(
    mut nodes: Vec<ChunkNode>,
    id: u64,
    main_ty: &TokenStream2,
    bytes: &Ident2,
    ghost: &Ident2,
    cold_pct: u8,
    prelude: &mut TokenStream2,
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
                // 尾部单节点不套冗余函数壳，直接晋升到下一轮与邻居合并。
                1 => next.extend(group),
                _ => {
                    next.push(seal_chunk_group(
                        group, id, depth, idx, main_ty, bytes, ghost, cold_pct, prelude,
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

pub(crate) fn build_obfuscated_bytes(
    mut input: Vec<u8>,
    tier: TierParams,
    allow_heap: bool,
    stack_main_bytes: bool,
) -> Result<TokenStream2, String> {
    let size = input.len();
    let id: u64 = random();

    let mut work_units: Vec<(Vec<u8>, bool)> = Vec::new();
    let mut i = 1;
    while !input.is_empty() {
        let base = ((i as f64).powf(1.4)).ceil() as usize;
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

    let main_type_path = if allow_heap && (!stack_main_bytes || size >= STACK_BUDGET) {
        quote!(::obfstr2::types::buffer::HeapBytes)
    } else if size >= STACK_BUDGET && !allow_heap {
        return Err(format!(
            "主容器 需要 {size}B 超过纯栈上限 {STACK_BUDGET}B 且无堆可用"
        ));
    } else {
        quote!(::obfstr2::types::buffer::StackBytes)
    };
    let main_ty = quote!(#main_type_path::<#size>);

    let mut chunk_prelude = TokenStream2::new();
    let mut chunk_nodes: Vec<ChunkNode> = Vec::new();
    let mut chunk_inline_count: u32 = 0;

    for (seq, (chunk_data, is_junk)) in (1..).zip(work_units) {
        let pt_len = chunk_data.len();
        let tag = format!("{id}_{seq}");
        let bytes_chunk = format_ident!("__bytes_chunk_{tag}");

        // 加密选层（垃圾块若在极端资源下无法加密则静默忽略）
        let (ciphertext, layers) = match encrypt_layers(chunk_data, tier, &format!("chunk#{seq}")) {
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

        let use_heap_chunk = if peak > STACK_BUDGET / 2 {
            if allow_heap {
                true
            } else if is_junk {
                continue;
            } else {
                return Err(format!(
                    "chunk#{seq} 需要 {peak}B 中间容器，超过纯栈上限 {}B 且无堆可用",
                    STACK_BUDGET / 2
                ));
            }
        } else if peak <= 16 {
            false
        } else {
            allow_heap && random::<bool>()
        };

        let chunk_type_path = if use_heap_chunk {
            quote!(::obfstr2::types::buffer::HeapBytes)
        } else {
            quote!(::obfstr2::types::buffer::StackBytes)
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
            &tag,
            tier.wrap_pct,
            tier.cold_pct,
            tier.max_latency,
        );

        let integrity_check_ast = gen_integrity_check(is_junk, &final_ident);

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
                    fn #trampoline_fn(f: &mut dyn ::core::ops::FnMut() -> #chunk_ret_ty) -> #chunk_ret_ty {
                        ::core::hint::black_box((::core::hint::black_box(f))())
                    }
                });

                quote! {
                    let mut #closure_ident = move || -> #chunk_ret_ty {
                        #storage_ast
                        #dec_prelude
                        #dec_body
                        #integrity_check_ast
                        ::core::hint::black_box(#final_ident)
                    };
                    let mut #val_chunk = #trampoline_fn(::core::hint::black_box(&mut #closure_ident));
                    unsafe { #bytes.extend_from_slice(#val_chunk.as_slice()).unwrap_unchecked(); }
                    #val_chunk.wipe_data();
                    drop(#val_chunk);
                }
            } else {
                let chunk_fn = format_ident!("__chunk_fn_{tag}");
                let val_chunk = format_ident!("__chunk_val_{tag}");

                chunk_prelude.extend(quote! {
                    #cold_attr
                    #[inline(never)]
                    fn #chunk_fn() -> #chunk_ret_ty {
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
                    let mut #val_chunk = #chunk_fn();
                    unsafe { #bytes.extend_from_slice(#val_chunk.as_slice()).unwrap_unchecked(); }
                    #val_chunk.wipe_data();
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
                    #val_chunk.wipe_data();
                    drop(#val_chunk);
                }
            }
        };

        let mut node_stmt = node_stmt;
        node_stmt.extend(quote! {
            if let Some(&_last_byte) = #bytes.as_slice().last() {
                #ghost_state = #ghost_state.wrapping_add(_last_byte);
            }
        });
        chunk_nodes.push(ChunkNode { stmt: node_stmt });
    }

    let roots = fold_chunk_nodes(
        chunk_nodes,
        id,
        &main_ty,
        &bytes,
        &ghost_state,
        tier.cold_pct,
        &mut chunk_prelude,
    );
    let mut chunk_body = TokenStream2::new();
    for node in roots {
        chunk_body.extend(node.stmt);
    }

    Ok(quote! {
        {
            #chunk_prelude
            let mut #raw_bytes = #main_ty::new();
            let mut #ghost_state: u8 = 0_u8;
            {
                #[allow(unused_variables)]
                let #bytes: &mut #main_ty = &mut #raw_bytes;
                #chunk_body
            }
            ::core::hint::black_box(#ghost_state);
            #raw_bytes
        }
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    #[test]
    fn effective_wrap_pct_starts_at_base() {
        assert_eq!(effective_wrap_pct(15, 0), 15);
        assert_eq!(effective_wrap_pct(50, 0), 50);
        assert_eq!(effective_wrap_pct(85, 0), 85);
        assert_eq!(effective_wrap_pct(0, 999), 0);
    }

    /// 折叠树必须把根节点数收敛到 `FOLD_ROOT_MAX`，且小块输入不生成任何折叠壳。
    #[test]
    fn fold_chunk_nodes_bounds_root_spine() {
        let main_ty = quote!(u8);
        let bytes = format_ident!("__b");
        let ghost = format_ident!("__g");
        let make = |n: usize| {
            (0..n)
                .map(|i| ChunkNode {
                    stmt: quote!(let _ = #i as u8;),
                })
                .collect::<Vec<_>>()
        };

        let mut prelude = TokenStream2::new();
        let roots = fold_chunk_nodes(make(1), 1, &main_ty, &bytes, &ghost, 0, &mut prelude);
        assert_eq!(roots.len(), 1);
        assert!(
            !prelude.to_string().contains("__fold_"),
            "单块不应生成折叠壳"
        );

        let mut prelude = TokenStream2::new();
        let roots = fold_chunk_nodes(make(1000), 2, &main_ty, &bytes, &ghost, 0, &mut prelude);
        assert!(
            roots.len() <= FOLD_GROUP_MAX,
            "根节点数应收敛到 {FOLD_GROUP_MAX}"
        );
        assert!(
            prelude.to_string().contains("__fold_"),
            "大规模必须生成折叠函数"
        );
    }

    /// 批量模拟三档 Tier 的加密选层过程（`encrypt_layers`），用于离线评估配置合理性。
    #[test]
    fn simulate_crypto_selection_across_tiers() {
        use lib_unknown::rand::core::fill_bytes;
        use std::collections::{BTreeMap, HashSet};

        const ITERATIONS: usize = 100;
        const CHUNK_SIZES: [usize; 9] = [1, 2, 4, 8, 16, 32, 64, 256, 1024];

        struct TierCase {
            name: &'static str,
            tier: TierParams,
        }

        let cases = [
            TierCase {
                name: "LOW",
                tier: TIER_LOW,
            },
            TierCase {
                name: "BALANCED",
                tier: TIER_BALANCED,
            },
            TierCase {
                name: "HIGH",
                tier: TIER_HIGH,
            },
        ];

        for case in &cases {
            let tier = case.tier;
            println!(
                "\n{:=^100}",
                format!(
                    " Tier {} (max_latency={}, min_security={}, magnification={}, pad_short={}) ",
                    case.name,
                    tier.max_latency,
                    tier.min_security,
                    tier.magnification,
                    tier.pad_short
                )
            );

            // 原语使用计数（跨所有 chunk size 聚合）
            let mut usage: BTreeMap<&'static str, u64> = BTreeMap::new();
            let mut layer_counts: Vec<usize> = Vec::new();
            let mut securities: Vec<u8> = Vec::new();
            let mut latency_sums: Vec<u32> = Vec::new();
            let mut fail = 0u64;
            let mut total = 0u64;

            // 按 chunk size 细分的层数 / 安全度均值，用于观察规模效应（是否随长度漂移）
            let mut per_size_layers: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
            let mut per_size_security: BTreeMap<usize, Vec<u8>> = BTreeMap::new();

            for &size in &CHUNK_SIZES {
                for _ in 0..ITERATIONS {
                    total += 1;
                    let mut data = vec![0u8; size];
                    if size > 0 {
                        fill_bytes(&mut data);
                    }
                    match encrypt_layers(data, tier, "sim") {
                        Ok((_, layers)) => {
                            let pre_padded = tier.pad_short && size < 16;
                            let mut vuln = 1.0_f64;
                            let mut lat_sum: u32 = 0;
                            for (idx, l) in layers.iter().enumerate() {
                                *usage.entry(l.crypto.name).or_insert(0) += 1;
                                lat_sum += l.crypto.latency as u32;
                                // 跳过前置 pkcs7_16 预填充层的安全度贡献，与 encrypt_layers 口径一致
                                if !(pre_padded && idx == 0 && l.crypto.name == "pkcs7_16") {
                                    vuln *= (100.0 - l.crypto.security as f64) / 100.0;
                                }
                            }
                            let sec = (100.0 * (1.0 - vuln)) as u8;

                            layer_counts.push(layers.len());
                            securities.push(sec);
                            latency_sums.push(lat_sum);
                            per_size_layers.entry(size).or_default().push(layers.len());
                            per_size_security.entry(size).or_default().push(sec);
                        }
                        Err(_e) => {
                            fail += 1;
                        }
                    }
                }
            }

            let avg = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
            let avg_u = |v: &[usize]| avg(&v.iter().map(|&x| x as f64).collect::<Vec<_>>());
            let avg_u8 = |v: &[u8]| avg(&v.iter().map(|&x| x as f64).collect::<Vec<_>>());
            let avg_u32 = |v: &[u32]| avg(&v.iter().map(|&x| x as f64).collect::<Vec<_>>());

            println!(
                "调用次数: {total}, 失败次数: {fail} ({:.3}%)",
                fail as f64 / total as f64 * 100.0
            );
            println!(
                "层数: min={} avg={:.2} max={}",
                layer_counts.iter().copied().min().unwrap_or(0),
                avg_u(&layer_counts),
                layer_counts.iter().copied().max().unwrap_or(0),
            );
            let below_target = securities
                .iter()
                .filter(|&&s| s < tier.min_security)
                .count();
            println!(
                "组合安全度: min={} avg={:.2} max={}, 未达标占比={:.2}% (目标>={})",
                securities.iter().copied().min().unwrap_or(0),
                avg_u8(&securities),
                securities.iter().copied().max().unwrap_or(0),
                below_target as f64 / securities.len().max(1) as f64 * 100.0,
                tier.min_security,
            );
            println!(
                "单 chunk 层延迟之和估算: min={} avg={:.2} max={}",
                latency_sums.iter().copied().min().unwrap_or(0),
                avg_u32(&latency_sums),
                latency_sums.iter().copied().max().unwrap_or(0),
            );

            // 按 chunk size 展示规模效应（层数 / 安全度是否随 size 增长而漂移）
            println!(
                "\n{:<10} | {:<10} | {:<10}",
                "ChunkSize", "AvgLayers", "AvgSecurity"
            );
            println!("{:-<36}", "");
            for &size in &CHUNK_SIZES {
                let layers = per_size_layers.get(&size).cloned().unwrap_or_default();
                let secs = per_size_security.get(&size).cloned().unwrap_or_default();
                println!(
                    "{:<10} | {:<10.2} | {:<10.2}",
                    size,
                    avg_u(&layers),
                    avg_u8(&secs),
                );
            }

            // 原语使用频率分布
            let total_usage: u64 = usage.values().sum();
            let mut rows: Vec<(&str, u64)> = usage.iter().map(|(&k, &v)| (k, v)).collect();
            rows.sort_by_key(|&(_, count)| std::cmp::Reverse(count));

            println!(
                "\n{:<15} | {:<10} | {:<10} | {:<10} | {:<10}",
                "Crypto", "Count", "Freq%", "Security", "Latency"
            );
            println!("{:-<65}", "");
            for (name, count) in &rows {
                if let Some(crypto) = CRYPTOS.iter().find(|c| c.name == *name) {
                    println!(
                        "{:<15} | {:<10} | {:<9.2}% | {:<10} | {:<10}",
                        name,
                        count,
                        *count as f64 / total_usage.max(1) as f64 * 100.0,
                        crypto.security,
                        crypto.latency,
                    );
                }
            }

            let available: HashSet<&str> = CRYPTOS
                .iter()
                .filter(|c| c.latency <= tier.max_latency)
                .map(|c| c.name)
                .collect();
            let used: HashSet<&str> = rows.iter().map(|&(n, _)| n).collect();
            let never_used: Vec<&&str> = available.iter().filter(|n| !used.contains(*n)).collect();
            if !never_used.is_empty() {
                println!(
                    "\n⚠ 从未被选中的可用原语（latency<={}）: {:?}",
                    tier.max_latency, never_used
                );
            }

            assert_eq!(
                fail, 0,
                "tier {} 存在无法完成加密选层的 chunk 长度",
                case.name
            );
        }
    }
}
