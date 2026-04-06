#![allow(clippy::cast_precision_loss)]

use crate::utils::db_to_amplitude;
use std::f64::consts::{PI, TAU};
use std::fmt::{Display, Formatter};

pub mod wav;

mod utils;

#[derive(Copy, Clone)]
pub enum NoiseColor {
    White,
    Pink,
    Brownian,
    Blue,
    Violet,
    Grey,
}

impl NoiseColor {
    fn amplitude_fn(&self) -> fn(usize) -> f64 {
        match self {
            Self::White => |_| 1.0,
            Self::Pink => |frq| 1.0 / (frq as f64).sqrt(),
            Self::Brownian => |frq| 1.0 / (frq as f64),
            Self::Blue => |frq| (frq as f64).sqrt(),
            Self::Violet => |frq| frq as f64,
            Self::Grey => |frq| {
                let f = frq as f64;
                // https://en.wikipedia.org/wiki/A-weighting
                // The weighting function R(f) is applied to the amplitude spectrum (not the intensity spectrum)
                // of the unweighted sound level.
                let r_a = |f: f64| {
                    ((12194.0f64).powi(2) * f.powi(4))
                        / ((f.powi(2) + 20.6f64.powi(2))
                            * f64::sqrt(
                                (f.powi(2) + 107.7f64.powi(2)) * (f.powi(2) + 737.9f64.powi(2)),
                            )
                            * (f.powi(2) + (12194.0f64).powi(2)))
                };
                // Invert the gain curve (with some scale for better precision in IFFT)
                0.1 / r_a(f)
            },
        }
    }
}

impl Display for NoiseColor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NoiseColor::White => write!(f, "white"),
            NoiseColor::Pink => write!(f, "pink"),
            NoiseColor::Brownian => write!(f, "brownian"),
            NoiseColor::Blue => write!(f, "blue"),
            NoiseColor::Violet => write!(f, "violet"),
            NoiseColor::Grey => write!(f, "grey"),
        }
    }
}

#[derive(Copy, Clone)]
pub enum WindowFunction {
    None,
    Sine,
    Hann,
    Hamming,
    Triangular,
}

impl WindowFunction {
    fn get_fn(&self) -> fn(usize, usize) -> f64 {
        match self {
            Self::None => |_, _| 1.0,
            Self::Sine => |n, samples| (PI * n as f64 / samples as f64).sin(),
            Self::Hann => |n, samples| (1.0 - (TAU * n as f64 / samples as f64).cos()) / 2.0,
            Self::Hamming => {
                |n, samples| 0.53836 - 0.46164 * (TAU * n as f64 / samples as f64).cos()
            }
            Self::Triangular => |n, samples| 1.0 - (2.0 * n as f64 / samples as f64 - 1.0).abs(),
        }
    }
}

/// Calculates the gain required to normalize the signal to a target level in dBFS.
/// Can be used for Peak or RMS normalization.
#[derive(Copy, Clone)]
pub enum NormalizationDBFS {
    Peak(f64),
    RMS(f64),
}

impl NormalizationDBFS {
    fn calculate_gain(&self, samples: &[f64]) -> f64 {
        match self {
            NormalizationDBFS::Peak(target_db) => {
                let peak: f64 = samples.iter().fold(0.0, |acc, val| acc.max(val.abs()));
                (1.0 / peak) * db_to_amplitude(*target_db)
            }
            NormalizationDBFS::RMS(target_db) => {
                let sqr_sum: f64 = samples.iter().fold(0.0, |acc, val| acc + val.powi(2));
                let rms = (sqr_sum / samples.len() as f64).sqrt();
                db_to_amplitude(*target_db) / rms
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::{overlap_chunks, split_chunk};
    use crate::wav::WavInfo;
    use std::f64::consts::FRAC_1_SQRT_2;

    struct Sine {
        frequency: usize,
        amplitude: f64,
    }

    impl Sine {
        fn new(frequency: usize, amplitude: f64) -> Self {
            Self {
                frequency,
                amplitude,
            }
        }
    }

    fn generate_sines(sines: &[Sine], sample_rate: usize) -> Vec<f64> {
        let mut samples = Vec::with_capacity(sample_rate);
        for n in 0..sample_rate {
            let mut amplitude = 0.0;
            for sine in sines {
                let t = n as f64 / sample_rate as f64;
                amplitude += sine.amplitude * (TAU * sine.frequency as f64 * t).sin();
            }
            samples.push(amplitude);
        }
        samples
    }

    fn measure_meters(samples: &[f64]) -> WavInfo {
        let peak: f64 = samples.iter().fold(0.0, |acc, val| acc.max(val.abs()));
        let sqr_sum: f64 = samples.iter().fold(0.0, |acc, val| acc + val.powi(2));
        let rms = (sqr_sum / samples.len() as f64).sqrt();
        WavInfo { peak, rms }
    }

    fn display_meters(m: &WavInfo) {
        println!("Peak: {} ({} dBFS)", m.peak, m.peak_db());
        println!("RMS: {} ({} dBFS)", m.rms, m.rms_db());
    }

    #[test]
    fn test_measure_meters() {
        let sines = vec![Sine::new(331, 1.0)];

        let samples = generate_sines(&sines, 48000);
        let m = measure_meters(&samples);
        display_meters(&m);
        assert!((m.peak - 1.0).abs() < 0.000001); // 1.0
        assert!((m.rms - FRAC_1_SQRT_2).abs() < 0.0000001); // 1 / sqrt(2)
        assert!(m.peak_db().abs() < 0.00001); // 0 dBFS
        assert!((m.rms_db() - -3.010299956639).abs() < 0.0000001); // -3.01 dBFS
    }

    #[test]
    fn test_normalization() {
        let sines = vec![Sine::new(331, 1.0), Sine::new(1579, 1.0)];

        let samples = generate_sines(&sines, 48000);
        let m = measure_meters(&samples);
        println!("Original signal:");
        display_meters(&m);
        println!();

        // Normalize by Peak
        {
            let target_peak_db = 0.0;
            let peak_gain = NormalizationDBFS::Peak(target_peak_db).calculate_gain(&samples);
            let samples_peak: Vec<f64> = samples.iter().map(|s| s * peak_gain).collect();
            let m = measure_meters(&samples_peak);
            println!("Normalize by Peak to {target_peak_db:0.02} dBFS:");
            println!("Peak gain: {}", peak_gain);
            display_meters(&m);
            println!();
            assert!((m.peak - 10f64.powf(target_peak_db / 20.0)).abs() < 0.000001);
        }

        // Normalize by RMS
        {
            let target_rms_db = -9.0;
            let rms_gain = NormalizationDBFS::RMS(target_rms_db).calculate_gain(&samples);
            let samples_rms: Vec<f64> = samples.iter().map(|s| s * rms_gain).collect();
            let m = measure_meters(&samples_rms);
            println!("Normalize by RMS to {target_rms_db:0.02} dBFS:");
            println!("RMS gain: {}", rms_gain);
            display_meters(&m);
            println!();
            assert!((m.rms - 10f64.powf(target_rms_db / 20.0)).abs() < 0.00001);
        }
    }

    const CHUNK_LEN: usize = 44100;
    const CHUNK_MID: usize = CHUNK_LEN / 2;

    fn make_hann_chunk(size: usize) -> Vec<f64> {
        let window_fn = WindowFunction::Hann.get_fn();
        let mut chunk = vec![0.0f64; size];
        for n in 0..size {
            chunk[n] = window_fn(n, size);
        }
        chunk
    }

    #[test]
    /// Test OLAP for chunks with Hann window function.
    fn test_hamming_window_chunks_overlap() {
        let chunk = make_hann_chunk(CHUNK_LEN);
        let overlap = overlap_chunks(&chunk[CHUNK_MID..], &chunk[..CHUNK_MID]);
        for n in overlap {
            assert!(
                n >= 0.999999999999999f64,
                "Value {} is too small, must be >= 0.999999999999999",
                n
            );
        }
    }

    #[test]
    fn test_hamming_window_many_chunks_overlap() {
        let (chunk0_0, chunk0_1) = split_chunk(make_hann_chunk(CHUNK_LEN));
        assert_eq!(chunk0_0.len(), CHUNK_MID);
        assert_eq!(chunk0_1.len(), CHUNK_MID);

        let mut chunk_n_1: Vec<f64> = chunk0_1; // second half of chunk N
        let mut chunk_m_0: Vec<f64>; // first half of chunk N+1
        for _ in 0..2 {
            let (c_0, c_1) = split_chunk(make_hann_chunk(CHUNK_LEN));
            chunk_m_0 = c_0;
            let buffer: Vec<f64> = overlap_chunks(&chunk_n_1, &chunk_m_0);
            for n in buffer {
                assert!(
                    n >= 0.999999999999999f64,
                    "Value {} is too small, must be >= 0.999999999999999",
                    n
                );
            }
            chunk_n_1 = c_1;
        }
    }
}
