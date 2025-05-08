use core::marker::PhantomData;

use ark_bn254::{g1, g2};
use ark_ec::short_weierstrass::Affine;
use ark_ff::{BigInt, Fp, QuadExtField};

pub const ALPHA: Affine<g1::Config> = Affine::new_unchecked(
    Fp(
        BigInt([
            0x1cda4b92a110e17,
            0xcffc2e4047ab6bc3,
            0x3cde11e11a4beee9,
            0x6eb02980f5c5ecf,
        ]),
        PhantomData,
    ),
    Fp(
        BigInt([
            0x314b2cdfd60e8b1e,
            0x3ce442175a29b036,
            0x117f35aacb7e0278,
            0x64651da4e3b6592,
        ]),
        PhantomData,
    ),
);

pub const BETA: Affine<g2::Config> = Affine::new_unchecked(
    QuadExtField {
        c0: Fp(
            BigInt([
                0xd468a34a006727e0,
                0x7929c199b404500c,
                0xf50481fea7677768,
                0x2af3765a06731e7e,
            ]),
            PhantomData,
        ),
        c1: Fp(
            BigInt([
                0x33d21c64c9663e58,
                0xc80029690eeb9f1b,
                0x724024e6d9176abf,
                0x29ca902f010d85bf,
            ]),
            PhantomData,
        ),
    },
    QuadExtField {
        c0: Fp(
            BigInt([
                0x8f6e83a7eb149136,
                0x72fc87ef376010cd,
                0xa4c612c7f834fa24,
                0x15758aa9a153e1da,
            ]),
            PhantomData,
        ),
        c1: Fp(
            BigInt([
                0x18240d7aa9f84834,
                0xb9b640fcd38875a8,
                0x2d59598c235bcaf9,
                0x2ac2e703abccc6f4,
            ]),
            PhantomData,
        ),
    },
);

pub const GAMMA: Affine<g2::Config> = Affine::new_unchecked(
    QuadExtField {
        c0: Fp(
            BigInt([
                0x8e83b5d102bc2026,
                0xdceb1935497b0172,
                0xfbb8264797811adf,
                0x19573841af96503b,
            ]),
            PhantomData,
        ),
        c1: Fp(
            BigInt([
                0xafb4737da84c6140,
                0x6043dd5a5802d8c4,
                0x9e950fc52a02f86,
                0x14fef0833aea7b6b,
            ]),
            PhantomData,
        ),
    },
    QuadExtField {
        c0: Fp(
            BigInt([
                0x619dfa9d886be9f6,
                0xfe7fd297f59e9b78,
                0xff9e1a62231b7dfe,
                0x28fd7eebae9e4206,
            ]),
            PhantomData,
        ),
        c1: Fp(
            BigInt([
                0x64095b56c71856ee,
                0xdc57f922327d3cbb,
                0x55f935be33351076,
                0xda4a0e693fd6482,
            ]),
            PhantomData,
        ),
    },
);

pub const DELTA: Affine<g2::Config> = Affine::new_unchecked(
    QuadExtField {
        c0: Fp(
            BigInt([
                0x2f4143aa35f70661,
                0x95519faed923b6a3,
                0x6d211cc40abe7c62,
                0x2be152575491fa3f,
            ]),
            PhantomData,
        ),
        c1: Fp(
            BigInt([
                0xa908f86c13ae2bef,
                0x3b217eef90edf44a,
                0xf9eb34313b912ef3,
                0xabe0bdd161445c0,
            ]),
            PhantomData,
        ),
    },
    QuadExtField {
        c0: Fp(
            BigInt([
                0xc24e2faa87c4d4e6,
                0x72597c75e8efdf8d,
                0xa7f60d690b28c7dd,
                0x27745653a044edef,
            ]),
            PhantomData,
        ),
        c1: Fp(
            BigInt([
                0x3e4e30842f4be14b,
                0x511d8d5bfe1c3066,
                0xa36967b7fcb18a26,
                0x9a9d2b7c1579e09,
            ]),
            PhantomData,
        ),
    },
);

pub const GAMMA_ABC: &[Affine<g1::Config>] = &[
    Affine::new_unchecked(
        Fp(
            BigInt([
                0xad49da92f2420db4,
                0xa43b47c6c62b1f14,
                0x25daa8ffe38262dd,
                0x7adb1835208462d,
            ]),
            PhantomData,
        ),
        Fp(
            BigInt([
                0xcd2ef4cb574ff877,
                0x22ad811c9314c546,
                0x15a8da7b81178e73,
                0xcffd860472945ea,
            ]),
            PhantomData,
        ),
    ),
    Affine::new_unchecked(
        Fp(
            BigInt([
                0x785e7b3ee764a6a,
                0xcc58bbfbabc4094b,
                0x9e98154910a7bdb3,
                0x2985e25eb9b76af2,
            ]),
            PhantomData,
        ),
        Fp(
            BigInt([
                0xc510377656c123aa,
                0x20f5602212204639,
                0x707237ddd35925d3,
                0xa92ad5fa4b05d1c,
            ]),
            PhantomData,
        ),
    ),
    Affine::new_unchecked(
        Fp(
            BigInt([
                0x224c096aa0adce1,
                0xf3b4f99420844bfe,
                0x28b3b639af9cf671,
                0x181c5bea75269686,
            ]),
            PhantomData,
        ),
        Fp(
            BigInt([
                0x6f8860844145d0e0,
                0xa0ab4379090e44a7,
                0xf441eade4d941056,
                0x217917ceab9f87c4,
            ]),
            PhantomData,
        ),
    ),
];

// Helpers

fn _print_g1(g1: &Affine<g1::Config>) {
    println!("Affine::new_unchecked(Fp(BigInt([");

    for n in g1.x.0 .0 {
        println!("{:#x},", n);
    }

    println!("]),PhantomData,),Fp(BigInt([");

    for n in g1.y.0 .0 {
        println!("{:#x},", n);
    }

    println!("]),PhantomData,),);");
}

fn _print_g2(g2: &Affine<g2::Config>) {
    println!("Affine::new_unchecked(QuadExtField {{ c0: Fp(BigInt([");

    for n in g2.x.c0.0 .0 {
        println!("{:#x},", n);
    }

    println!("]),PhantomData,),c1: Fp(BigInt([");

    for n in g2.x.c1.0 .0 {
        println!("{:#x},", n);
    }

    println!("]),PhantomData,),}},QuadExtField {{ c0: Fp(BigInt([");

    for n in g2.y.c0.0 .0 {
        println!("{:#x},", n);
    }

    println!("]),PhantomData,),c1: Fp(BigInt([");

    for n in g2.y.c1.0 .0 {
        println!("{:#x},", n);
    }

    println!("]),PhantomData,),}},);");
}
