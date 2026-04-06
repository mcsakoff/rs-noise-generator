pub(crate) fn split_chunk(chunk: Vec<f64>) -> (Vec<f64>, Vec<f64>) {
    let (chunk0_0, chunk0_1) = chunk.split_at_checked(chunk.len() / 2).unwrap();
    (chunk0_0.to_vec(), chunk0_1.to_vec())
}

pub(crate) fn overlap_chunks(chunk1: &[f64], chunk2: &[f64]) -> Vec<f64> {
    assert_eq!(chunk1.len(), chunk2.len());
    chunk1
        .iter()
        .zip(chunk2.iter())
        .map(|(a, b)| a + b)
        .collect()
}

pub(crate) fn amplitude_to_db(amplitude: f64) -> f64 {
    if amplitude > 0.0 {
        20.0 * amplitude.log10()
    } else {
        f64::NEG_INFINITY
    }
}

pub(crate) fn db_to_amplitude(db: f64) -> f64 {
    10.0f64.powf(db / 20.0)
}
