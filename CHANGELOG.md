# Changelog

本文件记录本项目所有值得关注的变更。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循[语义化版本 2.0.0](https://semver.org/lang/zh-CN/)。

> **版本号说明（0.x 阶段）**：在 1.0.0 发布之前，次版本号（`0.MINOR.0`）的变更
> 也可能包含破坏性改动，请留意标注为 **[BREAKING]** 的条目。

---

## [0.4.3] - 2026-11-16

### Changed
- 文档：README / README_en 修复数学公式渲染——行内公式与 `\text{}` 内下划线补空格 / 双重转义，避免 `_` 被 Markdown 解析为斜体；`$O(\log M)$` 与 `$\text{min\_security}$` 分别改为纯文本与行内代码；README_en 基准对比、Trade-offs、Use Cases、宏一览、特性表、基准数据六张表格统一对齐排版，i18n 锚点同步刷新

## [0.4.2] - 2026-11-16

### Changed
- 容器堆栈判定收拢至档位：`build_obfuscated_bytes` 移除散装的 `allow_heap` / `prefer_stack` 参数（签名收敛为 `(载荷, TierParams)` 两参），堆 / 栈判定统一由 `TierParams.prefer_stack` 字段与新增的 `use_heap(size, is_main, prefer_stack)` 完成——`alloc` 下 ≤16B 恒栈、达到预算（主容器 `STACK_BUDGET`，chunk 中间容器减半）恒堆，其间按档位掷币（1 档栈概率 76%、2 / 3 档 40%）；`alloc` 关闭时超预算编译期报错，垃圾块超预算保持静默跳过
- 主容器决策上浮：`build_obfuscated_bytes` 返回 `ObfuscatedBytes { tokens, main_is_heap }`，`cs!` / `s!` / `w!` / `cw!` 系外层容器族统一跟随内层的单次判定，删除 `want_heap()` 与 `b2_forced` 逃生门；消除内外两侧独立掷币导致外层零拷贝容器（`from_inner_with_nul`）与内层主容器族错配的风险
- 档位堆栈分布调整：`alloc` 下 1 档不再恒纯栈（>16B 载荷约 24% 走堆）、2 档 ≤16B 载荷恒栈而 >16B 时 60% 走堆、3 档主容器由恒堆调整为 60% 走堆；`i3` / `fl3` 主容器语义自此与 `b3` / `s3` / `cs3` 对齐
- `TierParams` 反反汇编区间参数改为 `RangeInclusive<u8>` 类型（档位常量以 `1..=2` 形式直书区间），`encrypt_layers` 与各编排函数统一接收 `&TierParams`，并移除不再需要的 `#[allow(unused)]`
- HB2 空白隐写编码结果的 UTF-8 处理由 `from_utf8(out).expect(...)` 改为 `String::from_utf8_lossy`（理论上不可达的错误路径，非法序列以 U+FFFD 替换而非编译期 panic）
- 文档：README / README_en 新增"堆栈路由"说明，修正 `w1!` / `cw1!` 档位描述与特性注意事项

## [0.4.1] - 2026-11-15

### Added
- 新增 `anti-disassembly` 特性：在解密链路的非内联包装函数体、内联解密片段、多级折叠聚合函数（`__fold_*`）与 chunk 展开作用域首尾四类语句位置，按档位概率注入 `anti_disassembly!` 反反汇编垃圾指令块（编译期展开为 x86 / x86_64 门控内联汇编）；低 / 均衡 / 高强度档注入概率 5% / 40% / 80%，块数（`count`）与垃圾字节基准（`garbage`）随档位递增，块数、位置与内容每次编译随机生成，无法特征化；底层依赖强制以 `red-zone-safety` 特性编译，x86_64 下 gadget 先下移 `rsp` 再 `push`，不破坏 System V 红区内被溢出的局部变量
- HB2 空白隐写语料库外置化：`obfstr2-macros` 新增 `assets/text/` 拟真文本语料（真实程序片段、日志、脚本等 15 个文件），编译期经 `include_dir` 嵌入并递归收集非空 UTF-8 文本，取代原硬编码 OpenSSL 伪特征字符串表（`HB2_DECOYS`）

### Changed
- [BREAKING] 默认特性改为 `["alloc", "anti-disassembly"]`：默认构建同时启用堆内存支持与反反汇编注入；需纯栈 / 无注入形态时请使用 `default-features = false` 并按需单独启用 `alloc`
- HB2 空白隐写符号表随机化：每次编码在 `\0`+`\n` 与 `\0`+空格间随机选择并 `shuffle`，解码端改用随密文一同发射的符号表字节串字面量（取代固定 `DEFAULT_TABLE`）
- HB2 隐写插入算法重写：从语料库文本按"空白符号 + `\0`"切分候选片段，随机选取、随机位置 `splice` 插入；预算约为编码数据的 10%（最低 3 字节），外层语料尝试 10 次 / 内层片段尝试 32 次封顶
- 加密选层简化：移除档位 `magnification` 倍率，所有档位均以随机 `1..3` 个基础层起步，直至组合安全度达到 `min_security`；高强度档强度改由原语叠加、`wrap_pct` / `junk_pct` / `cold_pct` 概率与反反汇编注入密度保证
- 编排参数重构：`build_dec_layers` / `seal_chunk_group` / `fold_chunk_nodes` 从散装参数（`wrap_pct` / `cold_pct` / `max_latency`）统一改为接收 `&TierParams`；`TierParams` 新增 `anti_disasm_pct` / `anti_disasm_count_range` / `anti_disasm_garbage_range` 三字段
- 依赖升级：`lib-unknown` 由 `=0.4.1` 升级至 `=0.4.2`；新增可选依赖 `anti-disassembly 0.1.0`（强制 `red-zone-safety`）与构建期依赖 `include_dir 0.7.4`
- 文档：README / README_en 特性标志表新增 `alloc` / `anti-disassembly` 并重写 `default` 行，核心亮点新增"反反汇编垃圾指令注入"章节，HB2 隐写描述同步语料库与随机符号表机制，基准数据更新为最新实测（`b2` / `b3` 运行开销因反反汇编注入显著上升）

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

[Unreleased]: https://github.com/AlbusPhos/obfstr2/compare/v0.4.3...HEAD
[0.4.3]: https://github.com/AlbusPhos/obfstr2/compare/v0.4.2...v0.4.3
[0.4.2]: https://github.com/AlbusPhos/obfstr2/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/AlbusPhos/obfstr2/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/AlbusPhos/obfstr2/compare/v0.3.3...v0.4.0
[0.3.3]: https://github.com/AlbusPhos/obfstr2/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/AlbusPhos/obfstr2/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/AlbusPhos/obfstr2/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/AlbusPhos/obfstr2/releases/tag/v0.3.0
