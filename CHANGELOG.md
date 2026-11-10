# Changelog

本文件记录本项目所有值得关注的变更。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循[语义化版本 2.0.0](https://semver.org/lang/zh-CN/)。

> **版本号说明（0.x 阶段）**：在 1.0.0 发布之前，次版本号（`0.MINOR.0`）的变更
> 也可能包含破坏性改动，请留意标注为 **[BREAKING]** 的条目。

---

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
