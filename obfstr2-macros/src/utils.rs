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

/// 把 `bN!` 返回的字节容器还原为目标宽字符容器。
pub(crate) fn wrap_wide(
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
            __wide_bytes.wipe_data();
            drop(__wide_bytes);
            unsafe { __wide_res.unwrap_unchecked() };
            __wide_buf
        }
    }
}
