/// Version-one deterministic PRNG used by game setup.
///
/// The algorithm is deliberately owned by the engine so a dependency upgrade
/// cannot silently make old seeds produce different replays.
#[derive(Clone, Debug)]
pub(crate) struct ReplayRng {
    state: u64,
}

impl ReplayRng {
    pub(crate) const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    pub(crate) fn shuffle<T>(&mut self, values: &mut [T]) {
        for upper in (1..values.len()).rev() {
            let index = self.index_inclusive(upper);
            values.swap(upper, index);
        }
    }

    fn index_inclusive(&mut self, upper: usize) -> usize {
        let range = u64::try_from(upper)
            .expect("slice indexes fit in u64")
            .checked_add(1)
            .expect("slice length fits in u64");
        let threshold = range.wrapping_neg() % range;
        loop {
            let value = self.next_u64();
            if value >= threshold {
                return usize::try_from(value % range).expect("result is at most a slice index");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ReplayRng;

    #[test]
    fn shuffle_is_stable_for_a_known_seed() {
        let mut values = [0, 1, 2, 3, 4, 5, 6, 7];
        ReplayRng::new(42).shuffle(&mut values);
        assert_eq!(values, [3, 1, 6, 2, 4, 0, 7, 5]);
    }
}
