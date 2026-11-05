// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Private buffer names: preserve the prepared Boost UUID stream and lazy owner.
use chacha20::ChaCha12Rng;
use rand_core::{Rng, SeedableRng};

pub(super) struct NameGenerator {
    random: Option<ChaCha12Rng>,
}

impl NameGenerator {
    pub(super) const fn new() -> Self {
        Self { random: None }
    }

    pub(super) fn next<E>(
        &mut self,
        seed: impl FnOnce() -> Result<[u8; 48], E>,
    ) -> Result<[u8; 53], E> {
        if self.random.is_none() {
            let seed = seed()?;
            let mut key = [0; 32];
            key.copy_from_slice(&seed[..32]);
            let mut nonce = [0; 8];
            nonce.copy_from_slice(&seed[40..]);
            // Boost's SeedSeq fills twelve u32 words, then resets the two
            // counter words (seed bytes32..40) and retains the final nonce.
            let mut random = ChaCha12Rng::from_seed(key);
            random.set_stream(u64::from_le_bytes(nonce));
            self.random = Some(random);
        }
        // Initialization commits only after a complete successful seed. The
        // owner survives all following names, matching the source static.
        let random = self.random.as_mut().expect("UUID generator initialized");
        let mut bytes = [0; 16];
        random.fill_bytes(&mut bytes);
        bytes[6] = (bytes[6] & 15) | 64;
        bytes[8] = (bytes[8] & 63) | 128;
        let mut name = [0; 53];
        name[..16].copy_from_slice(b"maafw-wl-buffer-");
        let digits = b"0123456789abcdef";
        let mut cursor = 16;
        for (index, byte) in bytes.into_iter().enumerate() {
            if [4, 6, 8, 10].contains(&index) {
                name[cursor] = b'-';
                cursor += 1;
            }
            name[cursor] = digits[usize::from(byte >> 4)];
            name[cursor + 1] = digits[usize::from(byte & 15)];
            cursor += 2;
        }
        Ok(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vectors() -> Vec<(u32, [u8; 48], u32, String)> {
        let path = std::env::var_os("MFS_P1_WLR_UUID_VECTORS")
            .expect("explicit original Boost UUID vector file");
        let text = std::fs::read_to_string(path).unwrap();
        let rows: Vec<_> = text
            .lines()
            .map(|line| {
                let fields: Vec<_> = line.split('\t').collect();
                assert_eq!(fields.len(), 4);
                assert_eq!(fields[1].len(), 96);
                let mut seed = [0; 48];
                for (index, byte) in seed.iter_mut().enumerate() {
                    *byte = u8::from_str_radix(&fields[1][index * 2..index * 2 + 2], 16).unwrap();
                }
                (
                    fields[0].parse().unwrap(),
                    seed,
                    fields[2].parse().unwrap(),
                    fields[3].to_owned(),
                )
            })
            .collect();
        assert_eq!(rows.len(), 2048);
        rows
    }

    #[test]
    #[ignore = "requires the separately executed original Boost vector corpus"]
    fn compares_original_boost_stream_seed_layout_and_initialization_retry() {
        let rows = vectors();
        for scenario in 0..8 {
            let mut generator = NameGenerator::new();
            let mut seeds = 0;
            for draw in 0..256 {
                let (observed, seed, index, expected) = &rows[scenario * 256 + draw];
                assert_eq!((*observed, *index), (scenario as u32, draw as u32));
                let actual = generator
                    .next(|| {
                        seeds += 1;
                        Ok::<_, ()>(*seed)
                    })
                    .unwrap();
                assert_eq!(actual[52], 0);
                assert_eq!(&actual[..52], expected.as_bytes());
            }
            assert_eq!(seeds, 1);
        }
        let mut retry = NameGenerator::new();
        for code in [1, 5, 12] {
            assert_eq!(retry.next(|| Err(code)), Err(code));
        }
        for (_, seed, _, expected) in &rows[512..768] {
            let actual = retry.next(|| Ok::<_, u32>(*seed)).unwrap();
            assert_eq!(&actual[..52], expected.as_bytes());
        }
        println!(
            "\nRust original Boost UUID seeds=8 paired_draws=2048 retry_draws=256 failed_initializations=3 warm_reseeds=0 passed"
        );
    }

    #[test]
    #[ignore = "requires original Linux/libstdc++ default UUID entropy observations"]
    fn compares_original_default_entropy_seed_failure_and_warm_state() {
        let root = std::path::PathBuf::from(std::env::var_os("MFS_P1_WLR_UUID_ENTROPY").unwrap());
        let rows = vectors();
        let failures = std::fs::read_to_string(root.join("retry.tsv.failures")).unwrap();
        let failures: Vec<_> = failures.lines().collect();
        assert_eq!(failures.len(), 3);
        for (line, expected) in failures
            .into_iter()
            .zip([(1, 1, 1), (2, 5, 6), (3, 12, 18)])
        {
            let values: Vec<u32> = line.split('\t').map(|v| v.parse().unwrap()).collect();
            assert_eq!(values.len(), 5);
            assert_eq!((values[0], values[1], values[2]), expected);
            assert!(values[3] > 0);
            assert_eq!(values[3], values[4]);
        }
        for mode in ["real", "controlled", "retry"] {
            let text = std::fs::read_to_string(root.join(format!("{mode}.tsv"))).unwrap();
            let records: Vec<_> = text.lines().collect();
            assert_eq!(records.len(), 256);
            let mut names = std::collections::BTreeSet::new();
            for (draw, record) in records.into_iter().enumerate() {
                let fields: Vec<_> = record.split('\t').collect();
                assert_eq!(fields.len(), 3);
                assert_eq!(fields[0].parse::<usize>().unwrap(), draw);
                assert_eq!(
                    fields[2].parse::<u32>().unwrap(),
                    if mode == "retry" { 30 } else { 12 }
                );
                assert_eq!(fields[1].len(), 52);
                assert!(fields[1].starts_with("maafw-wl-buffer-"));
                assert!(names.insert(fields[1]));
                if mode != "real" {
                    assert_eq!(fields[1], rows[512 + draw].3);
                }
            }
        }
        println!(
            "\nRust original default UUID modes=3 source_draws=768 controlled_pairs=512 seed_failures=3 source_warm_entropy_calls=0 passed"
        );
    }

    #[test]
    fn retries_failed_initialization_and_never_reseeds_a_live_generator() {
        let mut generator = NameGenerator::new();
        for code in [1, 5, 12] {
            assert_eq!(generator.next(|| Err(code)), Err(code));
        }
        let first = generator.next(|| Ok::<_, i32>([0; 48])).unwrap();
        let second = generator.next(|| Err(99)).unwrap();
        let mut control = NameGenerator::new();
        assert_eq!(control.next(|| Ok::<_, i32>([0; 48])).unwrap(), first);
        assert_eq!(control.next(|| Err(100)).unwrap(), second);
        assert_ne!(first, second);
        assert_eq!(first[52], 0);
    }

    #[test]
    fn discards_seed_counter_words_and_retains_the_nonce() {
        let mut original = NameGenerator::new();
        let mut changed_counter = NameGenerator::new();
        let mut changed_nonce = NameGenerator::new();
        let mut counter_seed = [0; 48];
        counter_seed[32..40].fill(255);
        let mut nonce_seed = [0; 48];
        nonce_seed[40..].fill(255);
        let first = original.next(|| Ok::<_, ()>([0; 48])).unwrap();
        assert_eq!(
            changed_counter.next(|| Ok::<_, ()>(counter_seed)).unwrap(),
            first
        );
        assert_ne!(
            changed_nonce.next(|| Ok::<_, ()>(nonce_seed)).unwrap(),
            first
        );
    }
}
