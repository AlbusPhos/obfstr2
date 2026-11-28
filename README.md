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

全面覆盖**字符串、C 字符串、UTF-16 宽字符串、字节数组、u32 / u64 定宽数组、外部文件、整数、浮点数及格式化字符串**。支持解引用直取裸值或存入受控容器；所有容器在离开作用域（`Drop`）时自动执行 Volatile 内存清零，防止敏感明文残留在栈或堆中。

## 目录
- [快速开始](#快速开始)
- [核心亮点与对抗验证](#核心亮点与对抗验证)
- [适用场景与设计边界](#适用场景与设计边界)
- [宏一览](#宏一览)
- [特性标志](#特性标志)
- [性能基准](#性能基准)
- [安全性](#安全性)
- [贡献与开发验证](#贡献与开发验证)
- [开源协议](#开源协议)

## 快速开始
```toml
[dependencies]
obfstr2 = "0.6"
```
所有混淆宏均以数字后缀区分强度档位：`1`（低延迟）、`2`（推荐均衡）、`3`（高强度抗逆向）；另有 [`*_user!`](#自定义强度档-_user) 自定义档与 [`*_key!`](#动态密钥档-_key) 动态密钥档。

```rust
use obfstr2::{b2, cs2, cw2, f2, fl2, i2, s2, s_fmt2, w1};
fn main() {
    // 字符串：返回自有安全容器，Drop 时自动清零
    let hello = s2!("hello");
    print!("{hello}");
    // C 字符串：解引用即 &CStr，as_ptr() 可直投底层系统调用
    let sh = cs2!(c"/bin/sh");
    assert_eq!(&*sh, c"/bin/sh");
    // 宽 C 字符串：UTF-16 码元 + 自动结尾 0u16，as_ptr() 可直投 Win32 PCWSTR
    let w = cw2!("kernel32.dll");
    assert_eq!(w[w.len() - 1], 0);
    // 宽字符串：纯 UTF-16 码元，不追加结尾 NUL，允许内部含 NUL
    let ws = w1!("a\0b");
    assert_eq!(&*ws, &[0x61u16, 0, 0x62][..]);
    // 字节串与字节数组：解引用即 &[u8]
    let b = b2!(b"abc");
    let c = b2!([0x61, 98, 99]);
    assert_eq!(&*b, &*c);
    // 标量整数与浮点：求值即得原生类型裸值，直接参与运算
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
在攻击者拥有二进制完全控制权的**白盒威胁模型**下，一旦标准算法常数（如 AES / ChaCha20 魔数）或固定内联模式被识别，即可用脚本或模拟器秒级批量还原。`obfstr2` 是一套针对**专用去混淆工具、反编译器（IDA / Ghidra F5）与符号执行引擎（angr / Z3）**深度定制的多态密码学混淆管线。

### 实战对抗验证：全面免疫专用去混淆器 (`rust-deobf`)
得益于非标多态原语管线、跨块幽灵状态耦合与调用图粉碎机制，`obfstr2` 在实测中彻底阻断了针对 Rust 深度定制的去混淆器 [**`rust-deobf`**](https://github.com/VAHPEM/rust-deobf) 的静态提取与 Unicorn 模拟执行分析：

| 混淆方案 / 工具                                                   | `strings`  |  `FLOSS`   | [`rust-deobf`](https://github.com/VAHPEM/rust-deobf) | 抗分析与去混淆评估                                                |
|:------------------------------------------------------------------|:----------:|:----------:|:----------------------------------------------------:|:------------------------------------------------------------------|
| `litcrypt` / `litcrypt2`                                          |   0 / 5    |   0 / 5    |                   **5 / 5 (全破)**                   | ❌ 静态字节异或特征明显                                           |
| `obfstr` (内联 XOR+ADD)                                           |   0 / 5    |   0 / 5    |                   **5 / 5 (全破)**                   | ❌ 内联展开模式固定，易被指令模拟还原                             |
| `cryptify` (默认密钥)                                             |   0 / 5    |   0 / 5    |                   **5 / 5 (全破)**                   | ❌ 固定解密例程与密钥布局                                         |
| `muddy` (ChaCha20-Poly1305)                                       |   0 / 5    |   0 / 5    |                   **5 / 5 (全破)**                   | ❌ 标准密码学常数与 AEAD 调用结构可被精准识别                     |
| 手写紧凑栈字符串 / XOR / ADD                                      |   0 / 5    |   0 / 5    |                   **5 / 5 (全破)**                   | ❌ 缺乏控制流屏障与跨块状态耦合                                   |
| **`obfstr2` ([`tests/rust_deobf.rs`](tests/rust_deobf.rs) 实测)** | **0 解出** | **0 解出** |                 **0 解出 (全免疫)**                  | ✅ **非标多态原语 + 跨块雪崩棘轮 + 虚表蹦床，彻底阻断自动化还原** |

*注：上表前 5 项基线数据引用自 [`rust-deobf` 官方评测基准](https://github.com/VAHPEM/rust-deobf)。*

> **持续对抗演进承诺：**
> 得益于加解密原语与隐写策略高度解耦的静态注册表架构，**开发者承诺定期更新、轮换内置密码学算法与隐写伪签名**，持续刷新代码展开形态与数据特征，让针对性提取规则持续失效。

### 1. 抗符号执行与反编译的密码学原语库
内置 16 种多态加解密原语（涵盖流密码、SPN、动态盒变换与分组密码），按延迟上限与目标安全度随机抽样叠加：
- **自研 MSE / MSE2 算法家族**：融合 CFB 密文反馈、随历史状态逐字节变异的动态 S-Box / P-Box、基于超越数（ $\pi$ 、$1/e$ ）定点近似的 128 位非线性密钥派生及 64B 分组 4 轮 ARX 扩散，消除一切标准算法魔数特征。
- **浮点跨域代数混淆**：将整数旋转、异或等价展开为 IEEE-754 浮点除法、截断与指数构造，大幅拉升 F5 伪代码认知负载，并引发 SMT 求解器在浮点非线性约束下的路径爆炸与超时。
- **动态盒求逆与模乘逆元**：编译期采用 $\pmod{2^8}$ 奇数乘法逆元加密、运行期单次乘法解密；运行期基于密钥实时洗牌生成 S-Box / P-Box 并求逆，消除二进制静态查表特征。
- **全局位置扩散**：引入基于 128 位无偏乘法区间映射的 Fisher-Yates 置换，打破字节边界对齐。

### 2. 逆向密钥棘轮与全链路雪崩耦合
拒绝在二进制中暴露每层的独立密钥，构建贯穿“编译期正向加密”与“运行期逆序解密”的双向状态耦合链：
- **零额外密钥存储（ $\pmod{2^{64}}$ 乘法逆元棘轮）**：每个分块仅存储唯一根密钥（经入口 `ghost_state` 掩码）。编译期利用 64 位奇数乘法逆元结合当前层尾字节逆向推演外层密钥；运行期通过尾字节正向还原下一层密钥：
  $$K _ {\text{next}} = (K _ {\text{cur}} + \text{ghost\\_state}) \times (\text{last\\_byte} \mid 1) \pmod{2^{64}}$$
- **跨块幽灵状态（`ghost_state`）**：分块解密拼入主容器后，提取主容器末尾最多 16B 滑动窗口计算 `LightHasher` 哈希并异或更新全局 `ghost_state`（动态密钥模式下混入密钥哈希）。任何跳过前置分块、内存单字节 Patch 或抠取单函数模拟执行的行为，均会引发后续密钥链彻底雪崩。

### 3. 超线性分块、拟真垃圾块与多态代数不透明谓词
- **超线性分块与短块填充**：按基数 $\text{base} = \lceil i^{2.4} \times 0.3 \rceil$、区间 $\text{base} \ldots \text{base} \times \max(1,\ (\text{remaining} \bmod 3 + 1) + \lfloor i/5 \rfloor)$ 随机切分载荷（头部细碎打散特征，尾部块尺寸快速增长摊薄开销）；中高强度档对 `< 16B` 短块强制前置 `pkcs7_16` 填充，消除短字符串长度侧信道。
- **100% 拟真垃圾块**：按概率混入伪造数据块，经历完全相同的多层加密、存储与运行期解密流程，在反编译视图中真假难辨。
- **多态代数与数论不透明谓词**：分块解密末尾随机注入 4 类完整性校验——**非线性 MBA 恒等式与 $\pmod{2^{64}}$ 逆元对偶**、**汉明重量 `popcnt` 容斥原理与正交位拆分**、**2-adic 环 Hensel 奇数幂不动点（ $v^2 \equiv 1 \pmod 8$、$v^4 \equiv 1 \pmod{16}$ ）** 及 **德摩根对称差双侧 MBA**。校验式绑定缓冲区地址与 `ghost_state`，垃圾块与被篡改分块必然触发 `wipe_data()` 自擦除。

### 4. 调用图粉碎、花指令注入与编译器屏障
- **自适应外包与虚表蹦床（Vtable Trampoline）**：在“单层”与“分块”双粒度上，外包概率随连续内联次数指数递增；以 50% 概率生成 `#[inline(never)]` 函数或 `&mut dyn FnMut` 动态派发蹦床，将直接调用转化为虚表间接调用，斩断静态调用图（Xrefs）。
- **O(log M) 多叉折叠树与 MB 级极速编译**：将相邻 2~6 个块节点递归封装为多级调用树，结合原生过程宏求值（免 `#![allow(long_running_const_eval)]`），避免巨型函数引发 LLVM 寄存器分配阶段 $O(V^2)$ 复杂度爆炸，以近似线性时间极速编译 MB 级大载荷。
- **冷热段剥离与反跨过程优化（Anti-IPO）**：按概率附加 `#[cold]` 将解密逻辑驱逐至 `.text.unlikely` 段；将函数指针转为 `usize` 投入 `black_box` 伪造地址逃逸，阻止 LLVM 跨过程常量传播。
- **反反汇编花指令注入**：在函数与解密片段首尾随机注入零数据依赖的 x86/x86_64 架构门控内联汇编（花指令），干扰线性反汇编与指令模式识别。

### 5. 多态隐写存储与受控内存生命周期
- **密文与密钥多态伪装**：密文与初始密钥随机编码为普通字节流、`u64`/`u128` 数组、MAC、UUID、IPv6、PEM 证书或 HB2 空白字符隐写（嵌入随机拟真代码/日志语料与动态空白符号表），规避高熵扫描。
- **全链路 Volatile 擦除**：最终容器在 `Drop` 时及变长解密层的中间临时容器在交接后，均立即执行整段容量的 Volatile 清零，杜绝明文或中间态残留。

---

## 适用场景与设计边界

| ✅ 推荐使用场景                                                                                                                                                                                                                                                              | ❌ 不适用场景与设计取舍                                                                                                                                                                                                                                                                                           |
|:-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| • **核心资产保护**：商业软件 DRM、许可证校验常数、内置公钥 / API Key<br>• **反作弊与安全 SDK**：隐藏敏感系统调用与模块名（如 `ntdll.dll`）<br>• **裸机与嵌入式固件**：`no_std` 环境下隐藏硬编码凭证与协议魔数<br>• **大型资产内嵌**：混淆 MB 级二进制固件/资源且要求快速编译 | • **可复现构建 (Reproducible Builds)**：多态引擎每次编译均生成不同二进制特征，产物哈希不固定<br>• **体积极端受限且高频热路径**：级联解密与垃圾块会带来必要的代码膨胀与运行开销（热路径请配 `LazyLock` 或用 `1` 档）<br>• **传输与持久化加密**：白盒混淆旨在抬高逆向成本，不替代标准密码学库用于网络传输或磁盘存储 |

---

## 宏一览
所有宏均提供 `1`（低延迟）、`2`（均衡）、`3`（高强度）三个档位。**所有容器类型（`Stack*` / `Heap*`）在 `Drop` 时均自动执行 Volatile 清零；标量类型（`i*` / `fl*`）求值直接返回原生裸值：**

| 宏族                    | 输入字面量示例                                           | 返回类型（解引用目标）                                         | 特性与约束说明                                                                                       |
|:------------------------|:---------------------------------------------------------|:---------------------------------------------------------------|:-----------------------------------------------------------------------------------------------------|
| `s1!` ~ `s3!`           | `"hello"`                                                | `StackStr<N>` / `HeapStr<N>`（`Deref<Target = str>`）          | UTF-8 字符串容器。                                                                                   |
| `b1!` ~ `b3!`           | `b"abc"` 或 `[0x41, 66]`                                 | `StackBytes<N>` / `HeapBytes<N>`（`Deref<Target = [u8]>`）     | 数组元素范围 `0..=255`。                                                                             |
| `d1!` ~ `d3!`           | `[0xDEAD_BEEF, 1u32]`                                    | `StackU32<N>` / `HeapU32<N>`（`Deref<Target = [u32]>`）        | `u32` 整数字面量数组；按小端还原，`N` 为元素个数。                                                   |
| `q1!` ~ `q3!`           | `[0x0123_4567_89AB_CDEF, 1u64]`                          | `StackU64<N>` / `HeapU64<N>`（`Deref<Target = [u64]>`）        | `u64` 整数字面量数组；按小端还原，`N` 为元素个数。                                                   |
| `cs1!` ~ `cs3!`         | `c"..."` / `"..."` / `b"..."`                            | `StackCStr<N>` / `HeapCStr<N>`（`Deref<Target = CStr>`）       | 自动追加结尾 `\0`，运行时零拷贝还原；非 UTF-8 请用 `b"..."`；**严禁内部含 NUL**。                    |
| `w1!` ~ `w3!`           | `"a\0b"`                                                 | `StackU16<N>` / `HeapU16<N>`（`Deref<Target = [u16]>`）        | 编译期转 UTF-16；**不追加**结尾 NUL，**允许**内部含 NUL（适用于长度前缀 API）。                      |
| `cw1!` ~ `cw3!`         | `"kernel32.dll"`                                         | `StackU16<N>` / `HeapU16<N>`（`Deref<Target = [u16]>`）        | 编译期转 UTF-16 并**自动追加结尾 `0u16`**；**严禁内部含 NUL**；`as_ptr()` 可直投 Win32 `PCWSTR`。    |
| `f1!` ~ `f3!`           | `"assets/data.bin"`                                      | `StackBytes<N>` / `HeapBytes<N>`（`Deref<Target = [u8]>`）     | 路径相对被编译 crate 的 `CARGO_MANIFEST_DIR`。                                                       |
| `i1!` ~ `i3!`           | `42u32` / `-1` / `0xFFu16`                               | 原生整数裸值（`u8`~`i128` / `usize`）                          | 空后缀默认为 `i32`；`usize`/`isize` 按 64 位语义编码。                                               |
| `fl1!` ~ `fl3!`         | `3.15f32` / `-1.0` / `1e10`                              | 原生浮点裸值（`f32` / `f64`）                                  | 空后缀默认为 `f64`；拒绝 `inf`/`NaN`；保留 `-0.0` 符号位（断言请用 `to_bits()`）。                   |
| `s_fmt1!` ~ `s_fmt3!`   | `"hi, {}!", name`                                        | `alloc::string::String`                                        | 仅混淆格式串中的静态字面量片段，随后走 `format!`；依赖 `alloc` 特性。                                |
| `*_user!`（全家族）     | `{ max_latency: 50, ... }, 载荷`                         | 同对应同族宏                                                   | 自定义档：以均衡档（`2`）为基底逐字段覆盖参数。                                                      |
| `*_key!`（全家族）      | `{ build_key, run_key }, 载荷`                           | 同对应同族宏                                                   | 动态密钥档：编译期 `build_key` 加密、运行期 `run_key` 解密。                                         |
| `*_key_user!`（全家族） | `{ build_key, run_key }, { max_latency: 50, ... }, 载荷` | 同对应同族宏                                                   | 同时配置动态密钥与自定义档参数。                                                                     |

> **堆栈路由与类型推断提示：**
> 开启 `alloc`（默认）时，载荷 $\le 16\text{B}$ 恒走栈，达到预算上限（主容器 `STACK_BUDGET` / 分块中间容器 `STACK_BUDGET / 2`）恒走堆，其间按档位偏好概率随机路由。可通过特性 `force_heap` / `force_stack` 或配置项 `force_heap` 强制**主容器**路由（优先级：`*_user!` 配置 > Cargo 特性 > 默认启发式；分块中间容器始终按预算与启发式路由）。主容器或单块峰值超出栈预算且无法走堆时将在编译期报错。由于**返回的具体类型（`Stack*` 或 `Heap*`）每次编译可能动态变化**，请始终使用 `let val = s2!("...");` 自动推断，切勿写死类型标注。

### 自定义强度档 `*_user!`
首参数为配置块 `{ key: val, ... }`（支持 `:` 或 `=`），以**均衡档（`2`）为基底**逐字段覆盖。加密选层遵循互补概率累积模型，随机抽样叠加直至组合安全度达标：
$$\text{Security} _ {\text{combined}} = 1 - \prod _ {i=1}^{k} \left(1 - \frac{\text{Security} _ i}{100}\right) \ge \text{min\\_security}$$

```rust
use obfstr2::{cs_user, s_user};
fn main() {
    let greeting = s_user!(
        {
            prefer_stack: true,      // 堆 / 栈掷币的栈偏好
            max_latency: 60,         // 单层延迟上限（>= 2）
            min_security: 75,        // 组合安全度下限（<= 99）
            junk_pct: 12,            // 垃圾块填充概率（0..=100）
            pad_short: true,         // 短块（< 16B）强制 PKCS7 预填充
        },
        "custom-tier",
    );
    assert_eq!(&*greeting, "custom-tier");
    assert_eq!(&*s_user!({}, "hello"), "hello"); // 空配置块等价于 s2!
    assert_eq!(cs_user!({ max_latency: 40 }, c"/bin/sh").to_str().ok(), Some("/bin/sh"));
}
```

| 配置键                      | 取值                           | 预设档位默认值（`1` / **`2` 基底** / `3`） | 说明                                                                                              |
|:----------------------------|:-------------------------------|:------------------------------------------:|:--------------------------------------------------------------------------------------------------|
| `prefer_stack`              | `bool`                         |       `true` / **`false`** / `false`       | 堆 / 栈掷币的栈偏好（`true` 时 76% 走栈，`false` 时 40% 走栈）                                    |
| `force_heap`                | `bool` / `Some(bool)` / `None` |        `None` / **`None`** / `None`        | 主容器强制路由：`true` 强制堆（需 `alloc`）、`false` 强制栈、`None` 继承特性/默认启发式           |
| `max_latency`               | `u8`（≥ 2）                    |          `36` / **`80`** / `100`           | 单层加密原语与存储策略的延迟上限                                                                  |
| `min_security`              | `u8`（≤ 99）                   |           `30` / **`63`** / `93`           | 组合安全度下限（乘法累积，达到即停）                                                              |
| `security_factor`           | `f64`（> 0.0 有限值）          |         `1.0` / **`1.0`** / `1.0`          | 单层安全度缩放系数：`< 1.0` 折减单层贡献（叠加层数更多），`> 1.0` 放大（层数更少）                |
| `wrap_pct`                  | `0..=100`                      |           `30` / **`60`** / `85`           | 逐层与逐块非内联外包基础概率（随连续内联次数指数递增）                                            |
| `junk_pct`                  | `0..=100`                      |           `8` / **`18`** / `35`            | 拟真垃圾块伴生填充概率                                                                            |
| `cold_pct`                  | `0..=100`                      |           `10` / **`20`** / `50`           | 外包函数附加 `#[cold]` 属性（冷热段剥离）的概率                                                   |
| `pad_short`                 | `bool`                         |       `false` / **`true`** / `true`        | 短块（`< 16B`）强制前置 `pkcs7_16` 预填充（不计入层数与安全度预算）                               |
| `anti_disasm_pct`           | `0..=100`                      |           `45` / **`60`** / `80`           | 反反汇编花指令注入概率（需启用 `anti-disassembly` 特性）                                          |
| `anti_disasm_count_range`   | `lo..=hi`（`u8`）              |      `1..=1` / **`1..=2`** / `2..=3`       | 单次注入的反反汇编块数随机区间                                                                    |
| `anti_disasm_garbage_range` | `lo..=hi`（`u8`）              |     `1..=5` / **`4..=10`** / `8..=15`      | 反反汇编垃圾字节基准随机区间                                                                      |

### 动态密钥档 `*_key!`
将编译期加密密钥与运行期解密密钥分离，二进制产物中不留存完整密钥：
```rust ,no_run
use obfstr2::s_key;

fn main() {
    // 从服务器或环境变量获取用于解密的密钥
    let key = std::env::var("OBFSTR2_TEST_KEY").expect("请设置 OBFSTR2_TEST_KEY");
    let s = s_key!(
        { build_key: env!("OBFSTR2_TEST_KEY"), run_key: key.as_bytes() },
        "hello",
    );
    assert_eq!(&*s, "hello");
}
```
- `build_key` 支持 `"..."`、`b"..."`、`[0x12, ...]` 与 `env!("VAR")`（自动注册 Cargo 环境变量依赖），不会写入产物；
- `run_key` 必须为运行期 `&[u8]` 表达式（字面量、`as_bytes()`、`env!` 等常量形式会在编译期被拒绝）；若运行期密钥与 `build_key` 不一致，将输出乱码或触发完整性自擦除（用于污染模拟与符号执行状态）。

### 轻量辅助原语
重导出自 `lib-unknown` 的轻量工具（仅做简单 XOR 掩码与 `black_box` 还原，适合**非敏感常量**或配合 `LazyLock` 缓存热路径）：

| 原语                               | 说明                                                                                      |
|:-----------------------------------|:------------------------------------------------------------------------------------------|
| `const_random!`                    | 编译期伪随机整数，支持 `()`（`u64`）、`(Type)`、`(Type, "salt")`；可用于 `const`/`static` |
| `hide_array!` / `hide_array_lazy!` | 编译期 XOR 掩码数组隐藏（求值即还原 / `LazyLock` 首次访问还原）                           |
| `lazy::LazyLock` / `OnceLock`      | `no_std` 兼容的惰性初始化容器                                                             |

```rust
use obfstr2::{const_random, hide_array, hide_array_lazy, s_user, lazy::LazyLock, types::str::StackStr};

const SALT: u64 = const_random!(u64, "demo");
static TABLE: LazyLock<[u32; 4]> = hide_array_lazy!(u32, [1, 2, 3, 4]);
// 热路径缓存示例：通过 force_heap: false 固定为 StackStr 类型以存入 static LazyLock
static HOT_STR: LazyLock<StackStr<16>> = LazyLock::new(|| s_user!({ force_heap: false }, "hot_path_secret!"));

fn main() {
    assert_eq!(&**HOT_STR, "hot_path_secret!");
    assert_eq!(hide_array!(u8, [1, 2, 3]), [1, 2, 3]);
    assert_eq!(TABLE[0], 1);
    let _ = SALT;
}
```

---

## 特性标志

| Feature                   | 默认 | 说明                                                                                                                  |
|:--------------------------|:----:|:----------------------------------------------------------------------------------------------------------------------|
| `default`                 |  ✅  | 默认启用 `alloc` 与 `anti-disassembly`。                                                                              |
| `alloc`                   |  ✅  | 启用堆内存容器（`Heap*`）及 `s_fmt*!` 宏（支持 `no_std + alloc`）。关闭后全档位恒使用 `Stack*`。                      |
| `anti-disassembly`        |  ✅  | 启用 x86 / x86_64 反反汇编花指令内联汇编注入与红区（Red-Zone）保护。                                                  |
| `force_heap`              |  ❌  | 强制各容器宏的主容器走堆（隐含启用 `alloc`）；与 `force_stack` 互斥。                                                 |
| `force_stack`             |  ❌  | 强制各容器宏的主容器走栈（超 `STACK_BUDGET` 编译期报错）；与 `force_heap` 互斥。                                      |
| `budget-1k` ~ `budget-8m` |  ❌  | 栈预算上限控制：可选 `1k`, `4k`（默认）, `16k`, `64k`, `256k`, `512k`, `1m`, `4m`, `8m`（多选时取最大档）。           |

> **纯栈裸机模式（无 `alloc`、无花指令）：** `obfstr2 = { version = "0.6", default-features = false }`
>
> *注意：`force_heap` 与 `force_stack` 互斥，测试时请勿直接使用 `--all-features`（参见[贡献与开发验证](#贡献与开发验证)）。*

---

## 性能基准
- **短小载荷（8 B ~ 1 KB）**：`b1!` 在 8B 档近乎零运行时开销、1KB 档约为百微秒级，适合配合 `LazyLock` 用于热路径；`b2!` 与 `b3!` 随抗分析强度提升，运行时开销与编译耗时相应增加。
- **超大载荷（256 KB ~ 10 MB）**：控制流开销随载荷增大被超线性分块快速摊薄，`obfstr2` 在**编译速度、产物体积与宏展开代码量上全面反超传统 `const` 方案**——10 MB 载荷下，`b1!` / `b2!` / `b3!` 的构建耗时仅为 `CasualX obfbytes!`（230.25s）的 **1/33、1/16 与 1/7**，且产物二进制体积膨胀率收敛至 **9% ~ 41%**（远低于后者的 96%）。

|      Payload | 用例              | 每次去混淆耗时 |       缓存构建耗时 | 产物二进制（未 strip） |    expand 字符数 |
|-------------:|:------------------|---------------:|-------------------:|-----------------------:|-----------------:|
|      **8 B** | 明文基线          |              — |          245.04 ms |               475.8 KB |              698 |
|           〃 | CasualX obfbytes! |        0.08 µs |   232.13 ms（-5%） |         476.4 KB（0%） |     3192（357%） |
|           〃 | obfstr2 b1!       |        0.00 µs |   343.12 ms（40%） |        523.9 KB（10%） |   20800（2880%） |
|           〃 | obfstr2 b2!       |       39.38 µs |   383.59 ms（57%） |        548.8 KB（15%） |   49105（6935%） |
|           〃 | obfstr2 b3!       |      157.72 µs |  977.21 ms（299%） |        635.9 KB（34%） |  82500（11719%） |
|     **32 B** | 明文基线          |              — |          212.28 ms |               475.8 KB |              723 |
|           〃 | CasualX obfbytes! |        0.03 µs |   242.04 ms（14%） |         476.7 KB（0%） |     3524（387%） |
|           〃 | obfstr2 b1!       |        0.10 µs |   413.66 ms（95%） |        534.4 KB（12%） |   57262（7820%） |
|           〃 | obfstr2 b2!       |      137.82 µs |  787.11 ms（271%） |        602.1 KB（27%） | 156729（21578%） |
|           〃 | obfstr2 b3!       |      216.11 µs |     1.13 s（432%） |        676.4 KB（42%） | 227905（31422%） |
|    **256 B** | 明文基线          |              — |          212.25 ms |               475.8 KB |              948 |
|           〃 | CasualX obfbytes! |        0.01 µs |   252.63 ms（19%） |         477.9 KB（0%） |     4644（390%） |
|           〃 | obfstr2 b1!       |       19.82 µs |  515.39 ms（143%） |        571.0 KB（20%） |   68721（7149%） |
|           〃 | obfstr2 b2!       |       99.06 µs |  878.01 ms（314%） |        622.4 KB（31%） | 213288（22399%） |
|           〃 | obfstr2 b3!       |      453.77 µs |     1.52 s（618%） |        761.2 KB（60%） | 458211（48234%） |
|   **1.0 KB** | 明文基线          |              — |          221.99 ms |               475.8 KB |             1717 |
|           〃 | CasualX obfbytes! |        0.00 µs |   262.32 ms（18%） |         478.7 KB（1%） |     8484（394%） |
|           〃 | obfstr2 b1!       |      118.14 µs |  605.60 ms（173%） |        591.6 KB（24%） |  161442（9303%） |
|           〃 | obfstr2 b2!       |      374.51 µs |     1.28 s（478%） |        682.2 KB（43%） | 312530（18102%） |
|           〃 | obfstr2 b3!       |      826.93 µs |     1.90 s（754%） |        810.1 KB（70%） | 657826（38213%） |
| **256.0 KB** | 明文基线          |              — |          214.49 ms |               731.9 KB |           262838 |
|           〃 | CasualX obfbytes! |        0.00 µs |    5.08 s（2267%） |        988.8 KB（35%） |  1314083（400%） |
|           〃 | obfstr2 b1!       |       28.42 ms |     1.91 s（790%） |         1.09 MB（53%） |  1373076（422%） |
|           〃 | obfstr2 b2!       |       44.56 ms |    3.84 s（1689%） |         1.35 MB（90%） |  1810578（589%） |
|           〃 | obfstr2 b3!       |      121.89 ms |    8.19 s（3718%） |        2.08 MB（191%） | 3988707（1418%） |
|  **1.00 MB** | 明文基线          |              — |          242.33 ms |                1.46 MB |          1049271 |
|           〃 | CasualX obfbytes! |        7.00 µs |   19.94 s（8130%） |         2.47 MB（68%） |  5246243（400%） |
|           〃 | obfstr2 b1!       |      116.08 ms |    3.05 s（1157%） |         1.98 MB（35%） |  3987224（280%） |
|           〃 | obfstr2 b2!       |      211.41 ms |    5.48 s（2162%） |         2.31 MB（58%） |  4596155（338%） |
|           〃 | obfstr2 b3!       |      501.65 ms |   11.12 s（4487%） |        3.42 MB（134%） |  7137882（580%） |
| **10.00 MB** | 明文基线          |              — |          504.14 ms |               10.46 MB |         10486455 |
|           〃 | CasualX obfbytes! |        5.07 ms | 230.25 s（45573%） |        20.47 MB（96%） | 52432162（400%） |
|           〃 | obfstr2 b1!       |         1.21 s |    6.84 s（1256%） |         11.39 MB（9%） | 31275657（198%） |
|           〃 | obfstr2 b2!       |         1.77 s |   14.33 s（2742%） |        12.33 MB（18%） | 36207718（245%） |
|           〃 | obfstr2 b3!       |         4.60 s |   31.55 s（6158%） |        14.75 MB（41%） | 45779509（337%） |

<details>
<summary>点击查看基准测试参数与复现方式</summary>

数据源自 `tests/perf.rs`，可通过 `cargo test --test perf -- --ignored --nocapture` 复跑（单次多态构建结果略有浮动，仅供量级参考）。
- **测试参数**：覆盖 `8 B` ~ `10.00 MB` 7 档载荷梯度，`profile = release`；解密返回值与校验和经 `black_box` 防止循环外提与死代码消除，每组重复测量取中位数。
- **每次去混淆耗时**：`(当前行总耗时 - 明文基线耗时) / 循环次数`，抵消进程启动与外层循环固定开销。
- **缓存构建耗时**：全局依赖预热（Warm-up）后，共享构建缓存仅重写目标源码触发增量编译的耗时。
</details>

---

## 安全性
> **免责声明**：本项目仅供商业软件知识产权保护、防逆向工程研究、固件安全加固等合法合规场景使用，严禁用于任何恶意软件制作或非法渗透测试。

- **平台支持**：纯 Rust 实现，支持主流桌面系统与 `no_std` 裸机环境（已在 `x86_64-unknown-none` 验证；Wasm 理论支持但未纳入 CI）。
- **Unsafe 边界**：过程宏在编译期已完成全部长度与对齐不变量验证，运行时仅在已验证处使用 `unwrap_unchecked` 消除冗余分支，并在容器 `Drop` 中执行 Volatile 内存覆写。
- **漏洞披露**：若发现安全隐患，请通过 [GitHub Private Vulnerability Reporting](https://github.com/AlbusPhos/obfstr2/security/advisories/new) 或发送邮件至 `owo.zzz@proton.me` 私密提交，**请勿**创建公开 Issue。

---

## 贡献与开发验证
欢迎提交 Issue 和 Pull Request！新增混淆原语或存储策略只需在 `obfstr2-macros/src/crypto.rs` 或 `storage.rs` 的静态注册表中追加表项，无需改动核心编排管线。

<details>
<summary>点击展开本地测试矩阵与 PR 检查清单</summary>

**常规测试与校准套件：**
```bash
# 动态密钥档测试需设置 OBFSTR2_TEST_KEY：build_key 在编译期读取，
# run_key 在运行期读取，二者必须一致。
export OBFSTR2_TEST_KEY='obfstr2-test-key-0123456789abcdef'

# 基础功能、编译期拒绝与裸机 no_std 验证（需安装 x86_64-unknown-none target）
cargo test --test smoke --test s_fmt --test compile_fail --test nostd
cd obfstr2-macros && cargo test --lib

# 性能基准与原语延迟校准
cargo test --test perf -- --ignored --nocapture
cd obfstr2-macros && cargo test --lib test_storages_latency -- --ignored --nocapture
cd obfstr2-macros && cargo test --lib tests::crypto::test_cryptos_latency -- --ignored --nocapture

# 自动化逆向对抗回归测试（需配置含 angr 与 rust-deobf 的 Python 虚拟环境）
source /path/to/.venv/bin/activate
cargo test --manifest-path obfstr2-macros/Cargo.toml --lib tests::crypto::test_cryptos_security -- --ignored --nocapture
cargo test --test angr -- main --ignored --nocapture
cargo test --test rust_deobf -- main --ignored --nocapture
```

**提交前代码规范检查：**
```bash
# clippy --all-targets 会编译含 env! 的动态密钥测试，需设置同名环境变量。
export OBFSTR2_TEST_KEY='obfstr2-test-key-0123456789abcdef'

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