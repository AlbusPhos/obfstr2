// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::{b1_full, b2_full, b3_full};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub fn s1(input: String) -> Result<TokenStream2, String> {
    let size = input.len();
    let obf = b1_full(input.into_bytes())?;
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

pub fn s2(input: String) -> Result<TokenStream2, String> {
    let size = input.len();
    let obf = b2_full(input.into_bytes())?;
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

pub fn s3(input: String) -> Result<TokenStream2, String> {
    let size = input.len();
    let obf = b3_full(input.into_bytes())?;
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
