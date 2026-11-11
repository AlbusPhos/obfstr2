# obfstr2

> **Polymorphic compile-time string/wide-str/wide-cstr/bytes/int/float/cstr/file obfuscation**

[![Crates.io](https://img.shields.io/crates/v/obfstr2?logo=rust)](https://crates.io/crates/obfstr2)
[![Docs.rs](https://img.shields.io/docsrs/obfstr2?logo=docs.rs)](https://docs.rs/obfstr2)
[![CI](https://img.shields.io/github/actions/workflow/status/AlbusPhos/obfstr2/ci.yml?logo=github&label=CI)](https://github.com/AlbusPhos/obfstr2/actions)
[![MSRV](https://img.shields.io/crates/msrv/obfstr2?logo=rust&label=MSRV)](https://crates.io/crates/obfstr2)
[![Downloads](https://img.shields.io/crates/d/obfstr2?color=blue)](https://crates.io/crates/obfstr2)
[![License](https://img.shields.io/crates/l/obfstr2)](https://mozilla.org/MPL/2.0/)

**语言：** [English](README_en.md) | 简体中文

一行宏即用的 Rust 编译期安全混淆框架：字面量输入，安全表达式输出，所有加密与数据形变在编译期阶段完成。每次编译均生成完全不同的二进制特征与密文形态，彻底挫败基于特征匹配的静态扫描与批量逆向脚本。

全面覆盖原生字符串、UTF-16 宽字符串、UTF-16 宽 C 字符串、字节数组、外部静态文件、标量整数、浮点数、C 风格字符串（CStr）以及格式化字符串九类载荷。支持解引用直取裸值或转入受控安全容器；字符串与字节类容器在离开作用域（`Drop`）时执行 Volatile 内存擦除，防止敏感明文残留在栈或堆内存中。

## 目录

- [快速开始](#快速开始-quick-start)
- [设计哲学](#设计哲学-design-philosophy)
- [宏一览](#宏一览)
- [适用场景 vs 不适用场景](#适用场景-vs-不适用场景)
- [特性标志](#特性标志-feature-flags)
- [平台与环境支持](#平台与环境支持)
- [性能](#性能-benchmarks)
- [安全性](#安全性-security)
- [贡献](#贡献-contributing)
- [变更日志](#变更日志-changelog)
- [开源协议](#开源协议-license)

## 快速开始 (Quick Start)

```toml
[dependencies]
obfstr2 = "0.3"
```

```rust
use obfstr2::{b2, cs2, cw2, f2, fl2, i2, s2, s_fmt2, w1};

fn main() {
    // 字符串：求值即得原文，返回自有容器，可绑定、传递、复用
    let hello = s2!("hello");
    print!("{hello}");
    // C 字符串：求值即得原文，解引用即 `CStr`，`as_ptr()` 可直投系统调用
    let sh = cs2!(c"/bin/sh");
    assert_eq!(&*sh, c"/bin/sh");
    // 宽 C 字符串：UTF-16 码元 + 自动结尾 0u16，as_ptr() 可直接用于 Win32 PCWSTR
    let w = cw2!("kernel32.dll");
    assert_eq!(w[w.len() - 1], 0);
    // 宽字符串：UTF-16 码元，不追加 NUL、允许内部 NUL
    let ws = w1!("a\0b");
    assert_eq!(&*ws, &[0x61u16, 0, 0x62][..]);
    // 字节串 / 字节数组：解引用即得原文 `[u8]`
    let b = b2!(b"abc");
    let c = b2!([0x61, 98, 99]);
    assert_eq!(&*b, &*c);
    // 整数：求值即得原文裸值，可直接算术、比较（空后缀视为 `i32`）
    let x = i2!(42u32);
    assert_eq!(x + 1, 43u32);
    // 浮点：求值即得原文裸值（空后缀视为 `f64`，`inf` / `NaN` 拒绝）
    let y = fl2!(3.15);
    assert_eq!(y.to_bits(), 3.15f64.to_bits());
    // 文件：路径相对被编译 crate 的 CARGO_MANIFEST_DIR，编译期读入
    let d = f2!("assets/fixture.bin");
    // 格式化字符串：字面量片段逐个混淆后走 `format!`，占位符照常使用，返回 `String`
    let name = "world";
    let greeting = s_fmt2!("hello, {}!", name);
    print!("{greeting}");
}
```

更多场景见 [`examples/showcase.rs`](examples/showcase.rs)（应用启动配置实战：`cargo run --example showcase`）。

> 档位说明：所有混淆宏均提供 1（低开销）、2（推荐均衡）、3（高强度抗逆向）三种安全档位。



## 设计哲学 (Design Philosophy)

### 核心原则


1. **极度多态化** —— 同一份输入每次编译都会产出形态完全不同的密文：
   - **随机分块**：按递增步长随机切分载荷，分块边界每次编译不同；
   - **原语随机叠加**：按强度档随机选取若干层加解密算法依次叠加，直到组合安全度达标为止；
   - **随机存储形态**：密文可能以字节串 / `u64` 数组 / `u128` 数组 / MAC 地址 / UUID / IPv6 地址 / PEM 等多种"伪装外观"之一出现；
   - **密钥隐写**：每层密钥同样经随机存储形态隐藏，仅在运行时才还原，展开代码中不会出现明文密钥常量；
   - **干扰注入**：额外混入垃圾代码与无意义的伪状态变量，进一步扰乱静态分析与数据流追踪。

   以上机制共同保证批量还原脚本无法依赖任何固定模式。

2. **混淆流程抽象化** —— 全部算法收敛为两张注册表：`Crypto { name / enc / dec / support / security / latency / metrics }`（多态加解密）与 `Storage { ast / support / security / latency }`（多态密文存储），`build_obfuscated_bytes` 只负责分块→加密→存储→发射的编排。新增算法只需追加表项，无需改动流程。
3. **编译期求值、最小运行时依赖** —— 过程宏展开为封闭 Token 流，混淆在编译期完成，运行时仅依赖 `lib-unknown` 的 `types`、`crypto` 与 `encoding`；`no_std` 可用；展开期已验证的不变量用 `unwrap_unchecked`，不留运行时校验开销。
4. **符号执行对抗** —— obfstr2 不再停留在"简单异或"的传统混淆层面，而是将**现代对称密码学设计原则（混淆 Confusion、扩散 Diffusion、SPN 网络、雪崩效应）**引入到字符串混淆中，以对抗 `angr` 等自动化符号执行逆向工具（见 [贡献](#贡献-contributing) 章节的复现命令）。
### 权衡取舍 (Trade-offs)

| 我们选择了   | 而不是             | 原因                                                                   |
|:-------------|:-------------------|:-----------------------------------------------------------------------|
| 强度优先     | 极致压缩生成体积   | 高强度的级联解密与多态展开必然带来代码膨胀，这是阻断逆向分析的必要代价 |
| 非确定性构建 | 可复现构建         | 保证多态性是抵抗自动化特征提取的核心防线                               |
| 工程有效防御 | 理论密码学永久安全 | 本项目目标是在逆向分析、符号破解与逆向工程中大幅抬高攻击成本           |

> 虽然不保证密码学安全，但在积极引入现代密码学设计思想。


### 非目标 (Non-Goals)

- 不做控制流混淆与反调试。
- 不保证可复现构建、不替代加密存储敏感数据。


## 宏一览

| 宏                                | 输入                                                                        | 强度档                                                                                                                             |
|-----------------------------------|-----------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------|
| `s1!` / `s2!` / `s3!`             | `"..."` 字符串字面量                                                        | 低延迟 / 均衡 / 高强度                                                                                                             |
| `b1!` / `b2!` / `b3!`             | `b"..."` 或 `[0x41, 66, ...]`（元素 0..=255）                               | 低延迟 / 均衡 / 高强度                                                                                                             |
| `i1!` / `i2!` / `i3!`             | `42u8` / `-1` / `0xFFu16` 等整数字面量（空后缀视为 `i32`）                  | 低延迟 / 均衡 / 高强度                                                                                                             |
| `fl1!` / `fl2!` / `fl3!`          | `3.15f32` / `-1.0` / `1e10` 等浮点字面量（空后缀视为 `f64`）                | 低延迟 / 均衡 / 高强度                                                                                                             |
| `cs1!` / `cs2!` / `cs3!`          | `"..."` / `c"..."` / `b"..."`（`b"..."` 可含非 UTF-8 字节，载荷禁内部 NUL） | 低延迟 / 均衡 / 高强度                                                                                                             |
| `w1!` / `w2!` / `w3!`             | `"..."` 字符串字面量（UTF-16，不追加 NUL，允许内部 NUL）                    | 低延迟 / 均衡 / 高强度                                                                                                             |
| `cw1!` / `cw2!` / `cw3!`          | `"..."` 字符串字面量（UTF-16，自动追加结尾 `0u16`，载荷禁内部 NUL）         | 低延迟 / 均衡 / 高强度                                                                                                             |
| `f1!` / `f2!` / `f3!`             | `"path/to/file"` 文件路径字面量                                             | 低延迟 / 均衡 / 高强度                                                                                                             |
| `s_fmt1!` / `s_fmt2!` / `s_fmt3!` | `"...{}..."` 格式串 + 参数                                                  | 字面量片段按档位混淆后走 `format!`，返回 `String`（需 `alloc`，`no_std` 可用；`2` / `3` 档的字面量片段可能走 `Heap*`，需默认特性） |


**容器类型**

- 字符串宏 → `StackStr<N>` / `HeapStr<N>`
- 字节 / 文件宏 → `StackBytes<N>` / `HeapBytes<N>`
- C 字符串宏 → `StackCStr<N>` / `HeapCStr<N>`（`N` 含结尾 `\0`，`N >= 1`；解引用为 `core::ffi::CStr`，`Drop` 自动清零）
- 宽字符串宏 → `StackU16<N>` / `HeapU16<N>`（`N` 为 UTF-16 码元数，不含结尾；允许内部 NUL；`Deref<Target = [u16]>`，`Drop` 自动清零）
- 宽 C 字符串宏 → `StackU16<N>` / `HeapU16<N>`（`N` 为 UTF-16 码元数，含结尾 `0u16`；`Deref<Target = [u16]>`，`as_ptr()` 可直接用于 `PCWSTR`，`Drop` 自动清零）
- 整数宏展开为裸整数值（`u8`/`i8`/`u16`/`i16`/`u32`/`i32`/`u64`/`i64`/`u128`/`i128`/`usize`/`isize`，由字面量后缀决定）
- 浮点宏展开为裸浮点值（`f32` / `f64`，由字面量后缀决定）

> 返回的具体类型可能随编译变化，请使用`let val = s2!("...");`类型推断，不要写死类型标注。

**整数 / 浮点宏**

- 复用同一套字节混淆内核（整数小端编码、浮点按 `to_bits` 小端编码后走分块→加密→存储→发射）；
- 无 `Drop` 自动清零。
- `usize` / `isize` 按 64 位语义编码（`u64` / `i64` 中转后 `as` 转换）；
- 浮点仅接受有限常规值（`inf` / `NaN` 一律拒绝），`-0.0` 按位保留符号位，断言请用 `to_bits()` 而非 `==`。

**C 字符串宏**

- 同样复用字节混淆内核：载荷追加结尾 `\0` 后整体混淆，运行时经 `try_from(&mut [u8])` 还原（首 NUL 截断 + 解密源擦除）；
- 三种字面量形态语义等价，非 UTF-8 载荷请用 `b"..."`；
- 载荷含任何内部 NUL 即编译期报错。

**宽字符串宏（无 NUL）**

- 只接受 `"..."` 字符串字面量，编译期经 `encode_utf16` 转为 UTF-16 码元；**不追加**结尾 `0u16`，且 **不限制内部 NUL**（`U+0000` 作为普通码元保留）；
- 返回 `StackU16<N>` / `HeapU16<N>`，`N` 即码元数（空串 `N = 0`）；`Deref<Target = [u16]>`，`Drop` 自动清零；
- 返回值不保证以 NUL 结尾，适用于显式长度 / 长度前缀 API；需要 NUL 结尾的 Win32 `PCWSTR` 请使用 `cwN!`；
- `w1!` 恒纯栈；`w2!` / `w3!` 按档位与 `alloc` 在 `StackU16` / `HeapU16` 间选择。

**宽 C 字符串宏**

- 只接受 `"..."` 字符串字面量，编译期经 `encode_utf16` 转为 UTF-16 码元，自动追加唯一结尾 `0u16`；载荷含任何 `U+0000` 即编译期报错（防截断）；
- 返回 `StackU16<N>` / `HeapU16<N>`，`N` 含结尾 `0u16`（空串即 `N = 1`）；`Deref<Target = [u16]>`，`as_ptr()` 可直接传给 `LoadLibraryW` / `GetModuleHandleW` 等 Win32 `PCWSTR` 参数；
- 非 BMP 字符按 UTF-16 代理对占 2 个码元；`String::from_utf16(&w[..w.len() - 1])` 可还原文本（末尾 `0u16` 需自行排除）；
- 需要无结尾 NUL / 允许内部 NUL 的宽字符串请使用 `wN!`；
- `cw1!` 恒纯栈；`cw2!` / `cw3!` 按档位与 `alloc` 在 `StackU16` / `HeapU16` 间选择。

## 适用场景 vs 不适用场景

**适合：**

- `no_std` 固件 / 裸机程序中隐藏字符串与常量字节。
- 需要规避静态字符串批量扫描的场景。

**不适合：**

- 需要合规审计、明文可读的场景。
- 需要制品哈希稳定（可复现构建）的发布流程。

## 特性标志 (Feature Flags)

| Feature       | 默认启用 | 说明                                                                                                       |
|:--------------|:---------|:-----------------------------------------------------------------------------------------------------------|
| `default`     | ✅       | 开启堆内存支持：同时启用下游 `lib-unknown` 与 `obfstr2-macros` 的 `alloc`，展开代码可用 `Heap*` 堆内存类型 |
| `budget-1k`   |          | 栈预算控制：纯栈上限 `STACK_BUDGET` 设为 1 KiB                                                             |
| `budget-4k`   |          | 栈预算控制：`STACK_BUDGET` 设为 4 KiB（默认值）                                                            |
| `budget-16k`  |          | 栈预算控制：`STACK_BUDGET` 设为 16 KiB                                                                     |
| `budget-64k`  |          | 栈预算控制：`STACK_BUDGET` 设为 64 KiB                                                                     |
| `budget-256k` |          | 栈预算控制：`STACK_BUDGET` 设为 256 KiB                                                                    |
| `budget-512k` |          | 栈预算控制：`STACK_BUDGET` 设为 512 KiB                                                                    |
| `budget-1m`   |          | 栈预算控制：`STACK_BUDGET` 设为 1 MiB                                                                      |
| `budget-4m`   |          | 栈预算控制：`STACK_BUDGET` 设为 4 MiB                                                                      |
| `budget-8m`   |          | 栈预算控制：`STACK_BUDGET` 设为 8 MiB                                                                      |

注意：

- 关闭后为纯栈：1 档（`s1` / `b1` / `i1` / `fl1` / `cs1` / `w1` / `cw1` / `f1`）恒纯栈、无影响；chunk 中间容器峰值超 `STACK_BUDGET`/2（默认 4 KiB 时为 2048B）且无堆可用时为编译期报错。
- `budget-*` 可自由组合：同时启用多个时取最大档生效，未指定时 `STACK_BUDGET` 为默认 4 KiB。
- `s_fmt1!` / `s_fmt2!` / `s_fmt3!` 展开为 `alloc::format!` 并返回 `String`，下游需 `alloc`（`no_std` 可用），与纯栈组合无关。
- 运行时固定依赖 `lib-unknown` 的 `crypto` / `types` / `encoding`；`encoding` 用于展开代码的 Hex / Base64 解码，不可裁剪。

如需禁用默认特性（纯栈、无堆）：

```toml
[dependencies]
obfstr2 = { version = "0.3", default-features = false }
```

## 平台与环境支持

- `no_std` 可用；`Heap*` 类型需启用 `default` feature。
- 支持操作系统：无平台相关代码（纯 Rust，主流桌面系统均可；裸机链路在 `x86_64-unknown-none` 上验证，见贡献章节）。
- WebAssembly：`wasm32-unknown-unknown` / `wasm32-wasi` 理论支持但未测试。


## 性能 (Benchmarks)

`b1!` 几乎零运行时开销（与明文基线持平）；`b2!` 运行时开销约数十倍；`b3!` 开销更高但对应更高抗分析强度。编译体积与构建耗时也随档位显著上升——请按「运行时敏感」还是「逆向成本敏感」选择档位。下方为详细数据与复现方式。


以下数据的唯一来源是 `tests/perf.rs`（`cargo test --test perf -- --ignored --nocapture` 可复跑；报告-only，数字只展示不断言）。多态导致每次结果不同，下表为单次实测示例，仅供量级参考。


<details>
<summary>参数配置</summary>

| 参数           | 取值     | 说明                                                                         |
|:---------------|:---------|:-----------------------------------------------------------------------------|
| `PAYLOAD_LEN`  | 1024     | 载荷字节数，全 `a`                                                           |
| `LOOPS`        | 512      | 每 guest 内“解密 + 求校验和”循环次数                                         |
| `REPEATS`      | 3        | 每用例重复次数，取中位数                                                     |
| `TIMEOUT_SECS` | 300      | 单次构建 / 运行超时（秒）                                                    |
| profile        | release  | 冷构建为独立临时工程从零编译（含三方依赖）；增量构建为预热后仅复写 `main.rs` |
| `EXPECTED_SUM` | 50855936 | 校验和断言值（`1024 × 97 × 512`），正确性门禁                                |

参数说明：

- 每次去混淆耗时：`（行耗时 − 基线耗时）/ LOOPS`，单位 µs；假设各行启动开销相同、差分后相消；基线行记 `—`；与基线无差异的行结果≈0 或为负，属噪声，原样显示。
- 运行耗时：`LOOPS` 次解密循环（含进程启动开销与容器 `Drop` 擦除）；循环体内均有 `black_box`（`CasualX` 与 obfstr2 均对返回值整体加钉，口径对称），防编译器把循环不变的解密外提。
- 增量构建耗时：共享预热 runner（合并 `obfstr` + `obfstr2` 依赖预编译一次）后，仅复写 `main.rs` 触发的增量编译耗时，同样取 3 次中位数；只测 `build`，不测运行 / 体积 / expand。
- 冷构建耗时：每次新建独立临时工程从零编译（含三方依赖编译），取 3 次中位数；行间差值才是宏展开的边际成本，百分比被固定成本稀释，仅供量级参考。
- 运行耗时 / 产物体积 / expand 字符数：取自冷构建阶段样本（与构建路径无关，热构建阶段不重复测量以节省时间）。
- expand 字符数：`cargo expand` 输出字符数，需执行机安装 `cargo-expand`，缺失记 `n/a`，含约 0.5k 样板代码。
- 产物二进制：未 strip，取 `DnyRun::bin_path()` 的 `fs::metadata` 大小。
- 括号内：相对明文基线的增幅百分比（基线行为 0% 起点）；`-0%` 为四舍五入到整数的结果，表示略低于基线。

</details>

| 用例              | 每次去混淆耗时 | 运行耗时（512 次解密循环，含启动开销） | 增量构建耗时         | 冷构建耗时            | 产物二进制（未 strip） | expand 字符数    |
|:------------------|:---------------|:---------------------------------------|:---------------------|:----------------------|:-----------------------|:-----------------|
| 明文基线          | —              | 10.365382ms                            | 171.992052ms         | 414.602076ms          | 445040                 | 1437             |
| CasualX obfbytes! | -0.10µs        | 10.313586ms（-0%）                     | 211.984155ms（23%）  | 1.546629633s（273%）  | 448072（1%）           | 7780（441%）     |
| obfstr2 b1!       | 19.57µs        | 20.383619ms（97%）                     | 686.693935ms（299%） | 5.199780109s（1154%） | 519688（17%）          | 104605（7179%）  |
| obfstr2 b2!       | 295.99µs       | 161.912404ms（1462%）                  | 1.619466966s（842%） | 5.734792802s（1283%） | 640912（44%）          | 284900（19726%） |
| obfstr2 b3!       | 926.98µs       | 484.979339ms（4579%）                  | 4.180500072s（2331%）| 7.775605694s（1775%） | 846864（90%）          | 637097（44235%） |

## 安全性 (Security)
> 免责声明：本项目仅供软件安全防护、防逆向工程研究、固件代码保护等合法合规场景使用，严禁用于任何恶意软件制作或非法渗透测试。

- Unsafe 代码：展开代码含 `unwrap_unchecked`（长度不变量已在展开期验证，运行时无校验开销）；容器 `Drop` 经 volatile 写清零明文。
- 如发现安全漏洞，请直接提交 Issue 说明（本仓库暂无私有上报通道）。


## 贡献 (Contributing)

欢迎提交 Issue 和 Pull Request！

**本地验证：**

- `cargo test --test smoke --test s_fmt`：全部宏正确性，进程内断言，毫秒级；
- `cargo run --example showcase`：showcase 综合示例；
- `cargo test --test compile_fail`：非法输入的编译期拒绝（各用例隔离的 dyntest 工程）；
- `cargo test --test nostd`：`x86_64-unknown-none` 裸机链路（`b1` 纯栈无分配 + `b2` 含堆，需安装该 target）；
- `cargo test --test perf -- --ignored --nocapture`：性能对比报告（明文基线 vs CasualX vs `b1/b2/b3`，编译耗时 / 运行耗时 / expand 字符数 / 产物体积，只展示不断言，需联网拉取 `obfstr` 并安装 `cargo-expand`）；
- `cd obfstr2-macros && cargo test --lib`：纯单元测试与加密选层模拟（毫秒级，自动跳过 `#[ignore]` 用例）；
- `cd obfstr2-macros && cargo test --lib test_storages -- --ignored --nocapture`：全部存储策略的 dyntest 往返验证（会构建临时 guest 工程）；
- `cd obfstr2-macros && cargo test --lib tests::crypto::test_cryptos_latency -- --ignored --nocapture`：原语 L 分综合校准（多样本多轮，报告-only，耗时数分钟）；
- `cd obfstr2-macros && cargo test --lib tests::crypto::test_cryptos_security -- --ignored --nocapture`：原语安全度与 angr 对抗校准（需安装 `angr`）；
- `ANGR_PYTHONPATH=/path/to/python-with-angr cargo test --test angr -- main --ignored --show-output`：抗符号执行验证（需安装 `angr`；通过 `ANGR_PYTHONPATH` 指定解释器路径）。

**提交前检查：**

```bash
cargo fmt --all -- --check
cd obfstr2-macros && cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cd obfstr2-macros && cargo clippy --all-targets --all-features -- -D warnings
```

提交 PR 前请先阅读 [设计哲学](#设计哲学-design-philosophy)：新增混淆原语请以 `Crypto` / `Storage` 表项形式接入（分别位于 `obfstr2-macros/src/crypto.rs` 与 `storage.rs`）；新增宏需在 `obfstr2/src/lib.rs` 加单层 `macro_rules` 包装 + 用户文档，`obfstr2-macros` 侧写解释文档 + `# Warning`。
## 变更日志 (Changelog)

版本变更详情请见 [CHANGELOG.md](CHANGELOG.md)。

## 开源协议 (License)

本项目基于 [MPL-2.0](./LICENSE) 协议开源。除非另有明确声明，您提交的任何贡献默认按同等协议条款授权。
