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
- [核心亮点](#核心亮点-core-highlights)
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
obfstr2 = "0.4"
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


## 核心亮点 (Core Highlights)

`obfstr2` 并非简单的“编译期异或”宏，而是一套针对**白盒逆向分析、反编译器（IDA/Ghidra F5）与符号执行引擎（angr/Z3）**深度定制的多态密码学混淆管线：

### 1. 抗符号执行与反编译的密码学原语库
内置 17+ 种多态加解密原语，涵盖流密码、代换-置换网络（SPN）、动态盒变换与分组密码，按运行期延迟（Latency）与安全度（Security）量化分级：
- **自研 MSE / MSE2 算法家族**：融合 CFB 密文反馈、随历史状态逐字节实时变异的动态 S-Box / P-Box、基于超越数（ $\pi$ 、 $1/e$ ）定点近似的 128 位非线性密钥派生，以及 64B 分组 4 轮 ARX 扩散，彻底断绝攻击者通过识别标准算法常数（如 ChaCha20 特征）调库批量解密的可能。
- **浮点跨域代数混淆（`rot_xor_f` / `xor_diff_f`）**：将基础整数位运算（旋转、异或）等价展开为 IEEE-754 浮点除法、截断与指数构造。在大幅拉升反编译器 F5 伪代码认知负载的同时，直接引发 SMT 求解器在浮点非线性约束下的路径爆炸与求解超时。
- **动态盒求逆与模乘逆元**：编译期采用 $\pmod{2^8}$ 奇数乘法逆元加密、运行期单次乘法解密（`inv_mul`）；运行期基于密钥实时洗牌生成 S-Box / P-Box 并求逆（`gen_s` / `inv_s`），彻底消除二进制静态查表特征。
- **全局位置扩散**：引入基于 128 位无偏乘法区间映射的 `fisher_yates` 置换，打破字节边界对齐。

### 2. 逆向密钥棘轮与全链路雪崩耦合
拒绝在二进制中暴露每层的独立密钥，构建贯穿“编译期正向加密”与“运行期逆序解密”的双向状态耦合链：
- **零额外密钥存储（ $\pmod{2^{64}}$ 乘法逆元棘轮）**：每个分块仅存储唯一根密钥。编译期利用 64 位奇数乘法逆元 `inv_mul64` 结合当前层尾字节逆向推演外层密钥；运行期解密时则通过尾字节正向还原下一层密钥：
  $$K _ {\text{next}} = (K _ {\text{cur}} + \text{ghost\\_state}) \times (\text{last\\_byte} \mid 1) \pmod{2^{64}}$$
- **跨块幽灵状态（`ghost_state`）**：每个分块解密并拼入主容器后，提取主容器末尾字节累加更新全局 `ghost_state`（编译期通过模拟器 `sim_ghost` 精确镜像追踪）。任何试图通过调试器跳过前置分块、内存单字节 Patch 或静态剥离单个子函数的行为，均会引发后续所有块与层密钥的彻底雪崩。

### 3. 超线性分块、拟真垃圾块与数论不透明谓词
- **超线性分块与短块填充**：按 $\lceil i^{1.4} \rceil$ 递增步长随机切分载荷（头部极度细碎以打散特征，尾部平滑增长以控制块数）；中高强度档对 $<16\text{B}$ 短块强制前置 `pkcs7_16` 填充，消除短字符串长度侧信道。
- **100% 拟真垃圾块（Junk Chunks）**：按档位概率随机混入伪造数据块，与真实分块经历完全相同的多层加密、密文存储与运行期解密流程，在反编译视图中真假难辨。
- **初等数论不透明谓词**：在分块解密末尾注入基于模 4 二次剩余（如 $x^2 \not\equiv 2 \pmod 4$ 恒成立）等数论恒等式的完整性校验。经 `black_box` 阻断编译期折叠后，垃圾块在运行期必然触发 `wipe_data()` 将自身清零擦除，确保主容器与 `ghost_state` 状态严丝合缝。

### 4. 调用图粉碎与 LLVM 优化屏障
针对性防御静态交叉引用（Xrefs）追踪、批量函数 Hook 与编译器激进优化：
- **自适应包装与虚表蹦床（Vtable Trampoline）**：连续内联层数越多，外包为独立函数的概率呈指数级上升。外包形态以 50% 概率随机生成具名 `#[inline(never)]` 函数，或通过 `&mut dyn FnMut` 胖指针间接调用的**动态派发蹦床函数**，将直接调用转化为虚表间接调用，直接斩断静态调用图。
- **O(log M) 自底向上多叉折叠树**：当分块数量较多时，自动将相邻 4~12 个块节点递归封装为多级调用树（`__fold_{id}_{depth}_{idx}`），避免根函数随载荷线性膨胀。
- **冷热段剥离与反跨过程优化（Anti-IPO）**：按概率随机附加 `#[cold]` 属性将解密逻辑驱逐至 `.text.unlikely` 段以破坏空间局部性；通过将函数指针强转为 `usize` 投入 `black_box` 伪造地址逃逸，阻止 LLVM 执行跨过程常量传播与死参数消除。

### 5. 多态隐写存储与受控内存生命周期
- **密文与密钥多态伪装**：密文及每块的 8 字节初始密钥均通过随机 `Storage` 策略编码，伪装为普通字节流、`u64`/`u128` 整数数组、MAC 地址、UUID、IPv6 地址、PEM 证书或 HB2 空白字符隐写，规避高熵数据段扫描。
- **全链路 Volatile 擦除**：不仅最终返回的字符串/字节容器在离开作用域（`Drop`）时执行整段容量的 Volatile 清零，变长解密层（如 `mse2`、`pkcs7_16`）的中间临时容器在交接后也会立即 `drop` 擦除，杜绝明文或中间态在栈/堆内存中残留。
- **精细化栈预算（`STACK_BUDGET`）**：支持通过 `budget-1k` ~ `budget-8m` 特性控制纯栈开销上限，编译期自动追踪多层加解密的峰值膨胀体积（Peak Size），在 `Stack*` 与 `Heap*` 容器间智能路由，无缝适配 `no_std` 裸机与嵌入式固件。

### 6. MB 级超大载荷极速编译（免 `long_running_const_eval`）
在处理 MB 级及以上的超大载荷（如通过 `f1!` / `f2!` 内嵌大型二进制固件或资源文件）时，`obfstr2` 的编译速度显著快于传统的 `obfstr`，且**无需在代码中添加 `#![allow(long_running_const_eval)]`**：
- **内联背压（Backpressure）与多叉折叠树**：大载荷如果展开为单一巨型函数，会导致 LLVM 后端在寄存器分配与指令调度阶段陷入 $O(V^2)$ 复杂度爆炸。`obfstr2` 借助 $\lceil i^{1.4} \rceil$ 超线性分块控制总块数，配合指数级内联背压（`effective_wrap_pct`）与 **O(log M) 多叉折叠树（`fold_chunk_nodes`）**，将每个函数体的子节点数严格收敛在常数级（ $\le 12$ ），使 LLVM 后端能够以近似线性的时间轻松完成数 MB 载荷的编译。

---

## 设计哲学 (Design Philosophy)

### 核心原则

1. **面向白盒的威胁模型** —— 传统密码学算法（如标准 ChaCha20/AES）在黑盒下坚不可摧，但在逆向人员拥有二进制完全控制权的**白盒模型**下，一旦算法特征或常数被识别，攻击者只需提取 Key/IV 即可在数十秒内用脚本批量解密。`obfstr2` 以**最大化逆向工程认知阻力与脚本还原耗时**为核心指标，通过非标自研原语与多态编排迫使攻击者逐个分析。
2. **互补概率安全度模型** —— 加密选层并非固定套路，而是根据档位设定的延迟上限（`max_latency`）与目标安全度（`min_security`），从原语池中随机抽样叠加，直至组合安全度满足：
   $$\text{Security} _ {\text{combined}} = 1 - \prod _ {i=1}^{k} \left(1 - \frac{\text{Security} _ i}{100}\right) \ge \text{min\\_security}$$
3. **注册表解耦架构** —— 全部算法收敛为两张静态注册表：`Crypto { name / enc / dec / support / security / latency / metrics }`（多态加解密）与 `Storage { ast / support / latency }`（多态密文存储），编排器 `build_obfuscated_bytes` 仅负责“分块 → 加密 → 存储 → 折叠发射”。新增原语只需追加表项，无需改动编排管线。
4. **编译期求值与零冗余校验** —— 过程宏在编译期完成全部加密与状态模拟，展开为封闭的 Token 流；运行时仅依赖 `lib-unknown` 的 `types`、`crypto` 与 `encoding`（`no_std` 可用）；编译期已严格验证的长度/对齐不变量在运行期直接使用 `unwrap_unchecked`，不留分支校验开销。

### 权衡取舍 (Trade-offs)

| 我们选择了   | 而不是             | 原因                                                                           |
|:-------------|:-------------------|:-------------------------------------------------------------------------------|
| 强度优先     | 极致压缩生成体积   | 高强度的级联解密、垃圾块与多态展开必然带来代码膨胀，这是阻断逆向分析的必要代价 |
| 非确定性构建 | 可复现构建         | 保证每次编译产物特征完全不同，是抵抗自动化特征提取与批量脚本的核心防线         |
| 白盒对抗强度 | 理论密码学永久安全 | 目标是在白盒逆向分析、符号执行与动态调试中大幅抬高攻击者的时间与算力成本       |

> 虽然不保证密码学理论上的永久安全，但在工程实现上深度引入了现代密码学设计思想（混淆 Confusion、扩散 Diffusion、SPN 网络、雪崩效应）。

### 非目标 (Non-Goals)

- 不做宿主业务代码的控制流平坦化（仅对宏展开的内部解密链路实施调用图与控制流混淆），也不内置反调试器检测。
- 不保证可复现构建（Reproducible Builds），不替代标准密码学库用于网络传输或持久化加密存储。


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

- 同样复用字节混淆内核：载荷追加结尾 `\0` 后整体混淆，运行时经 `from_inner_with_nul` 零拷贝还原（首 NUL 截断，解密容器直接转为 CStr）；
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
- 需要规避静态字符串批量扫描、反编译器伪代码分析与符号执行的场景。
- 需要内嵌并混淆 MB 级大型二进制文件（且不希望触发 `long_running_const_eval` 编译限制）的场景。

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
obfstr2 = { version = "0.4", default-features = false }
```

## 平台与环境支持

- `no_std` 可用；`Heap*` 类型需启用 `default` feature。
- 支持操作系统：无平台相关代码（纯 Rust，主流桌面系统均可；裸机链路在 `x86_64-unknown-none` 上验证，见贡献章节）。
- WebAssembly：`wasm32-unknown-unknown` / `wasm32-wasi` 理论支持但未测试。


## 性能 (Benchmarks)

`b1!` 几乎零运行时开销（与明文基线持平）；`b2!` 运行时开销约数十倍；`b3!` 开销更高但对应更高抗分析强度。在短小载荷（如下表 1 KiB 样本）下，由于生成了多态调用树与内联屏障，构建耗时随档位上升；**但在面对 MB 级及以上的超大载荷时，得益于原生过程宏执行、内联背压与 $O(\log M)$ 多叉折叠树，`obfstr2` 的编译速度反而比依赖 `const` 求值的 `obfstr` 更快，且无需配置 `allow(long_running_const_eval)`**。请按「运行时敏感」还是「逆向成本敏感」选择档位。下方为详细数据与复现方式。

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

| 用例              | 每次去混淆耗时 | 运行耗时（512 次解密循环，含启动开销） | 增量构建耗时          | 冷构建耗时            | 产物二进制（未 strip） | expand 字符数    |
|:------------------|:---------------|:---------------------------------------|:----------------------|:----------------------|:-----------------------|:-----------------|
| 明文基线          | —              | 10.364751ms                            | 171.966449ms          | 454.140508ms          | 445040                 | 1437             |
| CasualX obfbytes! | 0.01µs         | 10.371281ms（0%）                      | 202.474127ms（18%）   | 1.909735432s（321%）  | 448072（1%）           | 7780（441%）     |
| obfstr2 b1!       | 19.46µs        | 20.327803ms（96%）                     | 475.87713ms（177%）   | 5.003601624s（1002%） | 512192（15%）          | 71476（4874%）   |
| obfstr2 b2!       | 314.51µs       | 171.393708ms（1554%）                  | 1.232847096s（617%）  | 5.00176234s（1001%）  | 613456（38%）          | 190793（13177%） |
| obfstr2 b3!       | 768.22µs       | 403.694785ms（3795%）                  | 2.860009413s（1563%） | 6.236919564s（1273%） | 812000（82%）          | 536333（37223%） |

## 安全性 (Security)
> 免责声明：本项目仅供软件安全防护、防逆向工程研究、固件代码保护等合法合规场景使用，严禁用于任何恶意软件制作或非法渗透测试。

- **Unsafe 代码**：展开代码含 `unwrap_unchecked`（长度不变量已在展开期验证，运行时无校验开销）；容器 `Drop` 经 volatile 写清零明文。
- **漏洞披露**：若发现安全隐患，请通过
  [GitHub Private Vulnerability Reporting](https://github.com/AlbusPhos/obfstr2/security/advisories/new)
  提交，或发送邮件至 `owo.zzz@proton.me`。**请勿**为未公开的安全漏洞创建公开 Issue 或 Pull Request。


## 贡献 (Contributing)

欢迎提交 Issue 和 Pull Request！

**本地验证：**

- `cargo test --test smoke --test s_fmt`：全部宏正确性，进程内断言，毫秒级；
- `cargo run --example showcase`：showcase 综合示例；
- `cargo test --test compile_fail`：非法输入的编译期拒绝（各用例隔离的 dyntest 工程）；
- `cargo test --test nostd`：`x86_64-unknown-none` 裸机链路（`b1` 纯栈无分配 + `b2` 含堆，需安装该 target）；
- `cargo test --test perf -- --ignored --nocapture`：性能对比报告（明文基线 vs CasualX vs `b1/b2/b3`，编译耗时 / 运行耗时 / expand 字符数 / 产物体积，只展示不断言，需联网拉取 `obfstr` 并安装 `cargo-expand`）；
- `cd obfstr2-macros && cargo test --lib`：纯单元测试与加密选层模拟（毫秒级，自动跳过 `#[ignore]` 用例）；
- `cd obfstr2-macros && cargo test --lib test_storages_latency -- --ignored --nocapture`：全部存储策略的 dyntest 往返验证与 latency 校准（多长度、多轮采样，会构建临时 guest 工程，耗时较长）；
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