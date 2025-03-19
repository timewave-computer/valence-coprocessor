use valence_coprocessor_core::{Blake3Hasher, Hasher as _};

fn main() {
    let context = "poem";
    let target = 0b01000000u8;

    for i in 0..u8::MAX {
        for j in 0..u8::MAX {
            for k in 0..u8::MAX {
                let collision = [i, j, k];
                let key = Blake3Hasher::key(context, &collision);

                if (key[0] & 0b11100000u8) == target {
                    panic!("{:x?}", collision);
                }
            }
        }
    }

    /*
    let collision = [0, 0, 2];

    let key = Blake3Hasher::key(context, data);
    let keyp = Blake3Hasher::key(context, &collision);

    println!("{:x?}", collision);

    assert_eq!(key[0] >> 7, keyp[0] >> 7);
    assert_eq!(key[0] >> 6, keyp[0] >> 6);
    assert_ne!(key[0] >> 5, keyp[0] >> 5);
    */
}
