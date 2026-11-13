// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 根 crate 冒烟测试：全部 24 个宏的小载荷 roundtrip。
//!
//! 直接调用宏做断言，无需 dyntest 现场建工程，毫秒级。
use obfstr2::{
    b1, b2, b3, cs1, cs2, cs3, cw1, cw2, cw3, f1, f2, f3, fl1, fl2, fl3, i1, i2, i3, s1, s2, s3,
    w1, w2, w3,
};

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
    assert_eq!(fl2!(1e-45f32).to_bits(), (1e-45f32).to_bits());
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
