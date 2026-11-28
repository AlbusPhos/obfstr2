// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

/// UTF-16 码元按小端展开为字节流（长度恒为 `units.len() * 2`）。
pub(crate) fn wide_to_le(units: &[u16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(units.len() * 2);
    for u in units {
        bytes.extend_from_slice(&u.to_le_bytes());
    }
    bytes
}

/// `u32` 数组按小端展开为字节流（长度恒为 `words.len() * 4`）。
pub(crate) fn u32_to_le(words: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(words.len() * 4);
    for w in words {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    bytes
}

/// `u64` 数组按小端展开为字节流（长度恒为 `words.len() * 8`）。
pub(crate) fn u64_to_le(words: &[u64]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(words.len() * 8);
    for w in words {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    bytes
}

/// 把 `bN!` 返回的字节容器还原为目标定宽原语容器（`StackU16` / `HeapU16` 等）。
pub(crate) fn wrap_primitive(
    inner: TokenStream2,
    n: usize,
    main_type_path: TokenStream2,
) -> TokenStream2 {
    quote! {
        {
            let mut __wide_bytes = #inner;
            let mut __wide_buf = #main_type_path::<#n>::new();
            let __wide_res = __wide_buf
                .extend_from_zeroable_primitive_slice_le(&*__wide_bytes);
            // 仅 drop 即可：Buffer 的 Drop 已对整段 capacity 执行 volatile 清零。
            drop(__wide_bytes);
            unsafe { __wide_res.unwrap_unchecked() };
            __wide_buf
        }
    }
}
