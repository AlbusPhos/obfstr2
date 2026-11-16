// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::b_full;
use crate::core::{TIER_BALANCED, TIER_HIGH, TIER_LOW, TierParams};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

fn s_impl(input: String, tier: TierParams) -> Result<TokenStream2, String> {
    let size = input.len();
    let obf = b_full(input.into_bytes(), tier)?;
    let ts = obf.tokens;
    let main_type_path = if obf.main_is_heap {
        quote!(::obfstr2::types::str::HeapStr)
    } else {
        quote!(::obfstr2::types::str::StackStr)
    };

    Ok(quote! {
        {
            let mut bytes = #ts;
            unsafe { #main_type_path::<#size>::try_from(bytes.as_mut_slice()).unwrap_unchecked() }
        }
    })
}

pub fn s1(input: String) -> Result<TokenStream2, String> {
    s_impl(input, TIER_LOW)
}

pub fn s2(input: String) -> Result<TokenStream2, String> {
    s_impl(input, TIER_BALANCED)
}

pub fn s3(input: String) -> Result<TokenStream2, String> {
    s_impl(input, TIER_HIGH)
}

/// 自定义档：以调用方给定的 `TierParams` 混淆字符串载荷。
pub fn s_user(input: String, tier: TierParams) -> Result<TokenStream2, String> {
    s_impl(input, tier)
}
