// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 多态混淆加解密原语注册表。
//!
//! 威胁模型：并非传统的黑盒模型，而是白盒/终端攻击者模型。
//! ChaCha20是经受广泛验证的密码学安全算法，但在字符串混淆中，如果逆向人员在 IDA F5 中看到了常数 "expand 32-byte k" 或识别出了标准的 ARX 四分之一轮特征，他甚至不需要理解内部细节，只需从汇编里提取出 key 和 nonce，在 IDAPython 里调用 ChaCha20.new(key, nonce).decrypt(ct)，30 秒就能批量解出全程序所有字符串。
//! 相反，面对混合了浮点运算和动态盒变异的 mse，没有现成库可用，符号执行又会卡死，逆向人员只能痛苦地逐行抠伪代码或搭建 Unicorn 模拟执行环境。

use proc_macro2::{Ident as Ident2, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use std::sync::LazyLock;

/// 单层上下文：正反两趟共用同一种结构，`enc` 执行后 `out_size` 即唯一真相。
#[derive(Clone)]
pub(crate) struct LayerCtx {
    /// 容器家族路径（不含泛型，如 `::obfstr2::types::buffer::StackBytes`），
    pub(crate) type_path: TokenStream2,
    pub(crate) key: u64,
    pub(crate) key_ident: Ident2,
    pub(crate) iv: u8,
    /// 本层输入长度（enc 前 / dec 后应恢复到此长度）。
    pub(crate) in_size: usize,
    /// 本层输出长度（enc 后 / dec 前），由 `enc` 自报告写回。
    pub(crate) out_size: usize,
    /// 确定性临时标识后缀（`{macro_id}_{chunk_idx}_{enc_layer_idx}`），
    pub(crate) ident_tag: String,
}

#[allow(unused)]
#[derive(Clone, Copy)]
pub(crate) struct CryptoMetrics {
    /// [结构对抗] 最佳批量提取攻击难度 / 状态耦合度 (0~100)
    /// 衡量攻击者进行“单字节独立爆破/查表”或“静态常量特征扫描”的难度
    pub(crate) attack_complexity: u8,

    /// [人工对抗] 逆向工程师认知与脚本还原难度 (0~100)
    /// 衡量 IDA/Ghidra F5 反编译伪代码的恶心程度，以及人工用 Python 重写逆算法的耗时
    pub(crate) cognitive_load: u8,
}

#[derive(Clone)]
pub(crate) struct Crypto {
    #[allow(dead_code)]
    pub(crate) name: &'static str,
    /// 正向加密（拥有制）：接管输入容器，返回输出容器，并写回 `ctx.out_size`。
    pub(crate) enc: fn(&mut LayerCtx, Vec<u8>) -> Vec<u8>,
    /// 逆向解密：输入当前 ident，返回 `(解密 AST, 后续应使用的 ident)`。
    pub(crate) dec: fn(&LayerCtx, Ident2) -> (TokenStream2, Ident2),
    // 判断本层输入长度能否运用该操作（按当前 `data.len()` 逐层重估）。
    pub(crate) support: fn(usize) -> bool,
    /// 相对综合安全性
    pub(crate) security: u8,
    /// 相对综合开销
    pub(crate) latency: u8,
    /// 静态细分评估指标
    #[allow(dead_code)]
    pub(crate) metrics: CryptoMetrics,
}

pub(crate) static CRYPTOS: LazyLock<Vec<Crypto>> = LazyLock::new(|| {
    vec![
        // CBC 模式异或链
        Crypto {
            enc: |ctx, mut chunk| {
                let (key, iv) = (ctx.key, ctx.iv);
                let mut l = iv;
                for c in chunk.iter_mut() {
                    *c ^= key.rotate_left(l as u32) as u8;
                    l = *c;
                }
                ctx.out_size = chunk.len();
                chunk
            },
            dec: |ctx, in_ident| {
                let (key, iv) = (ctx.key_ident.clone(), ctx.iv);
                let ast = quote! {
                    {
                        let key = ::core::hint::black_box(#key as u64);
                        let mut l = ::core::hint::black_box(#iv as u8);
                        for c in #in_ident.iter_mut() {
                            let next_l = *c;
                            let v = *c ^ key.rotate_left(l as u32) as u8;
                            *c = ::core::hint::black_box(v);
                            l = ::core::hint::black_box(next_l);
                        }
                    }
                };
                (ast, in_ident)
            },
            name: "cbc_xor",
            support: |_len| true,
            security: 3,
            latency: 2,
            metrics: CryptoMetrics {
                attack_complexity: 22,
                cognitive_load: 12,
            },
        },
        // XOR 差分：结合位置密钥异或与前向密文差分
        Crypto {
            enc: |ctx, mut chunk| {
                let (key, iv) = (ctx.key, ctx.iv);
                let mut prev = iv;
                for (i, c) in chunk.iter_mut().enumerate() {
                    let k = lib_unknown::crypto::core::mix::mix64(key ^ i as u64) as u8;
                    let ct = (*c ^ k).wrapping_sub(prev);
                    *c = ct;
                    prev = ct;
                }
                ctx.out_size = chunk.len();
                chunk
            },
            dec: |ctx, in_ident| {
                let (key, iv) = (ctx.key_ident.clone(), ctx.iv);
                let ast = quote! {
                    {
                        let key = ::core::hint::black_box(#key as u64);
                        let mut prev = ::core::hint::black_box(#iv as u8);
                        for (i, c) in #in_ident.iter_mut().enumerate() {
                            let next_prev = *c;
                            let k = ::core::hint::black_box(
                                ::obfstr2::crypto::core::mix::mix64(key ^ i as u64) as u8
                            );
                            let v = (*c).wrapping_add(prev) ^ k;
                            *c = ::core::hint::black_box(v);
                            prev = ::core::hint::black_box(next_prev);
                        }
                    }
                };
                (ast, in_ident)
            },
            name: "xor_diff",
            support: |_len| true,
            security: 11,
            latency: 9,
            // mix64 位置密钥流 + 前向密文模加差分，打破了纯 GF(2) 线性关系且具上下文耦合
            metrics: CryptoMetrics {
                attack_complexity: 48,
                cognitive_load: 28,
            },
        },
        // 包装加减法与位置密钥
        Crypto {
            enc: |ctx, mut chunk| {
                let (key, iv) = (ctx.key, ctx.iv);
                for (i, c) in chunk.iter_mut().enumerate() {
                    let k =
                        lib_unknown::crypto::core::mix::mix64((key ^ iv as u64) ^ i as u64) as u8;
                    *c = c.wrapping_add(k);
                }
                ctx.out_size = chunk.len();
                chunk
            },
            dec: |ctx, in_ident| {
                let (key, iv) = (ctx.key_ident.clone(), ctx.iv);
                let ast = quote! {
                    {
                        let key = ::core::hint::black_box(#key as u64);
                        let iv = ::core::hint::black_box(#iv as u64);
                        for (i, c) in #in_ident.iter_mut().enumerate() {
                            let k = ::core::hint::black_box(
                                ::obfstr2::crypto::core::mix::mix64((key ^ iv) ^ i as u64) as u8
                            );
                            let v = c.wrapping_sub(k);
                            *c = ::core::hint::black_box(v);
                        }
                    }
                };
                (ast, in_ident)
            },
            name: "wrapping_add",
            support: |_len| true,
            security: 7,
            latency: 9,
            // 标准位置流密码：每字节调一次 mix64，但字节间无状态反馈，F5 逻辑直观
            metrics: CryptoMetrics {
                attack_complexity: 35,
                cognitive_load: 18,
            },
        },
        // 乘法逆元混淆
        Crypto {
            enc: |ctx, mut chunk| {
                let key = ctx.key;
                for (i, c) in chunk.iter_mut().enumerate() {
                    let k = (lib_unknown::crypto::core::mix::mix64(key ^ i as u64) as u8) | 1;
                    *c = c.wrapping_mul(lib_unknown::crypto::core::inv_mul::inv_mul8(k));
                }
                ctx.out_size = chunk.len();
                chunk
            },
            dec: |ctx, in_ident| {
                let key = ctx.key_ident.clone();
                let ast = quote! {
                    {
                        let key = ::core::hint::black_box(#key as u64);
                        for (i, c) in #in_ident.iter_mut().enumerate() {
                            let k = ::core::hint::black_box(
                                (::obfstr2::crypto::core::mix::mix64(key ^ i as u64) as u8) | 1
                            );
                            let v = c.wrapping_mul(k);
                            *c = ::core::hint::black_box(v);
                        }
                    }
                };
                (ast, in_ident)
            },
            name: "inv_mul",
            support: |_len| true,
            security: 9,
            latency: 24,
            // 解密端仅需单次奇数乘法（免求逆），开销低
            metrics: CryptoMetrics {
                attack_complexity: 32,
                cognitive_load: 18,
            },
        },
        // 动态位移混淆 + 位置流异或
        Crypto {
            enc: |ctx, mut chunk| {
                let key = ctx.key;
                for (i, c) in chunk.iter_mut().enumerate() {
                    let m = lib_unknown::crypto::core::mix::mix64(key ^ i as u64);
                    let rot = (m % 8) as u32;
                    let x = (m >> 8) as u8;
                    *c = c.rotate_left(rot) ^ x;
                }
                ctx.out_size = chunk.len();
                chunk
            },
            dec: |ctx, in_ident| {
                let key = ctx.key_ident.clone();
                let ast = quote! {
                    {
                        let key = ::core::hint::black_box(#key as u64);
                        for (i, c) in #in_ident.iter_mut().enumerate() {
                            let m = ::core::hint::black_box(
                                ::obfstr2::crypto::core::mix::mix64(key ^ i as u64)
                            );
                            let rot = ::core::hint::black_box((m % 8) as u32);
                            let x = ::core::hint::black_box((m >> 8) as u8);
                            *c ^= x;
                            let v = c.rotate_right(rot);
                            *c = ::core::hint::black_box(v);
                        }
                    }
                };
                (ast, in_ident)
            },

            name: "rot_xor",
            support: |_len| true,
            security: 11,
            latency: 10,
            // 伪随机流同时驱动异或掩码与动态循环移位位数，破坏字节内比特对齐
            metrics: CryptoMetrics {
                attack_complexity: 44,
                cognitive_load: 30,
            },
        },
        // mse系列：MSE 是一种采用密文反馈（CFB-like）链接的自定义对称加密算法，结合动态置换盒（P-box）与动态替换盒（S-box）实现非线性混淆，并引入基于超越数（π、1/e）定点近似的非线性密钥派生函数。其工程实现高度融合了浮点跨域混淆，具备极强的代码防逆向与抗符号执行特性。
        Crypto {
            enc: |ctx, mut chunk| {
                let (key, iv) = (ctx.key, ctx.iv);
                let _ = lib_unknown::crypto::mse::mse_no_ps(&mut chunk, key, iv);
                ctx.out_size = chunk.len();
                chunk
            },
            dec: |ctx, in_ident| {
                let (key, iv) = (ctx.key_ident.clone(), ctx.iv);
                let ast = quote! {
                    {
                        let key = ::core::hint::black_box(#key as u64);
                        let iv = ::core::hint::black_box(#iv as u8);
                        let _ = ::obfstr2::crypto::mse::imse_no_ps(#in_ident.as_mut_slice(), key, iv);
                        for b in #in_ident.iter_mut() {
                            *b = ::core::hint::black_box(*b);
                        }
                    }
                };
                (ast, in_ident)
            },

            name: "mse_no_ps",
            support: |_len| true,
            security: 40,
            latency: 31,
            // 每字节含 2x f_rotate_u64 + u128 定点超越数大数除法 + 2x rp64 + f_rotate_u8 + 双重模乘 + 密文反馈
            metrics: CryptoMetrics {
                attack_complexity: 78,
                cognitive_load: 82,
            },
        },
        Crypto {
            enc: |ctx, mut chunk| {
                let (key, iv) = (ctx.key, ctx.iv);
                let _ = lib_unknown::crypto::mse::mse_no_s(&mut chunk, key, iv);
                ctx.out_size = chunk.len();
                chunk
            },
            dec: |ctx, in_ident| {
                let (key, iv) = (ctx.key_ident.clone(), ctx.iv);
                let ast = quote! {
                    {
                        let key = ::core::hint::black_box(#key as u64);
                        let iv = ::core::hint::black_box(#iv as u8);
                        let _ = ::obfstr2::crypto::mse::imse_no_s(#in_ident.as_mut_slice(), key, iv);
                        for b in #in_ident.iter_mut() {
                            *b = ::core::hint::black_box(*b);
                        }
                    }
                };
                (ast, in_ident)
            },
            name: "mse_no_s",
            support: |_len| true,
            security: 46,
            latency: 43,
            // 在 mse_no_ps 基础上叠加每字节动态变异的双向 P-Box (DYB<8>::swap_by_val)，比特路由随历史状态演变
            metrics: CryptoMetrics {
                attack_complexity: 86,
                cognitive_load: 88,
            },
        },
        Crypto {
            enc: |ctx, mut chunk| {
                let (key, iv) = (ctx.key, ctx.iv);
                let _ = lib_unknown::crypto::mse::mse(&mut chunk, key, iv);
                ctx.out_size = chunk.len();
                chunk
            },
            dec: |ctx, in_ident| {
                let (key, iv) = (ctx.key_ident.clone(), ctx.iv);
                let ast = quote! {
                    {
                        let key = ::core::hint::black_box(#key as u64);
                        let iv = ::core::hint::black_box(#iv as u8);
                        let _ = ::obfstr2::crypto::mse::imse(#in_ident.as_mut_slice(), key, iv);
                        for b in #in_ident.iter_mut() {
                            *b = ::core::hint::black_box(*b);
                        }
                    }
                };
                (ast, in_ident)
            },
            name: "mse",
            support: |_len| true,
            security: 57,
            latency: 67,
            // 完整形态：含 gen_s 初始化分摊 + 双向动态变异 S-Box/P-Box + 128位超越数派生 + 浮点位运算
            metrics: CryptoMetrics {
                attack_complexity: 96,
                cognitive_load: 95,
            },
        },
        // MSE2：64 字节分组、4 轮 ARX + 动态 S/P 盒、PKCS#7 填充的变长分组密码。
        Crypto {
            enc: |ctx, chunk| {
                let mut dest = vec![0u8; chunk.len() + 64];
                let Ok(padded_len) =
                    lib_unknown::crypto::mse2::mse2(&chunk, &mut dest, ctx.key, ctx.iv as u64)
                else {
                    unreachable!("mse2 encryption failed")
                };
                dest.truncate(padded_len);
                ctx.out_size = padded_len;
                dest
            },
            dec: |ctx, in_ident| {
                let (key, iv) = (ctx.key_ident.clone(), ctx.iv);
                let path = ctx.type_path.clone();
                let pt = ctx.in_size;
                let out_ident = format_ident!("__tmp_{}", ctx.ident_tag);
                let ast = quote! {
                    // 只需设置长度：imse2 通过 `as_mut_slice()` 判断输出容量，随后会整体覆写。
                    // 使用 new_zeroed 避免仅为了预填长度而整块拷贝输入。
                    let mut #out_ident = unsafe { #path::<#pt>::new_zeroed(#pt).unwrap_unchecked() };

                    let key = ::core::hint::black_box(#key as u64);
                    let iv = ::core::hint::black_box(#iv as u64);

                    let _ = ::obfstr2::crypto::mse2::imse2(
                        #in_ident.as_slice(),
                        #out_ident.as_mut_slice(),
                        key,
                        iv,
                    );

                    drop(#in_ident);
                };
                (ast, out_ident)
            },
            name: "mse2",
            support: |len| len > 0,
            security: 60,
            latency: 93,
            // 64B 分组 + 4 轮 8-word ARX/动态盒；初始化摊销高，但分组级扩散与密钥扩展显著强于逐字节 mse
            metrics: CryptoMetrics {
                attack_complexity: 95,
                cognitive_load: 95,
            },
        },
        // 静态 S-Box 非线性替换
        Crypto {
            enc: |ctx, mut chunk| {
                for c in chunk.iter_mut() {
                    *c = lib_unknown::crypto::core::boxs::s8(
                        *c,
                        &lib_unknown::crypto::core::boxs::S1,
                    );
                }
                ctx.out_size = chunk.len();
                chunk
            },
            dec: |_ctx, in_ident| {
                let ast = quote! {
                    {
                        for c in #in_ident.iter_mut() {
                            let v = ::obfstr2::crypto::core::boxs::s8(*c, &::obfstr2::crypto::core::boxs::S2);
                            *c = ::core::hint::black_box(v);
                        }
                    }
                };
                (ast, in_ident)
            },
            name: "sbox_static",
            support: |_len| true,
            security: 1,
            latency: 2,
            // 单纯查表开销极低；虽有 dice0 多表切换与段名伪装，但二进制内 S2 表固定且无密钥参与，易被整表提取
            metrics: CryptoMetrics {
                attack_complexity: 8,
                cognitive_load: 13,
            },
        },
        // 动态比特洗牌 P-Box
        Crypto {
            enc: |ctx, mut chunk| {
                let key = ctx.key;
                let p = lib_unknown::crypto::core::boxs::gen_p(key);
                for c in chunk.iter_mut() {
                    *c = lib_unknown::crypto::core::boxs::p8(*c, &p);
                }
                ctx.out_size = chunk.len();
                chunk
            },
            dec: |ctx, in_ident| {
                let key = ctx.key_ident.clone();
                let ast = quote! {
                    {
                        let key = ::core::hint::black_box(#key as u64);
                        let p = ::core::hint::black_box(::obfstr2::crypto::core::boxs::gen_p(key));
                        let p_inv = ::core::hint::black_box(unsafe {
                            ::obfstr2::crypto::core::boxs::inv_p(&p).unwrap_unchecked()
                        });
                        for c in #in_ident.iter_mut() {
                            let v = ::obfstr2::crypto::core::boxs::p8(*c, &p_inv);
                            *c = ::core::hint::black_box(v);
                        }
                    }
                };
                (ast, in_ident)
            },
            name: "pbox_dynamic",
            support: |_len| true,
            security: 20,
            latency: 16,
            // 含 gen_p + inv_p 初始化及每字节 8 次比特提取循环；同一块内各字节共用同一比特置换表
            metrics: CryptoMetrics {
                attack_complexity: 32,
                cognitive_load: 42,
            },
        },
        // 动态密码本 S-Box
        Crypto {
            enc: |ctx, mut chunk| {
                let key = ctx.key;
                let s = lib_unknown::crypto::core::boxs::gen_s(key);
                for c in chunk.iter_mut() {
                    *c = lib_unknown::crypto::core::boxs::s8(*c, &s);
                }
                ctx.out_size = chunk.len();
                chunk
            },
            dec: |ctx, in_ident| {
                let key = ctx.key_ident.clone();
                let ast = quote! {
                    {
                        let key = ::core::hint::black_box(#key as u64);
                        let s = ::core::hint::black_box(::obfstr2::crypto::core::boxs::gen_s(key));
                        let s_inv = ::core::hint::black_box(unsafe {
                            ::obfstr2::crypto::core::boxs::inv_s(&s).unwrap_unchecked()
                        });
                        for c in #in_ident.iter_mut() {
                            let v = ::obfstr2::crypto::core::boxs::s8(*c, &s_inv);
                            *c = ::core::hint::black_box(v);
                        }
                    }
                };
                (ast, in_ident)
            },
            name: "sbox_dynamic",
            support: |_len| true,
            security: 38,
            latency: 39,
            // 循环查表仅 3 周期，但 gen_s (256次洗牌) + inv_s 初始化开销达 ~3800 周期，短字符串分摊 CPB 较高；消除了静态表特征
            metrics: CryptoMetrics {
                attack_complexity: 46,
                cognitive_load: 48,
            },
        },
        // PKCS7 填充到 16 字节边界。
        Crypto {
            enc: |ctx, chunk| {
                let mut dest = vec![0u8; chunk.len() + 16];
                let Ok(padded_len) =
                    lib_unknown::crypto::core::padding::pkcs7_pad(&chunk, &mut dest, 16)
                else {
                    unreachable!("pkcs7_pad failed")
                };
                dest.truncate(padded_len);
                ctx.out_size = padded_len; // 长度改变需要同步
                dest
            },
            dec: |ctx, in_ident| {
                let path = ctx.type_path.clone();
                let pt = ctx.in_size;
                let tag = ctx.ident_tag.clone();
                let out_ident = format_ident!("__tmp_{tag}"); // 长度改变需要更新容器
                let ast = quote! {
                    // 只需设置长度：pkcs7_unpad 通过 `as_mut_slice()` 判断输出容量，随后会整体覆写。
                    let mut #out_ident = unsafe { #path::<#pt>::new_zeroed(#pt).unwrap_unchecked() };

                    let align = ::core::hint::black_box(16usize);
                    let _ = ::core::hint::black_box(
                        ::obfstr2::crypto::core::padding::pkcs7_unpad(
                            #in_ident.as_slice(),
                            #out_ident.as_mut_slice(),
                            align,
                        )
                        .ok(),
                    );

                    drop(#in_ident); // 立即释放并触发 Drop 清零擦除
                };
                (ast, out_ident)
            },
            name: "pkcs7_16",
            support: |_len| true,
            security: 3,
            latency: 5,
            // 隐藏明文真实长度，运行期引入常数时间位掩码去填充校验循环
            metrics: CryptoMetrics {
                attack_complexity: 10,
                cognitive_load: 15,
            },
        },
        // 基于 Key 的 Fisher-Yates 字节位置置换（Transposition）
        Crypto {
            enc: |ctx, chunk| {
                let (key, iv) = (ctx.key, ctx.iv);
                let s = ((iv as u128) << 64) | (key as u128);
                let chunk = lib_unknown::crypto::core::shuffle_with(chunk, s);
                ctx.out_size = chunk.len();
                chunk
            },
            dec: |ctx, in_ident| {
                let (key, iv) = (ctx.key_ident.clone(), ctx.iv);
                let ast = quote! {
                    {
                        let key = ::core::hint::black_box(#key as u64);
                        let iv = ::core::hint::black_box(#iv as u8);
                        let s = ::core::hint::black_box(((iv as u128) << 64) | (key as u128));
                        ::obfstr2::crypto::core::unshuffle_with(#in_ident.as_mut_slice(), s);

                    }
                };
                (ast, in_ident)
            },
            name: "fisher_yates",
            support: |_len| true,
            security: 30,
            latency: 23,
            // 唯一具备跨字节全局位置扩散能力的原语；底层采用 rp128 + Lemire 128位无偏乘法区间映射 + 拒绝采样循环
            metrics: CryptoMetrics {
                attack_complexity: 68,
                cognitive_load: 64,
            },
        },
        // 浮点化旋转+异或：把整数 rotate/xor 展开为逐位浮点除法/截断/乘加，
        Crypto {
            enc: |ctx, mut chunk| {
                let key = ctx.key;
                for (i, c) in chunk.iter_mut().enumerate() {
                    let m = lib_unknown::crypto::core::mix::mix64(key ^ i as u64);
                    let rot = (m % 8) as u32;
                    let x = (m >> 8) as u8;
                    let v = lib_unknown::crypto::core::rotate::f_rotate_left_u8(*c, rot);
                    *c = lib_unknown::crypto::core::xor::f_xor_u8(v, x);
                }
                ctx.out_size = chunk.len();
                chunk
            },
            dec: |ctx, in_ident| {
                let key = ctx.key_ident.clone();
                let ast = quote! {
                    {
                        let key = ::core::hint::black_box(#key as u64);
                        for (i, c) in #in_ident.iter_mut().enumerate() {
                            let m = ::core::hint::black_box(
                                ::obfstr2::crypto::core::mix::mix64(key ^ i as u64)
                            );
                            let rot = ::core::hint::black_box((m % 8) as u32);
                            let x = ::core::hint::black_box((m >> 8) as u8);
                            let v = ::obfstr2::crypto::core::xor::f_xor_u8(*c, x);
                            let w = ::obfstr2::crypto::core::rotate::f_rotate_right_u8(v, rot);
                            *c = ::core::hint::black_box(w);
                        }
                    }
                };
                (ast, in_ident)
            },
            name: "rot_xor_f",
            support: |_len| true,
            security: 39,
            latency: 34,
            // f_xor_u8 (约220周期) + f_rotate_right_u8 (约55周期) + mix64；F5 伪代码被浮点除法与指数构造严重污染
            metrics: CryptoMetrics {
                attack_complexity: 52,
                cognitive_load: 76,
            },
        },
        // 浮点化 XOR 差分链
        Crypto {
            enc: |ctx, mut chunk| {
                let (key, iv) = (ctx.key, ctx.iv);
                let mut prev = iv;
                for (i, c) in chunk.iter_mut().enumerate() {
                    let k = lib_unknown::crypto::core::mix::mix64(key ^ i as u64) as u8;
                    let xored = lib_unknown::crypto::core::xor::f_xor_u8(*c, k);
                    let ct = xored.wrapping_sub(prev);
                    *c = ct;
                    prev = ct;
                }
                ctx.out_size = chunk.len();
                chunk
            },
            dec: |ctx, in_ident| {
                let (key, iv) = (ctx.key_ident.clone(), ctx.iv);
                let ast = quote! {
                    {
                        let key = ::core::hint::black_box(#key as u64);
                        let mut prev = ::core::hint::black_box(#iv as u8);
                        for (i, c) in #in_ident.iter_mut().enumerate() {
                            let next_prev = *c;
                            let k = ::core::hint::black_box(
                                ::obfstr2::crypto::core::mix::mix64(key ^ i as u64) as u8
                            );
                            let xored = (*c).wrapping_add(prev);
                            let v = ::obfstr2::crypto::core::xor::f_xor_u8(xored, k);
                            *c = ::core::hint::black_box(v);
                            prev = ::core::hint::black_box(next_prev);
                        }
                    }
                };
                (ast, in_ident)
            },
            name: "xor_diff_f",
            support: |_len| true,
            security: 39,
            latency: 32,
            // 结合前向密文差分反馈与 f_xor_u8 浮点代数重构，兼具跨字节状态耦合与高反编译认知阻力
            metrics: CryptoMetrics {
                attack_complexity: 64,
                cognitive_load: 72,
            },
        },
        // ChaCha20：标准 20 轮 ARX 流密码。
        Crypto {
            enc: |ctx, mut chunk| {
                let _ = lib_unknown::crypto::chacha20::chacha20_1(&mut chunk, ctx.key, ctx.iv);
                ctx.out_size = chunk.len();
                chunk
            },
            dec: |ctx, in_ident| {
                let (key, iv) = (ctx.key_ident.clone(), ctx.iv);
                let ast = quote! {
                    {
                        let key = ::core::hint::black_box(#key);
                        let iv = ::core::hint::black_box(#iv);

                        let _ = ::obfstr2::crypto::chacha20::chacha20_1(
                            #in_ident.as_mut_slice(),
                            key,
                            iv,
                        );

                    }
                };
                (ast, in_ident)
            },
            name: "chacha20",
            support: |_len| true,
            security: 36,
            latency: 37,
            // 密码学强度满分，但白盒威胁模型下若 ARX 轮特征被识别.
            metrics: CryptoMetrics {
                attack_complexity: 62,
                cognitive_load: 50,
            },
        },
        // 2 轮代换-置换网络（SPN）：轮函数为 S盒替换 -> P盒置换 -> 轮密钥异或，
        Crypto {
            enc: |ctx, mut chunk| {
                let key = ctx.key;
                for round in 0..2u64 {
                    let s_raw = key ^ round ^ 0xA5A5_A5A5_A5A5_A5A5;
                    let p_raw = key ^ round ^ 0x5A5A_5A5A_5A5A_5A5A;
                    let rk_raw = key ^ round ^ 0xDEAD_BEEF_DEAD_BEEF;
                    let s = lib_unknown::crypto::core::boxs::gen_s(
                        // 主要开销出在此处（512+次双倍两倍于mix128的置换）
                        lib_unknown::crypto::core::mix::mix64(s_raw),
                    );
                    let p = lib_unknown::crypto::core::boxs::gen_p(
                        lib_unknown::crypto::core::mix::mix64(p_raw),
                    );
                    let rk = lib_unknown::crypto::core::mix::mix64(rk_raw) as u8;
                    for c in chunk.iter_mut() {
                        *c = lib_unknown::crypto::core::boxs::s8(*c, &s);
                    }
                    for c in chunk.iter_mut() {
                        *c = lib_unknown::crypto::core::boxs::p8(*c, &p);
                    }
                    for c in chunk.iter_mut() {
                        *c ^= rk;
                    }
                }
                ctx.out_size = chunk.len();
                chunk
            },
            dec: |ctx, in_ident| {
                let key = ctx.key_ident.clone();
                let ast = quote! {
                    {
                        let key = ::core::hint::black_box(#key as u64);
                        for round in ::core::hint::black_box((0..2u64).rev()) {
                            let s_raw = key ^ round ^ 0xA5A5_A5A5_A5A5_A5A5u64;
                            let p_raw = key ^ round ^ 0x5A5A_5A5A_5A5A_5A5Au64;
                            let rk_raw = key ^ round ^ 0xDEAD_BEEF_DEAD_BEEFu64;
                            let s = ::obfstr2::crypto::core::boxs::gen_s(
                                ::obfstr2::crypto::core::mix::mix64(s_raw),
                            );
                            let p = ::obfstr2::crypto::core::boxs::gen_p(
                                ::obfstr2::crypto::core::mix::mix64(p_raw),
                            );
                            let rk = ::core::hint::black_box(
                                ::obfstr2::crypto::core::mix::mix64(rk_raw) as u8
                            );
                            let s_inv = ::core::hint::black_box(unsafe {
                                ::obfstr2::crypto::core::boxs::inv_s(&s).unwrap_unchecked()
                            });
                            let p_inv = ::core::hint::black_box(unsafe {
                                ::obfstr2::crypto::core::boxs::inv_p(&p).unwrap_unchecked()
                            });
                            for c in #in_ident.iter_mut() {
                                *c ^= rk;
                            }
                            for c in #in_ident.iter_mut() {
                                let v = ::obfstr2::crypto::core::boxs::p8(*c, &p_inv);
                                *c = ::core::hint::black_box(v);
                            }
                            for c in #in_ident.iter_mut() {
                                let v = ::obfstr2::crypto::core::boxs::s8(*c, &s_inv);
                                *c = ::core::hint::black_box(v);
                            }
                        }
                    }
                };
                (ast, in_ident)
            },
            name: "spn_2r",
            support: |_len| true,
            security: 48,
            latency: 61,
            // 2轮完整的 gen_s + inv_s + gen_p + inv_p 动态盒生成与求逆（初始化约7900周期），短字符串分摊 CPB 较高
            metrics: CryptoMetrics {
                attack_complexity: 74,
                cognitive_load: 68,
            },
        },
    ]
});
