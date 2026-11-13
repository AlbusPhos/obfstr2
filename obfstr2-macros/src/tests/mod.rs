// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod crypto;
mod storage;

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
