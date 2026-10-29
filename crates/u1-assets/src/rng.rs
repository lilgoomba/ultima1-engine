//! The game's pseudo-random number generator, re-implemented from a prose
//! description (SPEC §6.1, "verified from original executable").
//!
//! State is one 16-bit word. Each draw adds the constant 0x9248 (wrapping),
//! rotates the word right by three bits, keeps the new state, and takes its
//! low ten bits as the raw value `r` (0..=1023). A ranged draw `range(lo, hi)`
//! clamps `hi` to 255, then repeatedly subtracts `hi - lo` from `r` while `r`
//! exceeds it, and returns `lo + r` truncated to a byte. The result lies in
//! `lo..=hi` (both ends inclusive) and is not uniform.

/// Ultima I style 16-bit generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct U1Rng {
    pub state: u16,
}

impl U1Rng {
    pub fn new(seed: u16) -> Self {
        U1Rng { state: seed }
    }

    /// Replace the state (the game's "seed" operation stores the word as is).
    pub fn seed(&mut self, seed: u16) {
        self.state = seed;
    }

    /// Advance and return the raw 10-bit value.
    pub fn next_raw(&mut self) -> u16 {
        self.state = self.state.wrapping_add(0x9248).rotate_right(3);
        self.state & 0x03FF
    }

    /// Inclusive ranged draw as described in the module docs.
    pub fn range(&mut self, lo: u16, hi: u16) -> u16 {
        let mut r = self.next_raw();
        let hi = hi.min(255);
        let span = hi.wrapping_sub(lo);
        if span == 0 {
            return lo & 0xFF;
        }
        while r > span {
            r -= span;
        }
        (r.wrapping_add(lo)) & 0xFF
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_is_add_then_rotate() {
        let mut g = U1Rng::new(0);
        // 0 + 0x9248 = 0x9248; ror 3 -> 0x1249; low 10 bits = 0x249
        assert_eq!(g.next_raw(), 0x249);
        assert_eq!(g.state, 0x1249);
        // 0x1249 + 0x9248 = 0xA491; ror 3 -> 0x3492 (low bits 001 -> top)
        g.next_raw();
        assert_eq!(g.state, 0x3492);
    }

    #[test]
    fn range_is_inclusive_and_bounded() {
        let mut g = U1Rng::new(1234);
        let mut seen = [false; 5];
        for _ in 0..5000 {
            let v = g.range(1, 4);
            assert!((1..=4).contains(&v));
            seen[v as usize] = true;
        }
        assert!(seen[1..].iter().all(|&s| s));
        for _ in 0..1000 {
            assert!(g.range(1, 1000) <= 255, "hi is clamped to 255");
        }
    }
}
