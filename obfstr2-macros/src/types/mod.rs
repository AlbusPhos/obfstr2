// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 档位适配层：各类型载荷 → 字节混淆内核 → 容器 / 裸值的薄包装。
//!
//! 档位参数见 `crate::core::{TIER_LOW, TIER_BALANCED, TIER_HIGH}`；
//! 输入解析见 `crate::parse`。

mod bytes;
mod cstr;
mod cwstr;
mod float;
mod int;
mod str;
mod words;
mod wstr;

pub(crate) use bytes::{b_full_keyed, b_key, b_key_user, b_user, b1, b2, b3};
pub(crate) use cstr::{cs_key, cs_key_user, cs_user, cs1, cs2, cs3};
pub(crate) use cwstr::{cw_key, cw_key_user, cw_user, cw1, cw2, cw3};
pub(crate) use float::{fl_key, fl_key_user, fl_user, fl1, fl2, fl3};
pub(crate) use int::{i_key, i_key_user, i_user, i1, i2, i3};
pub(crate) use str::{s_key, s_key_user, s_user, s1, s2, s3};
pub(crate) use words::{
    d_key, d_key_user, d_user, d1, d2, d3, q_key, q_key_user, q_user, q1, q2, q3,
};
pub(crate) use wstr::{w_key, w_key_user, w_user, w1, w2, w3};
