# obfstr2

> **Polymorphic compile-time string/wide-str/wide-cstr/bytes/int/float/cstr/file obfuscation**

[![Crates.io](https://img.shields.io/crates/v/obfstr2?logo=rust)](https://crates.io/crates/obfstr2)
[![Docs.rs](https://img.shields.io/docsrs/obfstr2?logo=docs.rs)](https://docs.rs/obfstr2)
[![CI](https://img.shields.io/github/actions/workflow/status/AlbusPhos/obfstr2/ci.yml?logo=github&label=CI)](https://github.com/AlbusPhos/obfstr2/actions)
[![MSRV](https://img.shields.io/crates/msrv/obfstr2?logo=rust&label=MSRV)](https://crates.io/crates/obfstr2)
[![Downloads](https://img.shields.io/crates/d/obfstr2?color=blue)](https://crates.io/crates/obfstr2)
[![License](https://img.shields.io/crates/l/obfstr2)](https://mozilla.org/MPL/2.0/)

**语言：** [English](README_en.md) | 简体中文

一行宏即用的 Rust 编译期多态安全混淆框架。字面量输入，安全表达式输出，所有加密与控制流形变均在编译期完成。每次编译均生成截然不同的二进制特征与密文形态，彻底挫败静态特征扫描、Mandiant `FLOSS` 以及针对 Rust 定制的专用去混淆器（如 [`rust-deobf`](https://github.com/VAHPEM/rust-deobf)）。

全面覆盖**字符串、C 字符串、UTF-16 宽字符串、字节数组、外部文件、整数、浮点数及格式化字符串**。支持解引用直取裸值或存入受控容器；所有字符串与字节容器在离开作用域（`Drop`）时自动执行 Volatile 内存清零，防止敏感明文残留在栈或堆中。

## 目录

- [快速开始](#快速开始)
- [核心亮点与对抗验证](#核心亮点与对抗验证)
- [设计哲学与适用边界](#设计哲学与适用边界)
- [宏一览](#宏一览)
- [特性标志](#特性标志)
- [性能基准](#性能基准)
- [安全性](#安全性)
- [贡献与开发验证](#贡献与开发验证)
- [开源协议](#开源协议)

## 快速开始

```toml
[dependencies]
obfstr2 = "0.5"
```

所有混淆宏均以数字后缀区分强度档位：`1`（低延迟）、`2`（推荐均衡）、`3`（高强度抗逆向）；
另有 [`*_user!`](#自定义强度档-_user) 自定义档，以配置块在均衡档基础上逐字段覆盖档位参数。

```rust
use obfstr2::{b2, cs2, cw2, f2, fl2, i2, s2, s_fmt2, w1};

fn main() {
    // 字符串：返回自有安全容器，可绑定、传递、复用，Drop 时自动清零
    let hello = s2!("hello");
    print!("{hello}");

    // C 字符串：解引用即 &CStr，as_ptr() 可直投底层系统调用
    let sh = cs2!(c"/bin/sh");
    assert_eq!(&*sh, c"/bin/sh");

    // 宽 C 字符串：UTF-16 码元 + 自动结尾 0u16，as_ptr() 可直接用于 Win32 PCWSTR
    let w = cw2!("kernel32.dll");
    assert_eq!(w[w.len() - 1], 0);

    // 宽字符串：纯 UTF-16 码元，不追加结尾 NUL，且允许包含内部 NUL
    let ws = w1!("a\0b");
    assert_eq!(&*ws, &[0x61u16, 0, 0x62][..]);

    // 字节串与字节数组：解引用即 &[u8]
    let b = b2!(b"abc");
    let c = b2!([0x61, 98, 99]);
    assert_eq!(&*b, &*c);

    // 标量整数与浮点：求值即得原生类型裸值，可直接参与运算
    let x = i2!(42u32);
    assert_eq!(x + 1, 43u32);
    let y = fl2!(3.15);
    assert_eq!(y.to_bits(), 3.15f64.to_bits());

    // 外部文件：路径相对于当前 crate 的 CARGO_MANIFEST_DIR，编译期读入并混淆
    let asset = f2!("assets/fixture.bin");

    // 格式化字符串：字面量片段逐个混淆后走 format!，返回 String
    let name = "world";
    let greeting = s_fmt2!("hello, {}!", name);
    print!("{greeting}");
}
```

更多实战用法请参考 [`examples/showcase.rs`](examples/showcase.rs)（运行 `cargo run --example showcase`）。

---

## 核心亮点与对抗验证

`obfstr2` 并非简单的“编译期异或”宏，而是一套针对**专用去混淆工具、白盒逆向分析、反编译器（IDA / Ghidra F5）与符号执行引擎（angr / Z3）**深度定制的多态密码学混淆管线。

### 实战对抗验证：全面免疫专用去混淆器 (`rust-deobf`)

传统的编译期混淆库虽然能绕过 `strings` 与通用分析工具 `FLOSS`，但在针对 Rust 二进制特征与控制流深度定制的专用去混淆工具 [**`rust-deobf`**](https://github.com/VAHPEM/rust-deobf) 面前已全线失守（因其依赖固定内联异或模式、静态密钥布局或标准 AEAD 算法特征）。

得益于多态原语管线、跨块幽灵状态耦合与调用图粉碎机制，`obfstr2` 在实测中彻底阻断了 `rust-deobf` 的静态提取与 Unicorn 模拟执行分析：

1. **实战演示样例 ([`examples/showcase.rs`](examples/showcase.rs))**：受宏保护的业务字符串无一被解出（**0 解出**），去混淆器仅在标准库内部截获到无意义的静态查表字符碎片。
2. **自动化对抗测试 ([`tests/rust_deobf.rs`](tests/rust_deobf.rs))**：对齐 `rust-deobf` 基准评测规范植入 5 大典型 IOC（URL、IP:Port、Mutex、文件路径、注册表路径）。测试结果显示，传统 `obfstr` 基线产物在 0.1 秒内被 **5/5 全量还原**；而切换为 `obfstr2::s2` 均衡档后，函数调用图被粉碎扩张（14 $\to$ 452 个函数），模拟耗时膨胀近 80 倍，最终实现 **0/5 解出（全面免疫）**！

| 混淆方案 / 工具                            | `strings`  |  `FLOSS`   | [`rust-deobf`](https://github.com/VAHPEM/rust-deobf) | 抗分析与去混淆评估                                                |
|:-------------------------------------------|:----------:|:----------:|:----------------------------------------------------:|:------------------------------------------------------------------|
| `litcrypt` / `litcrypt2`                   |   0 / 5    |   0 / 5    |                   **5 / 5 (全破)**                   | ❌ 静态字节异或特征明显                                           |
| `obfstr` (内联 XOR+ADD)                    |   0 / 5    |   0 / 5    |                   **5 / 5 (全破)**                   | ❌ 内联展开模式固定，易被指令模拟还原                             |
| `cryptify` (默认密钥)                      |   0 / 5    |   0 / 5    |                   **5 / 5 (全破)**                   | ❌ 固定解密例程与密钥布局                                         |
| `muddy` (ChaCha20-Poly1305)                |   0 / 5    |   0 / 5    |                   **5 / 5 (全破)**                   | ❌ 标准密码学常数与 AEAD 调用结构可被精准识别                     |
| 手写紧凑栈字符串 / XOR / ADD               |   0 / 5    |   0 / 5    |                   **5 / 5 (全破)**                   | ❌ 缺乏控制流屏障与跨块状态耦合                                   |
| **`obfstr2` (`tests/rust_deobf.rs` 实测)** | **0 解出** | **0 解出** |                 **0 解出 (全免疫)**                  | ✅ **非标多态原语 + 跨块雪崩棘轮 + 虚表蹦床，彻底阻断自动化还原** |

*注：上表前 5 项基线数据引用自 [`rust-deobf` 官方评测基准](https://github.com/VAHPEM/rust-deobf)。*

> **持续对抗演进承诺：**
> 逆向与反逆向是动态博弈的过程。得益于底层高度解耦的注册表架构，**开发者承诺将定期更新、轮换内置密码学算法与隐写伪签名**，持续刷新代码展开形态与数据特征，确保 `obfstr2` 最新版本的动态多态安全性，让针对性提取规则持续失效。

### 1. 抗符号执行与反编译的密码学原语库
内置 17+ 种多态加解密原语，涵盖流密码、代换-置换网络（SPN）、动态盒变换与分组密码，按运行期延迟与安全度量化分级：
- **自研 MSE / MSE2 算法家族**：融合 CFB 密文反馈、随历史状态逐字节实时变异的动态 S-Box / P-Box、基于超越数（ $\pi$ 、 $1/e$ ）定点近似的 128 位非线性密钥派生，以及 64B 分组 4 轮 ARX 扩散，彻底断绝攻击者通过识别标准算法魔数（如 AES / ChaCha20 常数）调库批量解密的可能。
- **浮点跨域代数混淆**：将基础整数位运算（旋转、异或）等价展开为 IEEE-754 浮点除法、截断与指数构造。在大幅拉升反编译器 F5 伪代码认知负载的同时，直接引发 SMT 求解器在浮点非线性约束下的路径爆炸与求解超时。
- **动态盒求逆与模乘逆元**：编译期采用 $\pmod{2^8}$ 奇数乘法逆元加密、运行期单次乘法解密；运行期基于密钥实时洗牌生成 S-Box / P-Box 并求逆，彻底消除二进制静态查表特征。
- **全局位置扩散**：引入基于 128 位无偏乘法区间映射的 Fisher-Yates 置换，打破字节边界对齐。

### 2. 逆向密钥棘轮与全链路雪崩耦合
拒绝在二进制中暴露每层的独立密钥，构建贯穿“编译期正向加密”与“运行期逆序解密”的双向状态耦合链：
- **零额外密钥存储（ $\pmod{2^{64}}$ 乘法逆元棘轮）**：每个分块仅存储唯一根密钥（编译期经入口 `ghost_state` 异或掩码，确保单层块也强依赖全局状态）。多层加密在编译期利用 64 位奇数乘法逆元结合当前层尾字节逆向推演外层密钥；运行期解密时则通过尾字节正向还原下一层密钥：

$$K _ {\text{next}} = (K _ {\text{cur}} + \text{ghost\\_state}) \times (\text{last\\_byte} \mid 1) \pmod{2^{64}}$$

- **跨块幽灵状态（`ghost_state`）**：每个分块解密并拼入主容器后，提取主容器末尾最多 16 字节的滑动窗口，通过 `LightHasher` 计算哈希并异或更新全局 `ghost_state`（启用动态密钥时，初始状态还会混入密钥哈希）。任何试图通过调试器跳过前置分块、内存单字节 Patch 或静态抠出单个子函数模拟执行的行为，均会引发后续所有块与层密钥的彻底雪崩。

### 3. 超线性分块、拟真垃圾块与多态代数不透明谓词
- **超线性分块与短块填充**：按 $\lceil i^{1.4} \rceil$ 递增步长区间随机切分载荷（头部细碎以打散特征，尾部平滑增长以控制总块数）；中高强度档对 < 16B 短块强制前置 `pkcs7_16` 填充，消除短字符串长度侧信道。
- **100% 拟真垃圾块**：按档位概率随机混入伪造数据块，与真实分块经历完全相同的多层加密、密文存储与运行期解密流程，在反编译视图中真假难辨。
- **多态代数与数论不透明谓词**：在分块解密末尾随机注入 4 类高强度完整性校验——涵盖**非线性 MBA（混合布尔算术）恒等式与 $\pmod{2^{64}}$ 乘法逆元对偶**、**汉明重量 `popcnt` 容斥原理与正交位拆分（SMT 求解器毒药）**、**2-adic 环 Hensel 奇数幂不动点（ $v^2 \equiv 1 \pmod 8$ 、 $v^4 \equiv 1 \pmod{16}$ ）与奇偶校验**，以及**德摩根对称差双侧 MBA**。校验式将运行期缓冲区指针地址与 `ghost_state` 深度绑定，经 `black_box` 阻断编译期折叠后，垃圾块与状态被篡改的分块在运行期必然触发 `wipe_data()` 自擦除，确保主容器与全局状态严丝合缝。

### 4. 调用图粉碎与 LLVM 优化屏障
针对性防御静态交叉引用（Xrefs）追踪、批量函数 Hook 与编译器激进优化：
- **双粒度自适应外包与虚表蹦床（Vtable Trampoline）**：在“单层解密”与“分块聚合”双重粒度上，连续内联次数越多，外包为独立函数的概率呈指数级上升。外包形态以 50% 概率随机生成具名 `#[inline(never)]` 函数，或通过 `&mut dyn FnMut` 胖指针间接调用的动态派发蹦床函数，将直接调用转化为虚表间接调用，直接斩断静态调用图。
- **O(log M) 自底向上多叉折叠树**：当分块数量较多时，自动将相邻 4~12 个块节点递归封装为多级调用树，避免根函数随载荷线性膨胀。
- **冷热段剥离与反跨过程优化（Anti-IPO）**：按概率随机附加 `#[cold]` 属性将解密逻辑驱逐至 `.text.unlikely` 段以破坏空间局部性；通过将函数指针强转为 `usize` 投入 `black_box` 伪造地址逃逸，阻止 LLVM 执行跨过程常量传播与死参数消除。

### 5. 多态隐写存储与受控内存生命周期
- **密文与密钥多态伪装**：密文及每块的 8 字节初始密钥均通过随机存储策略编码，伪装为普通字节流、`u64`/`u128` 整数数组、MAC 地址、UUID、IPv6 地址、PEM 证书或 HB2 空白字符隐写，规避高熵数据段扫描。其中 HB2 隐写在编译期嵌入拟真文本语料库（代码片段、日志、脚本等）并随机生成空白符号表，使宿主文本与符号特征均不可被特征化。
- **全链路 Volatile 擦除**：不仅最终返回的容器在 `Drop` 时执行整段容量的 Volatile 清零，变长解密层的中间临时容器在交接后也会立即销毁擦除，杜绝明文或中间态在内存中残留。
- **精细化栈预算（`STACK_BUDGET`）**：支持通过 `budget-1k` ~ `budget-8m` 特性控制纯栈开销上限。编译期自动追踪主容器与分块多层加解密的峰值膨胀体积，在栈容器与堆容器间智能路由，无缝适配 `no_std` 裸机与嵌入式固件。

### 6. MB 级超大载荷极速编译（免 `long_running_const_eval`）
在处理 MB 级大型二进制固件或资源文件时，大载荷若展开为单一巨型函数，会导致 LLVM 后端在寄存器分配阶段陷入 $O(V^2)$ 复杂度爆炸。`obfstr2` 采用原生过程宏求值（无需配置 `#![allow(long_running_const_eval)]`），借助超线性分块、指数级内联背压与 **O(log M) 多叉折叠树**，将每个函数体的子节点数严格收敛在常数级（ $\le 12$ ），使 LLVM 后端能以近似线性的时间极速完成数 MB 载荷的编译。

### 7. 反反汇编花指令注入
针对反汇编器的线性扫描与指令模式识别，在非内联包装函数、内联解密片段、多级折叠聚合函数及宏展开根作用域的首尾，按概率随机注入与真实数据零依赖的架构门控内联汇编（花指令），使垃圾指令与真实解密指令在反汇编视图中深度交错。

---

## 设计哲学与适用边界

### 核心原则
1. **面向白盒的威胁模型**：传统密码学在黑盒下坚不可摧，但在攻击者拥有二进制完全控制权的白盒模型下，一旦标准算法常数（如 ChaCha20 魔数）被识别，提取 Key/IV 即可用脚本或去混淆器秒级还原。`obfstr2` 以**最大化逆向认知阻力与脚本还原耗时**为核心指标，迫使攻击者逐个硬啃非标多态逻辑。
2. **互补概率安全度模型**：加密选层并非固定套路，而是根据档位设定的延迟上限与目标安全度（`min_security`），从原语池中随机抽样叠加（支持通过 `security_factor` 缩放单层贡献），直至组合安全度达标：

$$\text{Security} _ {\text{combined}} = 1 - \prod _ {i=1}^{k} \left(1 - \frac{\text{Security} _ i}{100}\right) \ge \text{min\\_security}$$

3. **注册表解耦与定期轮换机制**：加解密原语与隐写存储策略均收敛为独立的静态注册表，编排器仅负责“分块 → 加密 → 存储 → 折叠发射”流水线。依托此可插拔架构，开发者承诺定期更换并扩充底层算法与伪签名特征，以持续对抗逆向工具的规则演进。
4. **编译期求值与零冗余校验**：过程宏在编译期完成全部加密、状态模拟及长度/对齐不变量验证；运行时直接使用 `unwrap_unchecked` 消除分支校验开销。

### 权衡取舍 (Trade-offs)

| 我们选择了           | 而不是                    | 原因                                                                                     |
|:---------------------|:--------------------------|:-----------------------------------------------------------------------------------------|
| **抗逆向强度优先**   | 极致压缩产物体积          | 级联解密、拟真垃圾块与多态展开必然带来代码膨胀，这是阻断逆向分析与自动化模拟器的必要代价 |
| **非确定性多态构建** | 可复现构建 (Reproducible) | 保证每次编译产物特征完全不同，是抵抗自动化特征提取与批量脚本的核心防线                   |
| **白盒工程对抗**     | 理论密码学永久安全        | 目标是在逆向分析、符号执行与动态调试中大幅抬高攻击者的时间与算力成本                     |

### 适用场景 vs 非目标

| ✅ 推荐使用场景                                                                                                                                                                                                                       | ❌ 不适用场景 / 非目标                                                                                                             |
|:--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:-----------------------------------------------------------------------------------------------------------------------------------|
| • 商业软件 DRM、许可证校验常数、内置公钥/API Key 保护<br>• 游戏反作弊与安全 SDK 中隐藏敏感系统调用名（如 `ntdll.dll`）<br>• `no_std` 嵌入式固件、裸机程序中隐藏硬编码凭证与协议魔数<br>• 内嵌并混淆 MB 级大型二进制资产且要求快速编译 | • 需要二进制制品哈希稳定的可复现构建发布流程<br>• 需要合规审计、明文可读的场景<br>• 不替代标准密码学库用于网络传输或持久化存储加密 |

---

## 宏一览

所有宏均提供 `1`（低延迟）、`2`（均衡）、`3`（高强度）三个档位：

| 宏族                    | 输入字面量示例                                           | 返回类型（解引用目标）                                         | 特性与约束说明                                                                                                       |
|:------------------------|:---------------------------------------------------------|:---------------------------------------------------------------|:---------------------------------------------------------------------------------------------------------------------|
| `s1!` ~ `s3!`           | `"hello"`                                                | `StackStr<N>` / `HeapStr<N>`<br>（`Deref<Target = str>`）      | `Drop` 自动清零。                                                                                                    |
| `b1!` ~ `b3!`           | `b"abc"` 或 `[0x41, 66]`                                 | `StackBytes<N>` / `HeapBytes<N>`<br>（`Deref<Target = [u8]>`） | 数组元素范围 `0..=255`；`Drop` 自动清零。                                                                            |
| `cs1!` ~ `cs3!`         | `c"..."` / `"..."` / `b"..."`                            | `StackCStr<N>` / `HeapCStr<N>`<br>（`Deref<Target = CStr>`）   | 自动追加结尾 `\0` 混淆，运行时零拷贝还原；非 UTF-8 请用 `b"..."`；**严禁内部含 NUL**（防截断）；`Drop` 自动清零。    |
| `w1!` ~ `w3!`           | `"a\0b"`                                                 | `StackU16<N>` / `HeapU16<N>`<br>（`Deref<Target = [u16]>`）    | 编译期转为 UTF-16 码元；**不追加**结尾 NUL，**允许**内部含 NUL；适用于长度前缀 API；`Drop` 自动清零。                |
| `cw1!` ~ `cw3!`         | `"kernel32.dll"`                                         | `StackU16<N>` / `HeapU16<N>`<br>（`Deref<Target = [u16]>`）    | 编译期转为 UTF-16 并**自动追加结尾 `0u16`**；**严禁内部含 NUL**；`as_ptr()` 可直投 Win32 `PCWSTR`；`Drop` 自动清零。 |
| `f1!` ~ `f3!`           | `"assets/data.bin"`                                      | `StackBytes<N>` / `HeapBytes<N>`<br>（`Deref<Target = [u8]>`） | 路径相对被编译 crate 的 `CARGO_MANIFEST_DIR`；`Drop` 自动清零。                                                      |
| `i1!` ~ `i3!`           | `42u32` / `-1` / `0xFFu16`                               | 原生整数裸值（`u8`~`i128` / `usize`）                          | 空后缀默认为 `i32`；`usize`/`isize` 按 64 位语义编码；无 `Drop` 清零。                                               |
| `fl1!` ~ `fl3!`         | `3.15f32` / `-1.0` / `1e10`                              | 原生浮点裸值（`f32` / `f64`）                                  | 空后缀默认为 `f64`；拒绝 `inf`/`NaN`；保留 `-0.0` 符号位（断言请用 `to_bits()`）；无 `Drop` 清零。                   |
| `s_fmt1!` ~ `s_fmt3!`   | `"hi, {}!", name`                                        | `alloc::string::String`                                        | 仅混淆格式串中的静态字面量片段，随后走 `format!`；依赖 `alloc` 特性。                                                |
| `*_user!`（全家族）     | `{ max_latency: 50, ... }, 载荷`                         | 同对应同族宏                                                   | 自定义档：首参数为配置块，以均衡档为基底逐字段覆盖档位参数。                                                         |
| `*_key!`（全家族）      | `{ build_key, run_key }, 载荷`                           | 同对应同族宏                                                   | 动态密钥获取：编译期 `build_key` 加密、运行期 `run_key` 解密；`run_key` 字面量被拒绝。                               |
| `*_key_user!`（全家族） | `{ build_key, run_key }, { max_latency: 50, ... }, 载荷` | 同对应同族宏                                                   | 同时配置动态密钥获取和自定义档强度。                                                                                 |

> **使用提示（堆栈路由与类型推断）：**
> 开启 `alloc`（默认）时，载荷 $\le 16\text{B}$ 恒走栈容器，达到预算上限（主容器 `STACK_BUDGET` / 分块中间容器 `STACK_BUDGET / 2`）恒走堆容器，其间按档位偏好概率随机路由。可通过 Cargo 特性 `force_heap` / `force_stack`（互斥）或 `*_user!` 配置项 `force_heap` 强制**主容器**路由，但分块中间容器不受强制，仍按上述预算与偏好启发式；`force_stack` 仍受 `STACK_BUDGET` 保护，超限会编译期报错。因此**返回的具体容器类型（`Stack*` 或 `Heap*`）可能随每次编译动态变化**，请始终使用 `let val = s2!("...");` 进行类型推断，切勿写死具体容器类型标注。

### 自定义强度档 `*_user!`

全家族统一为 **`({ 配置 }, 载荷)`** 两参形态：首参数为花括号配置块 `{ key: val, ... }`，次参数与对应档位宏的载荷一致。配置以**均衡档（档位 `2`）为基底**逐字段覆盖：

```rust
use obfstr2::{cs_user, s_user};

fn main() {
    let greeting = s_user!(
        {
            prefer_stack: true,      // 堆 / 栈掷币的栈偏好
            max_latency: 60,         // 单层延迟上限（>= 2）
            min_security: 75,        // 组合安全度下限（<= 99，乘法累积）
            junk_pct: 12,            // 垃圾块填充概率（0..=100）
            pad_short: true,         // 短块（< 16B）强制 PKCS7 预填充
        },
        "custom-tier",
    );
    assert_eq!(&*greeting, "custom-tier");

    // 空配置块 {} 等价于均衡档 s2!
    assert_eq!(&*s_user!({}, "hello"), "hello");

    // s_fmt_user! 的字面量片段走自定义档混淆
    assert_eq!(cs_user!({ max_latency: 40 }, c"/bin/sh").to_str().ok(), Some("/bin/sh"));
}
```

| 配置键                      | 取值                           | 预设档位默认值（`1` / **`2` 基底** / `3`） | 说明                                                                                      |
|:----------------------------|:-------------------------------|:------------------------------------------:|:------------------------------------------------------------------------------------------|
| `prefer_stack`              | `bool`                         |       `true` / **`false`** / `false`       | 堆 / 栈掷币的栈偏好（`true` 时 76% 走栈，`false` 时 40% 走栈）                            |
| `force_heap`                | `bool` / `Some(bool)` / `None` |        `None` / **`None`** / `None`        | 主容器强制路由：`true` 强制堆、`false` 强制栈、`None` 继承特性 / 默认启发式；仅主容器生效 |
| `max_latency`               | `u8`（≥ 2）                    |          `30` / **`80`** / `100`           | 单层加密原语与存储策略的延迟上限                                                          |
| `min_security`              | `u8`（≤ 99）                   |           `7` / **`63`** / `93`            | 组合安全度下限（乘法累积，达到即停）                                                      |
| `security_factor`           | `f64`（> 0.0 的有限值）        |         `1.0` / **`1.0`** / `1.0`          | 单层安全度缩放系数：`< 1.0` 折减单层贡献（叠加层数更多），`> 1.0` 放大（层数更少）        |
| `wrap_pct`                  | `0..=100`                      |           `8` / **`50`** / `85`            | 逐层与逐块非内联外包基础概率（随连续内联次数指数递增）                                    |
| `junk_pct`                  | `0..=100`                      |           `2` / **`18`** / `35`            | 拟真垃圾块伴生填充概率                                                                    |
| `cold_pct`                  | `0..=100`                      |           `10` / **`20`** / `50`           | 外包函数附加 `#[cold]` 属性（冷热段剥离）的概率                                           |
| `pad_short`                 | `bool`                         |       `false` / **`true`** / `true`        | 短块（< 16B）强制前置 `pkcs7_16` 预填充（不计入层数与安全度预算）                         |
| `anti_disasm_pct`           | `0..=100`                      |           `5` / **`40`** / `80`            | 反反汇编花指令注入概率（需启用 `anti-disassembly` 特性）                                  |
| `anti_disasm_count_range`   | `lo..=hi`（`u8`）              |      `1..=1` / **`1..=2`** / `2..=3`       | 单次注入的反反汇编块数随机区间                                                            |
| `anti_disasm_garbage_range` | `lo..=hi`（`u8`）              |     `1..=5` / **`4..=10`** / `8..=15`      | 反反汇编垃圾字节基准随机区间                                                              |

- 键支持 `key: val` 与 `key = val` 两种写法，逗号分隔，配置块内与载荷后均允许尾随逗号；
- `force_heap` 接受 `true` / `false` / `Some(true)` / `Some(false)` / `None`；`false` 表示强制走栈，而非“关闭强制堆”。优先级：`*_user!` 配置 > Cargo 特性（`force_heap` / `force_stack`）> 默认启发式；分块中间容器始终按预算与 `prefer_stack` 启发式路由，未启用 `alloc` 时 `true` / `Some(true)` 会在编译期被拒绝。

### 动态密钥档 `*_key!`

在对应档位宏基础上增加一对密钥参数，载荷加密与解密密钥分离：

```rust
use obfstr2::s_key;

// 从服务器获取密钥，如果密钥错误将触发 UB 或得到乱码。
fn get_server_key() -> &'static [u8] {
    "obfstr2-demo-key".as_bytes()
}

fn main() {
    // build_key 只参与编译期加密，不会以明文写入产物
    let s = s_key!(
        { build_key: "obfstr2-demo-key", run_key: get_server_key() },
        "hello",
    );
    assert_eq!(&*s, "hello");
}
```

- `build_key` 支持 `"..."`、`b"..."`、`[0x12, 0x34, ...]` 与 `env!("VAR")`（编译期读取并自动注册 Cargo 环境变量依赖）；
- `run_key` 必须是运行期表达式（`&[u8]`），字面量 / `as_bytes()` / `env!` 等编译期常量形式会被拒绝；
- `run_key` 必须与 `build_key` 逐字节一致；不一致时输出未定义，并可能触发解密链的完整性擦除（**有意的对抗设计**，用于污染模拟 / 符号 / 切片执行状态）；

### 轻量辅助原语

除原生混淆宏外，根 crate 还重导出了 `lib-unknown` 的轻量常量工具，适合**非敏感**常量与热路径降开销场景：

| 项目               | 说明                                                                                        |
|:-------------------|:--------------------------------------------------------------------------------------------|
| `const_random!`    | 编译期伪随机整数，支持 `()`（`u64`）、`(Type)`、`(Type, "salt")`；可用于 `const` / `static` |
| `hide_array!`      | XOR 掩码数组隐藏，`const fn` 可用，求值即还原                                               |
| `hide_array_lazy!` | 以 `LazyLock` 包裹 `hide_array!`，首次访问才还原                                            |
| `lazy`             | `no_std` 版 `LazyLock` / `OnceLock`                                                         |

```rust
use std::hint::black_box;
use obfstr2::{const_random, hide_array, hide_array_lazy, s_user};
use obfstr2::types::str::StackStr;
use obfstr2::lazy::LazyLock;

const SALT: u64 = const_random!(u64, "demo");
static TABLE: LazyLock<[u32; 4]> = hide_array_lazy!(u32, [1, 2, 3, 4]);
// 主要通过 _user 档显式关闭 force_heap 以固定类型为 StackStr，便于放入 LazyLock
static STR: LazyLock<StackStr<70>> = LazyLock::new(|| s_user!({ force_heap: false }, "hello_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"));

fn hot() {
    for _ in 0..black_box(100) {
        println!("{}", &*STR)
    }
}

fn main() {
    hot();
    let [a, b, c] = hide_array!(u8, [1, 2, 3]);
    assert_eq!((a, b, c), (1, 2, 3));
    assert_eq!(TABLE[0], 1);
    let _ = SALT;
}
```

> **强度提示：** 上述原语仅做简单掩码与 `black_box` 还原，防护强度远低于 obfstr2 原生多态加密宏。密钥、URL、系统调用名等敏感常量请始终使用 `s1!` ~ `s3!` 等原生宏。

---

## 特性标志

| Feature                   | 默认 | 说明                                                                                                                  |
|:--------------------------|:----:|:----------------------------------------------------------------------------------------------------------------------|
| `default`                 |  ✅  | 默认启用 `alloc` 与 `anti-disassembly`。                                                                              |
| `alloc`                   |  ✅  | 启用堆内存容器（`Heap*`）支持及 `s_fmt*!` 宏。支持 `no_std + alloc` 环境。                                            |
| `anti-disassembly`        |  ✅  | 启用 x86 / x86_64 反反汇编花指令内联汇编注入，并开启红区（Red-Zone）安全保护。对体积或运行时极度敏感的场景可关闭。    |
| `force_heap`              |  ❌  | 强制各容器宏的主容器走堆（隐含启用 `alloc`）；与 `force_stack` 互斥，不可同时启用。                                   |
| `force_stack`             |  ❌  | 强制各容器宏的主容器走栈；分块中间容器仍可走堆；与 `force_heap` 互斥，主容器超出 `STACK_BUDGET` 时编译期报错。        |
| `budget-1k` ~ `budget-8m` |  ❌  | 栈预算上限控制：可选 `1k`, `4k`（默认）, `16k`, `64k`, `256k`, `512k`, `1m`, `4m`, `8m`。同时启用多个时取最大档生效。 |

> `force_heap` 与 `force_stack` 互斥，`--all-features` 会同时启用二者并在编译期报错；请使用显式特性组合（参见[贡献与开发验证](#贡献与开发验证)）。

**纯栈裸机模式（无 `alloc`、无花指令注入）：**
```toml
[dependencies]
obfstr2 = { version = "0.5", default-features = false }
```

*注：关闭 `alloc` 后所有档位恒使用栈容器（`Stack*`）；若需保留 `alloc` 但强制主容器走栈，可启用 `force_stack`（分块中间容器仍可能走堆）。若主容器总大小达到 `STACK_BUDGET` 或单块解密峰值体积达到 `STACK_BUDGET / 2`，将在编译期直接报错提示调大 `budget-*`。*

---

## 性能基准

在短小载荷（如下表 1 KiB 样本）下，`b1!` 几乎零运行时开销；`b2!` 与 `b3!` 随抗分析强度提升，运行时开销与编译耗时相应增加。而在面对 **MB 级超大载荷**时，得益于原生过程宏与多叉折叠树设计，`obfstr2` 的编译速度反而显著快于依赖 `const` 求值的传统方案。请根据业务对“运行时性能”与“抗逆向强度”的侧重选择档位。

以下为 1 KiB 载荷、512 次解密循环的单次实测样本（多态构建每次结果略有浮动，仅供量级参考）：

| 用例              | 每次去混淆耗时 | 运行耗时（512 次解密循环，含启动开销） | 增量构建耗时          | 冷构建耗时            | 产物二进制（未 strip） | expand 字符数    |
|:------------------|:---------------|:---------------------------------------|:----------------------|:----------------------|:-----------------------|:-----------------|
| 明文基线          | —              | 10.30702ms                             | 171.832628ms          | 404.063369ms          | 445264                 | 1437             |
| CasualX obfbytes! | 0.10µs         | 10.360403ms（1%）                      | 202.252557ms（18%）   | 1.280738746s（217%）  | 448304（1%）           | 7780（441%）     |
| obfstr2 b1!       | 19.75µs        | 20.419212ms（98%）                     | 353.501118ms（106%）  | 4.838922938s（1098%） | 500664（12%）          | 77153（5269%）   |
| obfstr2 b2!       | 317.91µs       | 173.078567ms（1579%）                  | 888.272897ms（417%）  | 4.005122498s（891%）  | 612816（38%）          | 224527（15525%） |
| obfstr2 b3!       | 829.70µs       | 435.110996ms（4122%）                  | 2.436355796s（1318%） | 5.346699777s（1223%） | 838360（88%）          | 630688（43789%） |

<details>
<summary>点击查看基准测试参数与复现方式</summary>

数据源自 `tests/perf.rs`，可通过 `cargo test --test perf -- --ignored --nocapture` 复跑。
- **测试参数**：`PAYLOAD_LEN = 1024`（全 `'a'`），`LOOPS = 512`（解密 + 校验和累加，返回值经 `black_box` 防止循环外提），`profile = release`，每组重复 3 次取中位数。
- **单次去混淆耗时**：`(当前行耗时 - 基线耗时) / 512`，抵消进程启动固定开销。
- **构建耗时**：冷构建为新建独立临时工程从零编译（含依赖）；增量构建为共享预热依赖后仅重写 `main.rs` 触发编译。
</details>

---

## 安全性

> **免责声明**：本项目仅供商业软件知识产权保护、防逆向工程研究、固件安全加固等合法合规场景使用，严禁用于任何恶意软件制作或非法渗透测试。

- **平台支持**：纯 Rust 实现，支持主流桌面系统与 `no_std` 裸机环境（已在 `x86_64-unknown-none` 验证；Wasm 理论支持但未纳入 CI）。
- **Unsafe 边界**：展开代码中仅在编译期已严格验证长度不变量处使用 `unwrap_unchecked` 消除运行时冗余分支，以及在容器 `Drop` 中执行 Volatile 内存覆写。
- **漏洞披露**：若发现安全隐患，请通过 [GitHub Private Vulnerability Reporting](https://github.com/AlbusPhos/obfstr2/security/advisories/new) 或发送邮件至 `owo.zzz@proton.me` 私密提交，**请勿**创建公开 Issue。

---

## 贡献与开发验证

欢迎提交 Issue 和 Pull Request！新增混淆原语或存储策略只需在 `obfstr2-macros/src/crypto.rs` 或 `storage.rs` 的静态注册表中追加表项，无需改动核心编排管线。

<details>
<summary>点击展开本地测试矩阵与 PR 检查清单</summary>

**常规测试与校准套件：**
```bash
# 基础功能与宏正确性
cargo test --test smoke --test s_fmt
cd obfstr2-macros && cargo test --lib

# 非法输入编译期拒绝测试
cargo test --test compile_fail

# 裸机 no_std 链路验证（需安装 x86_64-unknown-none target）
cargo test --test nostd

# 性能基准报告
cargo test --test perf -- --ignored --nocapture

# 存储策略与加密原语延迟校准
cd obfstr2-macros && cargo test --lib test_storages_latency -- --ignored --nocapture
cd obfstr2-macros && cargo test --lib tests::crypto::test_cryptos_latency -- --ignored --nocapture

# 自动化逆向对抗回归测试（angr 符号执行 & rust-deobf 模拟器）
# 需配置含 angr 与 rust-deobf 的 Python 虚拟环境，激活虚拟环境或显式指定 VIRTUAL_ENV：
source /path/to/.venv/bin/activate
cargo test --manifest-path obfstr2-macros/Cargo.toml --lib tests::crypto::test_cryptos_security -- --ignored --nocapture
cargo test --test angr -- main --ignored --nocapture
cargo test --test rust_deobf -- main --ignored --nocapture
# 若未激活虚拟环境，也可显式传参：
# VIRTUAL_ENV=/path/to/.venv cargo test --test rust_deobf -- main --ignored --nocapture
```

**提交前代码规范检查：**
```bash
cargo fmt --all -- --check
cd obfstr2-macros && cargo fmt --all -- --check
cargo clippy --all-targets --features alloc,anti-disassembly -- -D warnings
cargo clippy --all-targets --features force_heap,anti-disassembly -- -D warnings
cargo clippy --all-targets --features alloc,force_stack,anti-disassembly -- -D warnings
cd obfstr2-macros && cargo clippy --all-targets --features alloc,anti-disassembly -- -D warnings

# 可选：安装 cargo-hack 后运行特性幂集 smoke 测试
cargo hack test --feature-powerset \
  --include-features alloc,force_heap,force_stack,budget-4m \
  --mutually-exclusive-features force_heap,force_stack \
  --test smoke
```
</details>

---

## 开源协议

本项目基于 [MPL-2.0](./LICENSE) 协议开源。版本变更历史详见 [CHANGELOG.md](CHANGELOG.md)。