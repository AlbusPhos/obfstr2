// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 过程宏 crate 的单元测试目录：子模块与被测源码文件一一对应，
//! 通过 `pub(crate)` 可见性访问 crate 内部实现（如 `CRYPTOS`、`encrypt_layers`）。

mod core;
mod crypto;
mod fmt;
mod key;
mod storage;
mod types;
mod user;

fn obfstr2_dep(no_default: bool) -> String {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
    if no_default {
        format!(
            "obfstr2 = {{ path = {:?}, default-features = false }}",
            root
        )
    } else {
        format!("obfstr2 = {{ path = {:?} }}", root)
    }
}
