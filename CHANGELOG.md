# Changelog

本文件记录本项目所有值得关注的变更。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循[语义化版本 2.0.0](https://semver.org/lang/zh-CN/)。

> **版本号说明（0.x 阶段）**：在 1.0.0 发布之前，次版本号（`0.MINOR.0`）的变更
> 也可能包含破坏性改动，请留意标注为 **[BREAKING]** 的条目。

---

## [0.3.1] - 2026-11-11

### Added
- 多级折叠调用树：chunk 节点数超过 12 时，自底向上折叠为 `#[inline(never)]` 函数，根作用域调用数收敛到 ≤ 12，避免大载荷展开后单一函数体线性膨胀（`obfstr2-macros/src/core.rs`）

### Changed
- 分块曲线由线性递增（`2i..8i`）改为超线性递增（`base = ceil(i^1.5) × 1.3`，`6 × base..9 × base`）：显著减少 chunk 数量、展开字符数与构建耗时，单 chunk 尺寸相应增大
- 纯栈中间容器上限 `STACK_BUDGET` 由 1024B 提升至 2048B，以适配更大的单 chunk；受分块曲线与原语膨胀影响，纯栈模式可处理的载荷上限相应下降
- 存储策略适用区间收窄：`mac` 由 `6..=1024` 调整为 `8..=256`，`uuid` / `ipv6` 由 `8..=1024` 调整为 `8..=386`
- PEM 存储由 `&[&str]` 行数组改为单条连续 `&str` 常量，形成更真实的连续 PEM 文本并减少 `.rodata` 元数据
- 文档：更新 README 徽章、存储形态列表、纯栈上限（2048B）与性能基准数据，并同步 `README_en.md`

## [0.3.0] - 2026-11-10

### Added
- 七类载荷宏，每类提供低延迟 / 均衡 / 高强度三档：`s1~3!`、`b1~3!`、`f1~3!`、`i1~3!`、`fl1~3!`、`cs1~3!`、`s_fmt1~3!`；保留已废弃的 `s_fmt!`（等价 `s_fmt1!`）
- 多态混淆编排：随机分块、按安全度目标叠加加密原语、密文随机伪装存储、逐层密钥隐写、垃圾块与完整性谓词
- 加密原语注册表：`cbc_xor`、`xor_diff`、`wrapping_add`、`inv_mul`、`rot_xor`、`mse_no_ps`、`mse_no_s`、`mse`、`mse2`、`sbox_static`、`pbox_dynamic`、`sbox_dynamic`、`pkcs7_16`、`fisher_yates`、`rot_xor_f`、`xor_diff_f`、`chacha20`、`spn_2r`
- 存储伪装策略：`bytes`、`mac`、`uuid`、`ipv6`、`pem`
- `CryptoMetrics { attack_complexity, cognitive_load }` 静态细分指标，供安全度校准使用
- 运行时容器 `StackStr` / `HeapStr` / `StackBytes` / `HeapBytes` / `StackCStr` / `HeapCStr`，字符串与字节容器在 `Drop` 时执行 Volatile 擦除
- `no_std` 支持；默认 `alloc` 特性启用堆容器，关闭后为纯栈模式

### Security
- 展开代码中不出现明文 `u64` 层密钥；密钥与密文一致走多态存储隐写
- 引入现代对称密码学设计要素（SPN、动态 S 盒 / P 盒、ARX、雪崩效应）以对抗 `angr` 等符号执行工具
- 字符串 / 字节 / CStr 容器在离开作用域时执行 Volatile 擦除

[0.3.0]: https://github.com/AlbusPhos/obfstr2/releases/tag/v0.3.0
[0.3.1]: https://github.com/AlbusPhos/obfstr2/releases/tag/v0.3.1
