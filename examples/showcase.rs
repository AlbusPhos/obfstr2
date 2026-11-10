// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use obfstr2::types::str::Str;
use obfstr2::{b3, cs2, f2, fl2, i1, i2, i3, s_fmt2, s1, s2, s3};

/// 应用身份：名称 / 版本 / 发布通道。
fn app_identity() -> (Box<dyn Str>, Box<dyn Str>, Box<dyn Str>) {
    (
        Box::new(s2!("Orbit Notes")),
        Box::new(s2!("1.4.2")),
        Box::new(s1!("stable")),
    )
}

/// 服务端地址：使用保留域名，仅作配置展示，不发起任何请求。
fn default_endpoint() -> Box<dyn Str> {
    Box::new(s2!("https://api.example.com/v1"))
}

/// 演示用访问令牌（虚构值），展示高强度档在敏感配置上的用法。
fn demo_token() -> Box<dyn Str> {
    Box::new(s3!("demo_7f3c9a1e5b2d4c6f8a0e"))
}

/// 功能开关位掩码：还原后就是普通整数，可直接参与位运算。
fn feature_flags() -> (u32, bool) {
    let flags = i2!(0b1011u32);
    (flags, flags & i1!(0b0001u32) != 0)
}

/// 网络与重试参数。
fn transport_defaults() -> (f64, u32, usize) {
    (fl2!(2.5), i3!(3u32), i1!(4096usize))
}

/// 编译期内嵌的构建清单（`assets/fixture.bin`）。
fn build_manifest() -> impl AsRef<[u8]> {
    f2!("assets/fixture.bin")
}

fn hex_preview(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(bytes.len() * 3);
    for (i, byte) in bytes.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn main() {
    let default_user = s2!("world");
    let arg_user = std::env::args().nth(1);
    let user = arg_user.as_deref().unwrap_or(&default_user);

    let (name, version, channel) = app_identity();
    let endpoint = default_endpoint();
    let token = demo_token();
    let (flags, metrics_enabled) = feature_flags();
    let (timeout, retries, body_limit) = transport_defaults();
    let manifest = build_manifest();
    let magic = b3!(b"ORBIT");
    let runtime = cs2!("orbit-runtime");

    println!(
        "{}",
        s_fmt2!(
            "启动 {} v{}（{} 通道）",
            name.as_str(),
            version.as_str(),
            channel.as_str()
        )
    );
    println!("  用户       : {}", s_fmt2!("你好，{}！", user));
    println!("  服务端     : {endpoint}");
    println!("  访问令牌   : {}…（已脱敏）", &token.as_str()[..12]);
    println!("  功能位掩码 : 0b{flags:04b}，metrics={metrics_enabled}");
    println!("  超时/重试  : {timeout}s / {retries} 次，响应上限 {body_limit} 字节");
    println!("  运行时库   : {runtime}");
    println!(
        "  文件签名   : {} / 构建清单 {} 字节 [{}]",
        String::from_utf8_lossy(&magic),
        manifest.as_ref().len(),
        hex_preview(manifest.as_ref())
    );

    // 混淆值还原后就是普通值，可直接比较、断言与运算。
    let scheme = s2!("https://");
    assert!(endpoint.starts_with(&*scheme));
    assert_eq!(&*s1!("stable"), channel.as_str());
    assert_eq!(&*magic, b"ORBIT");
}
