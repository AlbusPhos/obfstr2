// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 根 crate 冒烟测试：三档宏、`*_user!` 与 `*_key!` 家族的小载荷 roundtrip。
use obfstr2::*;

macro_rules! assert_obf_s {
    ($m1:ident, $m2:ident, $m3:ident, [$($input:expr),+ $(,)?]) => {
        $(
            let exp = $input;
            let (v1, v2, v3) = ($m1!($input), $m2!($input), $m3!($input));
            assert_eq!(&*v1, exp, "{} roundtrip failed", stringify!($m1));
            assert_eq!(&*v2, exp, "{} roundtrip failed", stringify!($m2));
            assert_eq!(&*v3, exp, "{} roundtrip failed", stringify!($m3));

            assert_eq!(format!("{v1}"), format!("{exp}"), "{} Display roundtrip failed", stringify!($m1));
            assert_eq!(format!("{v2}"), format!("{exp}"), "{} Display roundtrip failed", stringify!($m2));
            assert_eq!(format!("{v3}"), format!("{exp}"), "{} Display roundtrip failed", stringify!($m3));

            assert_eq!(format!("{v1:?}"), format!("{exp:?}"), "{} Debug roundtrip failed", stringify!($m1));
            assert_eq!(format!("{v2:?}"), format!("{exp:?}"), "{} Debug roundtrip failed", stringify!($m2));
            assert_eq!(format!("{v3:?}"), format!("{exp:?}"), "{} Debug roundtrip failed", stringify!($m3));

            assert_eq!(&*v1, &*v2, "{} != {}", stringify!($m1), stringify!($m2));
            assert_eq!(&*v1, &*v3, "{} != {}", stringify!($m1), stringify!($m3));
            assert_eq!(&*v2, &*v3, "{} != {}", stringify!($m2), stringify!($m3));


        )+
    };
}
macro_rules! assert_obf_b {
    ($m1:ident, $m2:ident, $m3:ident, [$($input:expr),+ $(,)?]) => {
        $(
            let exp = $input;
            let (v1, v2, v3) = ($m1!($input), $m2!($input), $m3!($input));
            assert_eq!(&*v1, exp, "{} roundtrip failed", stringify!($m1));
            assert_eq!(&*v2, exp, "{} roundtrip failed", stringify!($m2));
            assert_eq!(&*v3, exp, "{} roundtrip failed", stringify!($m3));

            // assert_eq!(format!("{v1}"), format!("{exp}"), "{} Display roundtrip failed", stringify!($m1));
            // assert_eq!(format!("{v2}"), format!("{exp}"), "{} Display roundtrip failed", stringify!($m2));
            // assert_eq!(format!("{v3}"), format!("{exp}"), "{} Display roundtrip failed", stringify!($m3));

            assert_eq!(format!("{:?}",&*v1), format!("{exp:?}"), "{} Debug roundtrip failed", stringify!($m1));
            assert_eq!(format!("{:?}",&*v2), format!("{exp:?}"), "{} Debug roundtrip failed", stringify!($m2));
            assert_eq!(format!("{:?}",&*v3), format!("{exp:?}"), "{} Debug roundtrip failed", stringify!($m3));

            assert_eq!(&*v1, &*v2, "{} != {}", stringify!($m1), stringify!($m2));
            assert_eq!(&*v1, &*v3, "{} != {}", stringify!($m1), stringify!($m3));
            assert_eq!(&*v2, &*v3, "{} != {}", stringify!($m2), stringify!($m3));


        )+
    };
}

#[cfg(not(feature = "budget-1k"))]
#[test]
fn str_roundtrip() {
    for _ in 0..10 {
        assert_obf_s!(
            s1,
            s2,
            s3,
            [
                "",
                "hello",
                "你好，世界🦀",
                "H̵̛͕̞̦̰̜͍̰̥̟͆̏͂̌͑ͅä̷͔̟͓̬̯̟͍̭͉͈̮͙̣̯̬͚̞̭̍̀̾͠m̴̡̧̛̝̯̹̗̹̤̲̺̟̥̈̏͊̔̑̍͆̌̀̚͝͝b̴̢̢̫̝̠̗̼̬̻̮̺̭͔̘͑̆̎̚ư̵̧̡̥̙̭̿̈̀̒̐̊͒͑r̷̡̡̲̼̖͎̫̮̜͇̬͌͘g̷̹͍͎̬͕͓͕̐̃̈́̓̆̚͝ẻ̵̡̼̬̥̹͇̭͔̯̉͛̈́̕r̸̮̖̻̮̣̗͚͖̝̂͌̾̓̀̿̔̀͋̈́͌̈́̋͜",
                "往復往復往復",
                "ruovttoluottamátkiruovttoluottamátkiruovttoluotta",
                "\0",
                "\0籣籪籵籰籸 籽籮粁籽 籲籼 籰籮籷籮类籪籽籮籭 ",
                "Hin- a Réckrees Hin- a Réckrees Hin- a Réckrees{{}}{}",
                "HioHioluot界🦀taư̵̧̡̥̙̭̿̈̀̒mák籮Hioluot界🦀taư̵̧̡̥̙̭̿̈̀̒mák籮粁$@&#($0\"k\0\x1f籽 籲rees Hi̹͍͎̬͕͓͕̐̃̈́̓̆̚͝ẻ̵̡̼̬̥̹͇̭͔̯̉͛̈́̕r̸n-{}}{}粁$@&#($0\"k\0\x1f籽 籲rees Hi̹͍͎̬͕͓͕̐̃̈́̓̆̚͝ẻ̵̡̼̬̥̹͇̭͔̯̉͛̈́̕r̸n-{}}{}luot界🦀taư̵̧̡̥̙̭̿̈̀̒mák籮粁$@&#($0\"k\0\x1f籽 籲rees Hi̹͍͎̬͕͓͕̐̃̈́̓̆̚͝ẻ̵̡̼̬̥̹͇̭͔̯̉͛̈́̕r̸n-{}}{}Hioluot界🦀taư̵̧̡̥̙̭̿̈̀̒mák籮粁$@&#($0\"k\0\x1f籽 籲rees Hi̹͍͎̬͕͓͕̐̃̈́̓̆̚͝ẻ̵̡̼̬̥̹͇̭͔̯̉͛̈́̕r̸n-{}}{}Hioluot界🦀taư̵̧̡̥̙̭̿̈̀̒mák籮粁$@&#($0\"k\0\x1f籽 籲rees Hi̹͍͎̬͕͓͕̐̃̈́̓̆̚͝ẻ̵̡̼̬̥̹͇̭͔̯̉͛̈́̕r̸n-{}}{}Hioluot界🦀taư̵̧̡̥̙̭̿̈̀̒mák籮粁$@&#($0\"k\0\x1f籽 籲rees Hi̹͍͎̬͕͓͕̐̃̈́̓̆̚͝ẻ̵̡̼̬̥̹͇̭͔̯̉͛̈́̕r̸n-{}}{}Hioluot界🦀taư̵̧̡̥̙̭̿̈̀̒mák籮粁$@&#($0\"k\0\x1f籽 籲rees Hi̹͍͎̬͕͓͕̐̃̈́̓̆̚͝ẻ̵̡̼̬̥̹͇̭͔̯̉͛̈́̕r̸n-{}}{}Hioluot界🦀taư̵̧̡̥̙̭̿̈̀̒mák籮粁$@&#($0\"k\0\x1f籽 籲rees Hi̹͍͎̬͕͓͕̐̃̈́̓̆̚͝ẻ̵̡̼̬̥̹͇̭͔̯̉͛̈́̕r̸n-{}}{}HioluHioluot界🦀taư̵̧̡̥̙̭̿̈̀̒mák籮Hioluot界🦀taư̵̧̡̥̙̭̿̈̀̒mák籮粁$Hioluot界🦀taư̵̧̡̥̙̭̿̈̀̒mák籮粁$@&#($0\"k\0\x1f籽 籲rees Hi̹͍͎̬͕͓͕̐̃̈́̓̆̚͝ẻ̵̡̼̬̥̹͇̭͔̯̉͛̈́̕r̸n-{}}{}@&#($0\"k\0\x1f籽 籲rees Hi̹͍͎̬͕͓͕̐̃̈́̓̆̚͝ẻ̵̡̼̬̥̹͇̭͔̯̉͛̈́̕r̸n-{}}{}粁$@&#($0\"k\0\x1f籽 籲rees Hi̹͍͎̬͕͓͕̐̃̈́̓̆̚͝ẻ̵̡̼̬̥̹͇̭͔̯̉͛̈́̕r̸n-{}}{}ot界🦀taư̵̧̡̥̙̭̿̈̀̒mák籮粁$@&#($0\"k\0\x1f籽 籲rees Hi̹͍͎̬͕͓͕̐̃̈́̓̆̚͝ẻ̵̡̼̬̥̹͇̭͔̯̉͛̈́̕r̸n-{}}{}Hioluot界🦀taư̵̧̡̥̙̭̿̈̀̒mák籮粁$@&#($0\"k\0\x1f籽 籲rees Hi̹͍͎̬͕͓͕̐̃̈́̓̆̚͝ẻ̵̡̼̬̥̹͇̭͔̯̉͛̈́̕r̸n-{}}{}",
            ]
        );
    }
}
#[cfg(feature = "budget-1k")]
#[test]
fn str_roundtrip() {
    for _ in 0..10 {
        assert_obf_s!(
            s1,
            s2,
            s3,
            [
                "",
                "hello",
                "你好，世界🦀",
                "H̵̛͕̞̦̰̜͍̰̥̟͆̏͂̌͑ͅä̷͔̟͓̬̯̟͍̭͉͈̮͙̣̯̬͚̞̭̍̀̾͠m̴̡̧̛̝̯̹̗̹̤̲̺̟̥̈̏͊̔̑̍͆̌̀̚͝͝b̴̢̢̫̝̠̗̼̬̻̮̺̭͔̘͑̆̎̚ư̵̧̡̥̙̭̿̈̀̒̐̊͒͑r̷̡̡̲̼̖͎̫̮̜͇̬͌͘g̷̹͍͎̬͕͓͕̐̃̈́̓̆̚͝ẻ̵̡̼̬̥̹͇̭͔̯̉͛̈́̕r̸̮̖̻̮̣̗͚͖̝̂͌̾̓̀̿̔̀͋̈́͌̈́̋͜",
                "往復往復往復",
                "ruovttoluottamátkiruovttoluottamátkiruovttoluotta",
                "\0",
                "\0籣籪籵籰籸 籽籮粁籽 籲籼 籰籮籷籮类籪籽籮籭 ",
                "Hin- a Réckrees Hin- a Réckrees Hin- a Réckrees{{}}{}",
                "HioHioluot界🦀taư̵̧̡̥̙̭̿̈̀̒mák籮Hioluot界🦀taư̵̧̡̥̙̭̿̈̀̒mák籮粁$@&#($0\"k\0\x1f籽 籲rees Hi̹͍͎̬͕͓͕̐̃̈́̓̆̚͝ẻ̵̡̼̬̥̹͇̭͔̯̉͛̈́̕r̸n-{}}{}粁$@&#($0\"k\0\x1f籽 籲rees Hi̹͍͎̬͕͓͕̐̃̈́̓̆̚͝ẻ̵̡̼̬̥̹͇̭͔̯̉͛̈́̕r̸n-{}}{}luot界🦀taư̵̧̡̥̙̭̿̈̀̒mák籮粁$@&#($0\"k\0\x1f籽 籲rees Hi̹"
            ]
        );
    }
}
#[test]
fn bytes_roundtrip() {
    for _ in 0..10 {
        assert_obf_b!(
            b1,
            b2,
            b3,
            [
                b"",
                [0u8],
                b"\0",
                [0, 255],
                [0x61, 98, 0, 255],
                // [8;512], // 未来需要支持
                b"smoke-dc0fcfcc6310b906",
                b"{}{{}}`'8)**Y*Y*^$@&#($0\"k\0\xff\x39\xa1\x03",
            ]
        );
    }
}

#[test]
fn file_all_tiers_match_include_bytes() {
    // 路径相对本 crate 的 CARGO_MANIFEST_DIR 解析
    assert_eq!(
        &*f1!("assets/fixture.bin"),
        include_bytes!("../assets/fixture.bin")
    );
    assert_eq!(
        &*f2!("assets/fixture.bin"),
        include_bytes!("../assets/fixture.bin")
    );
    assert_eq!(
        &*f3!("assets/fixture.bin"),
        include_bytes!("../assets/fixture.bin")
    );
}

#[test]
fn int_all_tiers_roundtrip() {
    // 无符号边界 + 有符号边界 + 默认 i32 + 进制/下划线 + usize/isize/u128
    assert_eq!(i1!(0u8), 0u8);
    assert_eq!(i1!(255u8), 255u8);
    assert_eq!(i2!(0u8), 0u8);
    assert_eq!(i3!(255u8), 255u8);
    assert_eq!(i2!(-128i8), -128i8);
    assert_eq!(i2!(-32768i16), i16::MIN);
    assert_eq!(i2!(127i8), 127i8);
    assert_eq!(i2!(0xFFu16), 0xFFu16);
    assert_eq!(i2!(1_000u32), 1_000u32);
    assert_eq!(i2!(42), 42i32);
    assert_eq!(i2!(-1), -1i32);
    assert_eq!(i2!(-2147483648), i32::MIN);
    assert_eq!(i2!(2147483647), i32::MAX);
    assert_eq!(i2!(18446744073709551615u64), u64::MAX);
    assert_eq!(i2!(-9223372036854775808i64), i64::MIN);
    assert_eq!(i2!(340282366920938463463374607431768211455u128), u128::MAX);
    assert_eq!(i2!(-170141183460469231731687303715884105728i128), i128::MIN);
    assert_eq!(i2!(123usize), 123usize);
    assert_eq!(i2!(-123isize), -123isize);
    // 裸值可直接算术、比较、绑定传递
    let x = i2!(40u32);
    assert_eq!(x + 2, 42u32);
    let y: u32 = i2!(1u32);
    assert_eq!(y, 1u32);
}

#[test]
fn int_all_tiers_agree() {
    // 三档对同一输入解码一致
    assert_eq!(i1!(42u32), i2!(42u32));
    assert_eq!(i2!(42u32), i3!(42u32));
    assert_eq!(i1!(-7i16), i3!(-7i16));
}

#[test]
fn float_all_tiers_roundtrip() {
    // 断言一律用 to_bits：-0.0 与 0.0 的 == 为 true 但位不同，NaN 恒不自等。
    assert_eq!(fl1!(0.0f32).to_bits(), 0.0f32.to_bits());
    assert_eq!(fl1!(-0.0f32).to_bits(), (-0.0f32).to_bits());
    assert!(fl1!(-0.0f32).is_sign_negative());
    assert_eq!(fl2!(1.5f32).to_bits(), 1.5f32.to_bits());
    assert_eq!(fl2!(3.15).to_bits(), 3.15f64.to_bits());
    assert_eq!(fl2!(-1.0).to_bits(), (-1.0f64).to_bits());
    assert_eq!(fl2!(1e10).to_bits(), 1e10f64.to_bits());
    assert_eq!(fl2!(5f32).to_bits(), 5f32.to_bits());
    assert_eq!(
        fl2!(340282346638528859811704183484516925440.0f32).to_bits(),
        f32::MAX.to_bits()
    );
    assert_eq!(fl2!(1.7976931348623157e308).to_bits(), f64::MAX.to_bits());
    // 注：f32::MIN_POSITIVE 是最小正规格数（0x00800000），1e-45f32 落入次正规区
    assert_eq!(fl2!(1e-45f32).to_bits(), 1e-45f32.to_bits());
    assert_eq!(
        fl2!(1.17549435e-38f32).to_bits(),
        f32::MIN_POSITIVE.to_bits()
    );
    assert_eq!(fl3!(2.5f64).to_bits(), 2.5f64.to_bits());
    // 裸值可直接算术、比较、绑定传递
    let x = fl2!(1.5f32);
    assert_eq!((x + 1.0).to_bits(), 2.5f32.to_bits());
    let y: f64 = fl2!(1.0);
    assert_eq!(y.to_bits(), 1.0f64.to_bits());
}

#[test]
fn float_all_tiers_agree() {
    // 三档对同一输入解码一致（按位比较）
    assert_eq!(fl1!(3.15).to_bits(), fl2!(3.15).to_bits());
    assert_eq!(fl2!(3.15).to_bits(), fl3!(3.15).to_bits());
    assert_eq!(fl1!(-0.0f32).to_bits(), fl3!(-0.0f32).to_bits());
}

#[test]
fn cstr_all_tiers_roundtrip() {
    // 三种字面量形态等价；空串 / ASCII / 非 UTF-8 边界
    assert_eq!(&*cs1!(""), c"");
    assert_eq!(&*cs1!(c""), c"");
    assert_eq!(&*cs1!(b""), c"");
    assert_eq!(&*cs1!("/bin/sh"), c"/bin/sh");
    assert_eq!(&*cs1!(c"/bin/sh"), c"/bin/sh");
    assert_eq!(&*cs1!(b"/bin/sh"), c"/bin/sh");
    assert_eq!(&*cs2!("/bin/sh"), c"/bin/sh");
    assert_eq!(&*cs3!("/bin/sh"), c"/bin/sh");
    // as_bytes 不含结尾 NUL；as_bytes_with_nul 含结尾 NUL
    assert_eq!(cs2!("hello").as_bytes(), b"hello");
    assert_eq!(cs2!("hi").as_bytes_with_nul(), b"hi\0");
    // b"..." 可表达非 UTF-8 载荷（CStr 只校验 NUL 语义）
    assert_eq!(cs2!(b"\xff\xfe").as_bytes(), &[0xFF, 0xFE]);
    // 三档对同一输入解码一致
    assert_eq!(&*cs1!("tier-check"), &*cs2!("tier-check"));
    assert_eq!(&*cs2!("tier-check"), &*cs3!("tier-check"));
    // Display lossy 与 as_ptr 可用
    assert_eq!(format!("{}", cs2!("abc")), "abc");
    assert!(!cs2!("abc").as_ptr().is_null());
}

#[test]
fn wide_cstr_all_tiers_roundtrip() {
    macro_rules! check {
        ($input:literal) => {{
            let mut expected: Vec<u16> = $input.encode_utf16().collect();
            expected.push(0);

            let (a, b, c) = (cw1!($input), cw2!($input), cw3!($input));
            assert_eq!(
                &*a,
                expected.as_slice(),
                "cw1 roundtrip failed: {:?}",
                $input
            );
            assert_eq!(
                &*b,
                expected.as_slice(),
                "cw2 roundtrip failed: {:?}",
                $input
            );
            assert_eq!(
                &*c,
                expected.as_slice(),
                "cw3 roundtrip failed: {:?}",
                $input
            );

            assert_eq!(&*a, &*b);
            assert_eq!(&*b, &*c);
            assert_eq!(a[a.len() - 1], 0, "结尾码元应为 NUL");
            let text = String::from_utf16(&a[..a.len() - 1]).unwrap();
            assert_eq!(text, $input);
        }};
    }

    check!("");
    check!("hello");
    check!("你好，世界🦀");
    check!("kernel32.dll");
}

#[test]
fn wide_str_all_tiers_roundtrip() {
    macro_rules! check {
        ($input:literal) => {{
            let expected: Vec<u16> = $input.encode_utf16().collect();

            let (a, b, c) = (w1!($input), w2!($input), w3!($input));
            assert_eq!(
                &*a,
                expected.as_slice(),
                "w1 roundtrip failed: {:?}",
                $input
            );
            assert_eq!(
                &*b,
                expected.as_slice(),
                "w2 roundtrip failed: {:?}",
                $input
            );
            assert_eq!(
                &*c,
                expected.as_slice(),
                "w3 roundtrip failed: {:?}",
                $input
            );

            assert_eq!(&*a, &*b);
            assert_eq!(&*b, &*c);
            // 不自动追加结尾 NUL：长度必须与 encode_utf16 的码元数完全一致
            assert_eq!(a.len(), expected.len(), "不应追加结尾 NUL");
            let text = String::from_utf16(&*a).unwrap();
            assert_eq!(text, $input);
        }};
    }

    check!("");
    check!("hello");
    check!("你好，世界🦀");
    check!("a\0b");
    check!("\0");
    check!("\0a\0");
    check!("kernel32.dll");
}

#[test]
fn wide_str_keeps_nul_and_no_terminator() {
    let w = w2!("a\0b");
    assert_eq!(&*w, &[0x61u16, 0, 0x62][..]);
    assert_eq!(w.len(), 3);

    let empty = w2!("");
    assert!(empty.is_empty());

    let tail = w1!("abc");
    assert_eq!(&*tail, &[0x61u16, 0x62, 0x63][..]);
}

#[test]
fn macro_metavariable_forwarding_works() {
    // 用户把宏再包一层 macro_rules! 时，字面量/表达式元变量会被不可见 None 分组包裹；
    // 所有基于 syn::Expr 的解析入口都必须解包，否则会误报“不支持表达式”。
    macro_rules! fwd_s {
        ($v:literal) => {
            s1!($v)
        };
    }
    macro_rules! fwd_b {
        ($v:literal) => {
            b1!($v)
        };
    }
    macro_rules! fwd_b_arr {
        ($v:literal) => {
            b1!([0x61, $v])
        };
    }
    macro_rules! fwd_cs {
        ($v:literal) => {
            cs1!($v)
        };
    }
    macro_rules! fwd_i {
        ($v:literal) => {
            i1!($v)
        };
    }
    macro_rules! fwd_neg_i {
        ($v:literal) => {
            i1!(-$v)
        };
    }
    macro_rules! fwd_f {
        ($v:literal) => {
            fl1!($v)
        };
    }
    macro_rules! fwd_neg_f {
        ($v:literal) => {
            fl1!(-$v)
        };
    }
    macro_rules! fwd_w {
        ($v:literal) => {
            w1!($v)
        };
    }
    macro_rules! fwd_cw {
        ($v:literal) => {
            cw1!($v)
        };
    }

    assert_eq!(&*fwd_s!("hello"), "hello");
    assert_eq!(&*fwd_b!(b"hi"), &b"hi"[..]);
    assert_eq!(&*fwd_b_arr!(0x62), &[0x61u8, 0x62][..]);
    assert_eq!(&*fwd_cs!("hi"), c"hi");
    assert_eq!(&*fwd_cs!(c"hi"), c"hi");
    assert_eq!(fwd_i!(42i32), 42i32);
    assert_eq!(fwd_neg_i!(42i32), -42i32);
    assert_eq!(fwd_f!(5f32), 5.0f32);
    assert_eq!(fwd_neg_f!(5f32).to_bits(), (-5.0f32).to_bits());
    assert_eq!(fwd_neg_f!(0.0f32).to_bits(), (-0.0f32).to_bits());
    assert_eq!(&*fwd_w!("hi"), &[0x68u16, 0x69][..]);
    assert_eq!(&*fwd_cw!("hi"), &[0x68u16, 0x69, 0][..]);
}

#[test]
fn owned_containers_can_bind_and_reuse() {
    // 回归 CasualX E0716 差异点：自有容器可绑定、传递、复用
    let hello = s2!("hello");
    assert_eq!(hello.to_uppercase(), "HELLO");
    assert_eq!(&*hello, "hello");
    let b = b2!(b"abc");
    assert_eq!(b.len(), 3);
    assert_eq!(&*b, &*b);
    fn echo_len(data: &[u8]) -> usize {
        data.len()
    }
    assert_eq!(echo_len(&b), 3);
}

#[test]
fn user_macros_roundtrip() {
    // 自定义档：`({ 配置 }, 载荷)` 统一形态的 roundtrip
    use obfstr2::{b_user, cs_user, cw_user, f_user, fl_user, i_user, s_fmt_user, s_user, w_user};

    // 空配置块 == 均衡档语义
    assert_eq!(&*s_user!({}, "hello"), "hello");
    // 全字段覆盖 + 多行尾随逗号
    let s = s_user!(
        {
            prefer_stack: true,
            max_latency: 50,
            min_security: 70,
            security_factor: 0.8,
            wrap_pct: 30,
            junk_pct: 10,
            cold_pct: 15,
            pad_short: true,
        },
        "hello",
    );
    assert_eq!(&*s, "hello");

    assert_eq!(&*b_user!({ prefer_stack: true }, b"abc"), &b"abc"[..]);
    assert_eq!(&*b_user!({}, [0x61, 98, 99]), &[0x61u8, 0x62, 0x63][..]);
    assert_eq!(&*cs_user!({ max_latency: 50 }, c"/bin/sh"), c"/bin/sh");
    assert_eq!(&*cs_user!({}, "/bin/sh"), c"/bin/sh");
    let cw = cw_user!({ min_security: 60 }, "kernel32.dll");
    assert_eq!(
        &cw[..cw.len() - 1],
        "kernel32.dll".encode_utf16().collect::<Vec<_>>().as_slice()
    );
    assert_eq!(cw[cw.len() - 1], 0);
    assert_eq!(&*w_user!({}, "a\0b"), &[0x61u16, 0, 0x62][..]);
    assert_eq!(i_user!({ max_latency: 50 }, 42u32) + 1, 43u32);
    assert_eq!(i_user!({}, -1), -1i32);
    assert_eq!(
        fl_user!({ max_latency: 50 }, 3.15).to_bits(),
        3.15f64.to_bits()
    );
    assert_eq!(
        &*f_user!({ prefer_stack: true }, "assets/fixture.bin"),
        b"secret"
    );
    assert_eq!(
        s_fmt_user!({ max_latency: 50 }, "hello, {}!", "world"),
        "hello, world!"
    );
    let name = "world";
    assert_eq!(s_fmt_user!({}, "hi, {name}!", name = name), "hi, world!");
}

#[test]
fn user_macro_forwarding_through_user_macro_rules() {
    // 用户再包一层 macro_rules!（`:literal` 元变量转发产生 None 分组）不应误报
    use obfstr2::s_user;

    macro_rules! fwd_s {
        ($v:literal) => {
            s_user!({ prefer_stack: true }, $v)
        };
    }
    // 配置块转发需以 `:tt` 捕获（`{ key: val }` 不是合法表达式字面量，
    // `:expr` 捕获会在匹配阶段被 rustc 以 "struct literal body without path" 拒绝）
    macro_rules! fwd_cfg_tt {
        ($cfg:tt, $v:literal) => {
            s_user!($cfg, $v)
        };
    }
    assert_eq!(&*fwd_s!("hello"), "hello");
    assert_eq!(&*fwd_cfg_tt!({ max_latency: 40 }, "hello"), "hello");
}

// ── 动态密钥档：正确 run_key 的端到端 roundtrip ────────────────────────────

const KEY_STR: &str = "obfstr2-test-key-0123456789abcdef";

/// 运行期读取密钥（Path/Call 形态，绕开编译期字面量拒绝）。
fn run_key() -> &'static [u8] {
    KEY_STR.as_bytes()
}

#[test]
fn key_macros_roundtrip() {
    assert_eq!(
        &*s_key!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            ""
        ),
        ""
    );
    assert_eq!(
        &*s_key!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            "hello"
        ),
        "hello"
    );
    assert_eq!(
        &*s_key!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            "你好，世界🦀"
        ),
        "你好，世界🦀"
    );

    assert_eq!(
        &*b_key!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            b"abc"
        ),
        b"abc"
    );
    assert_eq!(
        &*b_key!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            [0x61, 98, 99]
        ),
        &[0x61u8, 0x62, 0x63][..]
    );

    assert_eq!(
        &*f_key!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            "assets/fixture.bin"
        ),
        b"secret"
    );

    assert_eq!(
        s_fmt_key!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            "hello, {}!",
            "world"
        ),
        "hello, world!"
    );

    assert_eq!(
        i_key!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            42u8
        ),
        42u8
    );
    assert_eq!(
        i_key!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            -1
        ),
        -1i32
    );

    assert_eq!(
        fl_key!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            3.15f64
        )
        .to_bits(),
        3.15f64.to_bits()
    );

    assert_eq!(
        &*cs_key!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            c"/bin/sh"
        ),
        c"/bin/sh"
    );

    assert_eq!(
        &*w_key!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            "a\0b"
        ),
        &[0x61u16, 0, 0x62][..]
    );

    let cw = cw_key!(
        { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
        "kernel32.dll"
    );
    assert_eq!(
        &cw[..cw.len() - 1],
        "kernel32.dll".encode_utf16().collect::<Vec<_>>().as_slice()
    );
    assert_eq!(cw[cw.len() - 1], 0);
}

#[test]
fn key_user_macros_roundtrip() {
    let s = s_key_user!(
        { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
        { max_latency: 50, min_security: 60, prefer_stack: true },
        "hello"
    );
    assert_eq!(&*s, "hello");

    assert_eq!(
        &*b_key_user!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            { prefer_stack: true },
            b"abc"
        ),
        b"abc"
    );

    assert_eq!(
        &*f_key_user!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            { prefer_stack: true },
            "assets/fixture.bin"
        ),
        b"secret"
    );

    assert_eq!(
        s_fmt_key_user!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            { max_latency: 50 },
            "hi, {name}!",
            name = "world"
        ),
        "hi, world!"
    );

    assert_eq!(
        i_key_user!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            { max_latency: 50 },
            42u32
        ),
        42u32
    );

    assert_eq!(
        fl_key_user!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            { max_latency: 50 },
            3.15f32
        )
        .to_bits(),
        3.15f32.to_bits()
    );

    assert_eq!(
        &*cs_key_user!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            { prefer_stack: true },
            c"/bin/sh"
        ),
        c"/bin/sh"
    );

    assert_eq!(
        &*w_key_user!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            { prefer_stack: true },
            "a\0b"
        ),
        &[0x61u16, 0, 0x62][..]
    );

    let cw = cw_key_user!(
        { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
        { min_security: 60 },
        "kernel32.dll"
    );
    assert_eq!(
        &cw[..cw.len() - 1],
        "kernel32.dll".encode_utf16().collect::<Vec<_>>().as_slice()
    );
    assert_eq!(cw[cw.len() - 1], 0);
}

#[test]
fn key_build_key_from_env_roundtrip() {
    // build_key 走 env!：过程宏展开期读取，并在产物中注册 Cargo 环境依赖。
    // run_key 必须来自运行期表达式（这里用函数调用，避免被编译期字面量校验拒绝）。
    fn pkg_name_bytes() -> &'static [u8] {
        env!("CARGO_PKG_NAME").as_bytes()
    }
    let s = s_key!(
        { build_key: env!("CARGO_PKG_NAME"), run_key: pkg_name_bytes() },
        "env-keyed"
    );
    assert_eq!(&*s, "env-keyed");
}

#[test]
fn test_lib_unknown() {
    // `const_random!` 再导出的 const 上下文可用性：覆盖三种调用形式。
    const RAND_U64: u64 = obfstr2::const_random!();
    const RAND_U128: u128 = obfstr2::const_random!(u128);
    const RAND_U16_SALTED: u16 = obfstr2::const_random!(u16, "obfstr2-smoke");

    // 值由构建种子派生，此处仅校验类型宽度与 const 求值成功。
    assert_eq!(RAND_U64.count_ones() + RAND_U64.count_zeros(), u64::BITS);
    assert_eq!(RAND_U128.count_ones() + RAND_U128.count_zeros(), u128::BITS);
    assert_eq!(
        RAND_U16_SALTED.count_ones() + RAND_U16_SALTED.count_zeros(),
        u16::BITS
    );

    let [a, b, c] = obfstr2::hide_array!(u8, [1, 2, 3]);
    assert_eq!(a, 1);
    assert_eq!(b, 2);
    assert_eq!(c, 3);

    let [a, b, c] = obfstr2::hide_array!(u128, [u128::MAX, u128::MIN, 0xfffffff333]);
    assert_eq!(a, u128::MAX);
    assert_eq!(b, u128::MIN);
    assert_eq!(c, 0xfffffff333);

    static C: obfstr2::lazy::LazyLock<[u128; 3]> =
        obfstr2::hide_array_lazy!(u128, [u128::MAX, u128::MIN, 0xfffffff333]);
    let [a, b, c] = *C;
    assert_eq!(a, u128::MAX);
    assert_eq!(b, u128::MIN);
    assert_eq!(c, 0xfffffff333);

    obfstr2::hide_array_lazy!(static D : u64 = [u64::MAX, 0xfffffff333, 1111]);
    let [a, b, c] = *D;
    assert_eq!(a, u64::MAX);
    assert_eq!(b, 0xfffffff333);
    assert_eq!(c, 1111);
}

// ── 独立容器类型（Stack* / Heap*）完整测试矩阵 ──────────────────────────────

use core::any::{Any, TypeId, type_name, type_name_of_val};
#[cfg(feature = "alloc")]
use obfstr2::types::{
    buffer::{HeapBytes, HeapU16},
    cstr::HeapCStr,
    str::HeapStr,
};
use obfstr2::types::{
    buffer::{StackBytes, StackU16},
    cstr::StackCStr,
    str::StackStr,
};

/// 运行期精确容器类型断言：
/// - `Expected` 与 `Actual` 为独立泛型参数，即使类型或容量 `N` 不符也不会导致编译失败；
/// - 同时校验 `TypeId`（精确到元素类型与定容长度 `N`）与类型名中的 `"Stack"` / `"Heap"` 标识。
fn assert_container<Expected: 'static, Actual: 'static>(
    val: &Actual,
    expect_heap: bool,
    label: &str,
) {
    let actual_name = type_name_of_val(val);
    let expected_name = type_name::<Expected>();
    let expected_kind = if expect_heap { "Heap" } else { "Stack" };

    assert!(
        actual_name.contains(expected_kind),
        "[{label}] 堆栈族错误：期望包含 `{expected_kind}`，实际类型为 `{actual_name}`"
    );
    assert_eq!(
        Any::type_id(val),
        TypeId::of::<Expected>(),
        "[{label}] 容器精确类型或容量 N 不匹配：期望 `{expected_name}`，实际为 `{actual_name}`"
    );
}

#[test]
#[cfg(feature = "alloc")]
fn test_user_force_heap_matrix() {
    use obfstr2::{b_user, cs_user, cw_user, f_user, s_user, w_user};

    // ── 1a. *_user! + force_heap: true（短载荷 <= 16B）──
    let s = s_user!({ force_heap: true }, "123");
    assert_container::<HeapStr<3>, _>(&s, true, "s_user(force_heap: true)");
    assert_eq!(&*s, "123");

    let b = b_user!({ force_heap: true }, b"123");
    assert_container::<HeapBytes<3>, _>(&b, true, "b_user(force_heap: true)");
    assert_eq!(&*b, b"123");

    let f = f_user!({ force_heap: true }, "assets/fixture.bin");
    assert_container::<HeapBytes<6>, _>(&f, true, "f_user(force_heap: true)");
    assert_eq!(&*f, b"secret");

    let cs = cs_user!({ force_heap: true }, c"123");
    assert_container::<HeapCStr<4>, _>(&cs, true, "cs_user(force_heap: true)");
    assert_eq!(&*cs, c"123");

    let w = w_user!({ force_heap: true }, "123");
    assert_container::<HeapU16<3>, _>(&w, true, "w_user(force_heap: true)");
    assert_eq!(&*w, &[0x31u16, 0x32, 0x33][..]);

    let cw = cw_user!({ force_heap: true }, "123");
    assert_container::<HeapU16<4>, _>(&cw, true, "cw_user(force_heap: true)");
    assert_eq!(&*cw, &[0x31u16, 0x32, 0x33, 0][..]);

    // ── 1b. *_key_user! + force_heap: Some(true)（含空载荷 0B 边界）──
    let sk = s_key_user!(
        { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
        { force_heap: Some(true) },
        ""
    );
    assert_container::<HeapStr<0>, _>(&sk, true, "s_key_user(Some(true), empty)");
    assert_eq!(&*sk, "");

    let bk = b_key_user!(
        { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
        { force_heap: Some(true) },
        b"123"
    );
    assert_container::<HeapBytes<3>, _>(&bk, true, "b_key_user(Some(true))");
    assert_eq!(&*bk, b"123");

    let fk = f_key_user!(
        { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
        { force_heap: Some(true) },
        "assets/fixture.bin"
    );
    assert_container::<HeapBytes<6>, _>(&fk, true, "f_key_user(Some(true))");
    assert_eq!(&*fk, b"secret");

    let csk = cs_key_user!(
        { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
        { force_heap: Some(true) },
        "123"
    );
    assert_container::<HeapCStr<4>, _>(&csk, true, "cs_key_user(Some(true))");
    assert_eq!(&*csk, c"123");

    let wk = w_key_user!(
        { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
        { force_heap: Some(true) },
        "123"
    );
    assert_container::<HeapU16<3>, _>(&wk, true, "w_key_user(Some(true))");
    assert_eq!(&*wk, &[0x31u16, 0x32, 0x33][..]);

    let cwk = cw_key_user!(
        { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
        { force_heap: Some(true) },
        "123"
    );
    assert_container::<HeapU16<4>, _>(&cwk, true, "cw_key_user(Some(true))");
    assert_eq!(&*cwk, &[0x31u16, 0x32, 0x33, 0][..]);
}

#[test]
fn test_user_force_stack_matrix() {
    use obfstr2::{b_user, cs_user, cw_user, f_user, s_user, w_user};

    const LONG_STR: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const LONG_BYTES: &[u8; 64] =
        b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    // ── 2a. *_user! + force_heap: false（64B 长载荷）──
    let s = s_user!(
        { prefer_stack: false, force_heap: false },
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_container::<StackStr<64>, _>(&s, false, "s_user(force_heap: false)");
    assert_eq!(&*s, LONG_STR);

    let b = b_user!(
        { prefer_stack: false, force_heap: false },
        b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_container::<StackBytes<64>, _>(&b, false, "b_user(force_heap: false)");
    assert_eq!(&*b, LONG_BYTES);

    let f = f_user!({ prefer_stack: false, force_heap: false }, "assets/fixture.bin");
    assert_container::<StackBytes<6>, _>(&f, false, "f_user(force_heap: false)");
    assert_eq!(&*f, b"secret");

    let cs = cs_user!(
        { prefer_stack: false, force_heap: false },
        c"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_container::<StackCStr<65>, _>(&cs, false, "cs_user(force_heap: false)");
    assert_eq!(cs.as_bytes(), LONG_BYTES);

    let w = w_user!(
        { prefer_stack: false, force_heap: false },
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_container::<StackU16<64>, _>(&w, false, "w_user(force_heap: false)");
    assert_eq!(w.len(), 64);

    let cw = cw_user!(
        { prefer_stack: false, force_heap: false },
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_container::<StackU16<65>, _>(&cw, false, "cw_user(force_heap: false)");
    assert_eq!(cw.len(), 65);

    // ── 2b. *_key_user! + force_heap: Some(false)（64B 长载荷）──
    let sk = s_key_user!(
        { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
        { prefer_stack: false, force_heap: Some(false) },
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_container::<StackStr<64>, _>(&sk, false, "s_key_user(Some(false))");
    assert_eq!(&*sk, LONG_STR);

    let bk = b_key_user!(
        { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
        { prefer_stack: false, force_heap: Some(false) },
        b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_container::<StackBytes<64>, _>(&bk, false, "b_key_user(Some(false))");
    assert_eq!(&*bk, LONG_BYTES);

    let fk = f_key_user!(
        { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
        { prefer_stack: false, force_heap: Some(false) },
        "assets/fixture.bin"
    );
    assert_container::<StackBytes<6>, _>(&fk, false, "f_key_user(Some(false))");
    assert_eq!(&*fk, b"secret");

    let csk = cs_key_user!(
        { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
        { prefer_stack: false, force_heap: Some(false) },
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_container::<StackCStr<65>, _>(&csk, false, "cs_key_user(Some(false))");
    assert_eq!(csk.as_bytes(), LONG_BYTES);

    let wk = w_key_user!(
        { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
        { prefer_stack: false, force_heap: Some(false) },
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_container::<StackU16<64>, _>(&wk, false, "w_key_user(Some(false))");
    assert_eq!(wk.len(), 64);

    let cwk = cw_key_user!(
        { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
        { prefer_stack: false, force_heap: Some(false) },
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_container::<StackU16<65>, _>(&cwk, false, "cw_key_user(Some(false))");
    assert_eq!(cwk.len(), 65);
}

#[test]
#[cfg(feature = "force_heap")]
fn test_feature_force_heap_matrix() {
    use obfstr2::{b_user, cs_user, cw_user, f_user, s_user, w_user};

    // s 家族（3B 短串 + 空串）
    assert_container::<HeapStr<0>, _>(&s1!(""), true, "s1(empty)");
    assert_container::<HeapStr<3>, _>(&s1!("123"), true, "s1");
    assert_container::<HeapStr<3>, _>(&s2!("123"), true, "s2");
    assert_container::<HeapStr<3>, _>(&s3!("123"), true, "s3");
    assert_container::<HeapStr<3>, _>(&s_user!({ force_heap: None }, "123"), true, "s_user(None)");
    assert_container::<HeapStr<3>, _>(
        &s_key!({ build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() }, "123"),
        true,
        "s_key",
    );

    // b 家族
    assert_container::<HeapBytes<0>, _>(&b1!(b""), true, "b1(empty)");
    assert_container::<HeapBytes<3>, _>(&b1!(b"123"), true, "b1");
    assert_container::<HeapBytes<3>, _>(&b2!(b"123"), true, "b2");
    assert_container::<HeapBytes<3>, _>(&b3!(b"123"), true, "b3");
    assert_container::<HeapBytes<3>, _>(
        &b_user!({ force_heap: None }, b"123"),
        true,
        "b_user(None)",
    );
    assert_container::<HeapBytes<3>, _>(
        &b_key!({ build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() }, b"123"),
        true,
        "b_key",
    );

    // f 家族（assets/fixture.bin = 6B）
    assert_container::<HeapBytes<6>, _>(&f1!("assets/fixture.bin"), true, "f1");
    assert_container::<HeapBytes<6>, _>(&f2!("assets/fixture.bin"), true, "f2");
    assert_container::<HeapBytes<6>, _>(&f3!("assets/fixture.bin"), true, "f3");
    assert_container::<HeapBytes<6>, _>(
        &f_user!({ force_heap: None }, "assets/fixture.bin"),
        true,
        "f_user(None)",
    );
    assert_container::<HeapBytes<6>, _>(
        &f_key!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            "assets/fixture.bin"
        ),
        true,
        "f_key",
    );

    // cs 家族（3B + 1 NUL = 4B）
    assert_container::<HeapCStr<1>, _>(&cs1!(c""), true, "cs1(empty)");
    assert_container::<HeapCStr<4>, _>(&cs1!(c"123"), true, "cs1");
    assert_container::<HeapCStr<4>, _>(&cs2!(c"123"), true, "cs2");
    assert_container::<HeapCStr<4>, _>(&cs3!(c"123"), true, "cs3");
    assert_container::<HeapCStr<4>, _>(
        &cs_user!({ force_heap: None }, c"123"),
        true,
        "cs_user(None)",
    );
    assert_container::<HeapCStr<4>, _>(
        &cs_key!({ build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() }, c"123"),
        true,
        "cs_key",
    );

    // w 家族（3 UTF-16 码元）
    assert_container::<HeapU16<0>, _>(&w1!(""), true, "w1(empty)");
    assert_container::<HeapU16<3>, _>(&w1!("123"), true, "w1");
    assert_container::<HeapU16<3>, _>(&w2!("123"), true, "w2");
    assert_container::<HeapU16<3>, _>(&w3!("123"), true, "w3");
    assert_container::<HeapU16<3>, _>(&w_user!({ force_heap: None }, "123"), true, "w_user(None)");
    assert_container::<HeapU16<3>, _>(
        &w_key!({ build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() }, "123"),
        true,
        "w_key",
    );

    // cw 家族（3 UTF-16 码元 + 1 NUL = 4）
    assert_container::<HeapU16<1>, _>(&cw1!(""), true, "cw1(empty)");
    assert_container::<HeapU16<4>, _>(&cw1!("123"), true, "cw1");
    assert_container::<HeapU16<4>, _>(&cw2!("123"), true, "cw2");
    assert_container::<HeapU16<4>, _>(&cw3!("123"), true, "cw3");
    assert_container::<HeapU16<4>, _>(
        &cw_user!({ force_heap: None }, "123"),
        true,
        "cw_user(None)",
    );
    assert_container::<HeapU16<4>, _>(
        &cw_key!({ build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() }, "123"),
        true,
        "cw_key",
    );
}

#[test]
#[cfg(feature = "force_stack")]
fn test_feature_force_stack_matrix() {
    use obfstr2::{b_user, cs_user, cw_user, f_user, s_user, w_user};

    // s 家族（64B 长载荷）
    assert_container::<StackStr<64>, _>(
        &s1!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        false,
        "s1(64B)",
    );
    assert_container::<StackStr<64>, _>(
        &s2!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        false,
        "s2(64B)",
    );
    assert_container::<StackStr<64>, _>(
        &s3!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        false,
        "s3(64B)",
    );
    assert_container::<StackStr<64>, _>(
        &s_user!(
            { prefer_stack: false, force_heap: None },
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ),
        false,
        "s_user(None, 64B)",
    );
    assert_container::<StackStr<64>, _>(
        &s_key!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ),
        false,
        "s_key(64B)",
    );

    // b 家族（64B 长载荷）
    assert_container::<StackBytes<64>, _>(
        &b1!(b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        false,
        "b1(64B)",
    );
    assert_container::<StackBytes<64>, _>(
        &b2!(b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        false,
        "b2(64B)",
    );
    assert_container::<StackBytes<64>, _>(
        &b3!(b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        false,
        "b3(64B)",
    );
    assert_container::<StackBytes<64>, _>(
        &b_user!(
            { prefer_stack: false, force_heap: None },
            b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ),
        false,
        "b_user(None, 64B)",
    );
    assert_container::<StackBytes<64>, _>(
        &b_key!(
            { build_key: "obfstr2-test-key-0123456789abcdef", run_key: run_key() },
            b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ),
        false,
        "b_key(64B)",
    );

    // f 家族
    assert_container::<StackBytes<6>, _>(&f1!("assets/fixture.bin"), false, "f1");
    assert_container::<StackBytes<6>, _>(&f2!("assets/fixture.bin"), false, "f2");
    assert_container::<StackBytes<6>, _>(&f3!("assets/fixture.bin"), false, "f3");
    assert_container::<StackBytes<6>, _>(
        &f_user!({ prefer_stack: false, force_heap: None }, "assets/fixture.bin"),
        false,
        "f_user(None)",
    );

    // cs 家族（64B + 1 NUL = 65B）
    assert_container::<StackCStr<65>, _>(
        &cs1!(c"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        false,
        "cs1(65B)",
    );
    assert_container::<StackCStr<65>, _>(
        &cs2!(c"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        false,
        "cs2(65B)",
    );
    assert_container::<StackCStr<65>, _>(
        &cs3!(c"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        false,
        "cs3(65B)",
    );
    assert_container::<StackCStr<65>, _>(
        &cs_user!(
            { prefer_stack: false, force_heap: None },
            c"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ),
        false,
        "cs_user(None, 65B)",
    );

    // w 家族（64 UTF-16 码元 = 128B）
    assert_container::<StackU16<64>, _>(
        &w1!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        false,
        "w1(64u16)",
    );
    assert_container::<StackU16<64>, _>(
        &w2!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        false,
        "w2(64u16)",
    );
    assert_container::<StackU16<64>, _>(
        &w3!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        false,
        "w3(64u16)",
    );
    assert_container::<StackU16<64>, _>(
        &w_user!(
            { prefer_stack: false, force_heap: None },
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ),
        false,
        "w_user(None, 64u16)",
    );

    // cw 家族（64 UTF-16 码元 + 1 NUL = 65u16）
    assert_container::<StackU16<65>, _>(
        &cw1!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        false,
        "cw1(65u16)",
    );
    assert_container::<StackU16<65>, _>(
        &cw2!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        false,
        "cw2(65u16)",
    );
    assert_container::<StackU16<65>, _>(
        &cw3!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        false,
        "cw3(65u16)",
    );
    assert_container::<StackU16<65>, _>(
        &cw_user!(
            { prefer_stack: false, force_heap: None },
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ),
        false,
        "cw_user(None, 65u16)",
    );
}

#[test]
#[cfg(not(any(feature = "force_heap", feature = "force_stack")))]
fn test_default_heuristic_container_matrix() {
    use obfstr2::{b_user, cs_user, cw_user, f_user, s_user, w_user};

    // <= 16B 短载荷：无论是否开启 alloc，默认启发式均保证 100% 走栈
    assert_container::<StackStr<16>, _>(&s3!("0123456789abcdef"), false, "s3(16B boundary)");
    assert_container::<StackBytes<16>, _>(&b3!(b"0123456789abcdef"), false, "b3(16B boundary)");
    assert_container::<StackBytes<6>, _>(&f3!("assets/fixture.bin"), false, "f3(6B)");
    assert_container::<StackCStr<16>, _>(&cs3!(c"0123456789abcde"), false, "cs3(15B+NUL=16B)");
    assert_container::<StackU16<8>, _>(&w3!("01234567"), false, "w3(8u16=16B)");
    assert_container::<StackU16<8>, _>(&cw3!("0123456"), false, "cw3(7u16+NUL=8u16=16B)");

    assert_container::<StackStr<3>, _>(
        &s_user!({ force_heap: None }, "123"),
        false,
        "s_user(None)",
    );
    assert_container::<StackBytes<3>, _>(
        &b_user!({ force_heap: None }, b"123"),
        false,
        "b_user(None)",
    );
    assert_container::<StackBytes<6>, _>(
        &f_user!({ force_heap: None }, "assets/fixture.bin"),
        false,
        "f_user(None)",
    );
    assert_container::<StackCStr<4>, _>(
        &cs_user!({ force_heap: None }, c"123"),
        false,
        "cs_user(None)",
    );
    assert_container::<StackU16<3>, _>(
        &w_user!({ force_heap: None }, "123"),
        false,
        "w_user(None)",
    );
    assert_container::<StackU16<4>, _>(
        &cw_user!({ force_heap: None }, "123"),
        false,
        "cw_user(None)",
    );

    #[cfg(not(feature = "alloc"))]
    {
        assert_container::<StackStr<32>, _>(
            &s3!("0123456789abcdef0123456789abcdef"),
            false,
            "s3(no_alloc, 32B)",
        );
        assert_container::<StackBytes<32>, _>(
            &b3!(b"0123456789abcdef0123456789abcdef"),
            false,
            "b3(no_alloc, 32B)",
        );
        assert_container::<StackCStr<33>, _>(
            &cs3!(c"0123456789abcdef0123456789abcdef"),
            false,
            "cs3(no_alloc, 33B)",
        );
        assert_container::<StackU16<32>, _>(
            &w3!("0123456789abcdef0123456789abcdef"),
            false,
            "w3(no_alloc, 32u16)",
        );
        assert_container::<StackU16<33>, _>(
            &cw3!("0123456789abcdef0123456789abcdef"),
            false,
            "cw3(no_alloc, 33u16)",
        );
    }
}
