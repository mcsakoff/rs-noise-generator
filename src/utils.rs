
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
