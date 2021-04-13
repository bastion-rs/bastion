//! Utility functions for mathematical operations

/// Generates a random number in `0..n`.
pub fn random(n: u32) -> u32 {
    use std::cell::Cell;
    use std::num::Wrapping;

    thread_local! {
        static RNG: Cell<Wrapping<u32>> = Cell::new(Wrapping(0x5f3759df));
    }

    RNG.with(|rng| {
        // This is the 32-bit variant of Xorshift.
        //
        // Source: https://en.wikipedia.org/wiki/Xorshift
        let mut x = rng.get();
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        rng.set(x);

        // This is a fast alternative to `x % n`.
        //
        // Author: Daniel Lemire
        // Source: https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/
        ((x.0 as u64).wrapping_mul(n as u64) >> 32) as u32
    })
}

/// Simple linear Knuth shuffle
pub fn shuffle_linear<T>(v: &mut Vec<T>) -> &mut Vec<T> {
    let l = v.len();
    for n in 0..l {
        let i = random((l - n) as u32) as usize;
        v.swap(i, l - n - 1);
    }
    v
}
