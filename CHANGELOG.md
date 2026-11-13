# Changelog

本文件记录本项目所有值得关注的变更。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循[语义化版本 2.0.0](https://semver.org/lang/zh-CN/)。

> **版本号说明（0.x 阶段）**：在 1.0.0 发布之前，次版本号（`0.MINOR.0`）的变更
> 也可能包含破坏性改动，请留意标注为 **[BREAKING]** 的条目。

---

## [Unreleased]

## [0.4.0] - 2026-11-13

### Added
- 新增 `hb2` 空白字符隐写存储策略：编译期将密文逐字节编码为 8 个 HB2 符号（`\0` / `\n`），并随机插入约 10% 的 OpenSSL / 系统程序伪特征字符串（每条 ≤30 字节）；运行时 `hb2_decode` 宽松跳过表外文本。适用载荷 8..=8192 字节，`latency = 85`，仅参与高强度档（均衡档上限 `max_latency = 80`）
- 新增 `obfstr2-macros/src/tests/storage.rs` 的 `test_storages_latency`（取代原 `storage.rs` 内的 `test_storages` 往返测试）：对全部存储策略做多长度、多轮次（Run / Size / Build 加权）latency 校准压测，并在压测中校验解码结果与原始数据一致；同时将 `obfstr2_dep` 辅助函数抽到 `tests/mod.rs` 供 crypto / storage 测试共用

### Changed
- 依赖升级：`lib-unknown` 由 `=0.3.2` 升级至 `=0.4.1`；`inv_mul8` / `inv_mul64` 迁移至 `crypto::core::inv_mul` 子模块，并新增 `encoding-hb2` 特性依赖
- 密钥调度重构：每个 chunk 仅生成并存储一个根密钥，后续层密钥在编译期通过 64 位奇数乘法逆元 `inv_mul64` 结合上一层尾字节逆向派生（`key × inv_mul64(last | 1) - ghost_state`），运行期解密时正向还原下一层密钥，避免在二进制中为每一层单独存储密钥
- 跨块 `ghost_state` 由 `u8` 拓宽为 `u64` 并随机初始化；加密阶段引入编译期镜像 `sim_ghost` 精确追踪主容器末尾字节，使编译期正向加密与运行期逆序解密的密钥推进保持一致
- `cs1!` / `cs2!` / `cs3!` 改用 `CStr::from_inner_with_nul` 零拷贝消费解密容器，移除独立的 CStr 构造、载荷拷贝与源缓冲擦除
- `mse2` / `pkcs7_16` 解密输出改用 `Buffer::new_zeroed` 预填长度，移除为设置长度而进行的整块输入拷贝
- 移除解密容器在 `drop` 前的显式 `wipe_data()`：`Buffer::Drop` 已对整段 capacity 执行 volatile 清零（覆盖字符串、字节、CStr、宽字符串与 chunk 中间容器）
- MAC / UUID / IPv6 / PEM 存储解码直接写入目标容器的字节视图，移除 16B / 48B scratch 容器及其拷贝与擦除
- 存储策略 latency 重新校准：`mac` 10→80、`uuid` 10→68、`ipv6` 10→71、`pem` 12→66，使各策略按实测开销参与对应档位筛选；低延迟档（上限 30）自此仅保留 `bytes`、`u64`、`u128` 三种存储
- PEM 存储适用区间由 128..=10240 调整为 128..=8192
- `Storage` 注册表移除未被使用的 `security` 字段
- 文档：README 新增“核心亮点”章节，重写设计哲学、非目标、适用场景、性能与安全披露；`README_en.md` 完成全量同步并写入 i18n 锚点

### Fixed
- `build_obfuscated_bytes` 的 `stack_main_bytes` 参数更名为 `prefer_stack`，修正 `b2` 的语义反转
- `cs2!` 与字节内核统一主容器族选择（新增 `b2_forced`），避免堆/栈类型随机不一致导致的类型不匹配

## [0.3.3] - 2026-11-11

### Added
- 栈预算控制 feature：`budget-1k` / `budget-4k` / `budget-16k` / `budget-64k` / `budget-256k` / `budget-512k` / `budget-1m` / `budget-4m` / `budget-8m`，将纯栈上限 `STACK_BUDGET` 分别设为 1 KiB / 4 KiB / 16 KiB / 64 KiB / 256 KiB / 512 KiB / 1 MiB / 4 MiB / 8 MiB（`obfstr2` feature 转发至 `obfstr2-macros`；同时启用多个时取最大档，未指定时默认 4 KiB）
- 主容器选择：载荷大小达到 `STACK_BUDGET` 且堆可用时改用 `HeapBytes`，避免大载荷压栈；纯栈模式下载荷超过 `STACK_BUDGET` 且无堆可用时给出明确编译期错误

### Changed
- `STACK_BUDGET` 默认值由 2048B 提升至 4096B；chunk 中间容器走堆阈值同步调整为 `STACK_BUDGET`/2（默认 2048B）
- 分块曲线调整为 `base = ceil(i^1.4)`、取块范围 `2 × base..8 × base`，块大小分布更宽、混淆粒度更多样
- 存储策略适用区间调整（`obfstr2-macros/src/storage.rs`）：`mac` 由 `8..=256` 调整为 `6..=386`；`uuid` / `ipv6` 由 `8..=386` 调整为 `8..=512`；`pem` 由 `128..=4096` 调整为 `128..=10240`
- 文档：README / README_en 特性标志表增加 `budget-*`，纯栈上限说明改为随 `STACK_BUDGET` 联动；性能基准数据更新为最新实测（含 `mac` / `uuid` / `ipv6` / `pem` 区间调整后的 b1 / b2 / b3 数据）

## [0.3.2] - 2026-11-11

### Added
- `w1!` / `w2!` / `w3!` UTF-16 宽字符串混淆：编译期经 `encode_utf16` 转换，**不追加**结尾 `0u16`、**允许内部 NUL**；返回 `StackU16<N>` / `HeapU16<N>`（`N` 为码元数，空串为 0），适合显式长度 / 长度前缀 API；`w1` 恒纯栈，`w2` / `w3` 按档位与 `alloc` 选择容器
- `cw1!` / `cw2!` / `cw3!` UTF-16 宽 C 字符串混淆：编译期经 `encode_utf16` 转换并自动追加唯一结尾 `0u16`，拒绝内部 NUL；返回 `StackU16<N>` / `HeapU16<N>`（`N` 含结尾码元），`as_ptr()` 可直接用于 Win32 `PCWSTR`；`cw1` 恒纯栈，`cw2` / `cw3` 按档位与 `alloc` 选择容器
- 文档：README / README_en 增加宽字符串、宽 C 字符串宏、容器类型与示例

### Changed
- 解析层：统一解包 `macro_rules!` 元变量产生的不可见 `None` 分组，`iN!` / `flN!` / `csN!` / `bN!` 现在也能被用户宏转发；浮点解析由 4 个分支合并为 2 个；负整数改用 `u128` 绝对值 + `wrapping_neg` 简化边界处理；`bN!` 改为单次 `syn::Expr` 解析并改进误传字符串时的错误文案

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
- 七类载荷宏，每类提供低延迟 / 均衡 / 高强度三档：`s1~3!`、`b1~3!`、`f1~3!`、`i1~3!`、`fl1~3!`、`cs1~3!`、`s_fmt1~3!`
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

[Unreleased]: https://github.com/AlbusPhos/obfstr2/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/AlbusPhos/obfstr2/compare/v0.3.3...v0.4.0
[0.3.3]: https://github.com/AlbusPhos/obfstr2/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/AlbusPhos/obfstr2/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/AlbusPhos/obfstr2/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/AlbusPhos/obfstr2/releases/tag/v0.3.0
