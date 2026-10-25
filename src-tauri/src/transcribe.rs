pub const SAMPLE_RATE: usize = 16_000;
pub const TARGET_CHUNK_SECS: f32 = 30.0;
pub const SEARCH_WINDOW_SECS: f32 = 3.0;
const FRAME_SECS: f32 = 0.03;

pub fn split_chunks(samples: &[f32]) -> Vec<(usize, usize)> {
    if samples.is_empty() {
        return Vec::new();
    }

    let target = (TARGET_CHUNK_SECS * SAMPLE_RATE as f32) as usize;
    let search = (SEARCH_WINDOW_SECS * SAMPLE_RATE as f32) as usize;
    let frame = (FRAME_SECS * SAMPLE_RATE as f32) as usize;
    let mut chunks = Vec::new();
    let mut start = 0_usize;

    while samples.len() - start > target + search {
        let low = start + target - search;
        let high = start + target + search;
        let mut best = start + target;
        let mut best_energy = f32::MAX;
        let mut offset = low;
        while offset + frame <= high {
            let energy = samples[offset..offset + frame]
                .iter()
                .map(|sample| sample * sample)
                .sum::<f32>();
            if energy < best_energy {
                best_energy = energy;
                best = offset + frame;
            }
            offset += frame;
        }
        chunks.push((start, best));
        start = best;
    }

    chunks.push((start, samples.len()));
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk_secs(chunk: (usize, usize)) -> f32 {
        (chunk.1 - chunk.0) as f32 / SAMPLE_RATE as f32
    }

    #[test]
    fn short_audio_stays_one_chunk() {
        let samples = vec![0.1; SAMPLE_RATE * 12];
        assert_eq!(split_chunks(&samples), vec![(0, samples.len())]);
    }

    #[test]
    fn empty_audio_has_no_chunks() {
        assert!(split_chunks(&[]).is_empty());
    }

    #[test]
    fn long_audio_is_contiguous_and_bounded() {
        let samples = vec![0.2; SAMPLE_RATE * 65];
        let chunks = split_chunks(&samples);

        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0].0, 0);
        assert_eq!(chunks.last().unwrap().1, samples.len());
        for pair in chunks.windows(2) {
            assert_eq!(pair[0].1, pair[1].0, "chunks must not overlap or lose samples");
        }
        for chunk in &chunks[..chunks.len() - 1] {
            assert!(
                (27.0..=33.0).contains(&chunk_secs(*chunk)),
                "unexpected chunk duration: {}",
                chunk_secs(*chunk)
            );
        }
    }

    #[test]
    fn prefers_quiet_split_point() {
        let mut samples = vec![0.8_f32; SAMPLE_RATE * 40];
        let quiet_start = (29.0 * SAMPLE_RATE as f32) as usize;
        let quiet_end = quiet_start + (0.3 * SAMPLE_RATE as f32) as usize;
        for sample in &mut samples[quiet_start..quiet_end] {
            *sample = 0.0001;
        }

        let chunks = split_chunks(&samples);
        let first_split_secs = chunks[0].1 as f32 / SAMPLE_RATE as f32;

        assert!(
            (28.8..=30.2).contains(&first_split_secs),
            "split was at {first_split_secs}s"
        );
    }
}
