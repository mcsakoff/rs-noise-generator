#![allow(clippy::cast_precision_loss)]

use anyhow::Result;
use rand::RngExt;
use rustfft::num_complex::Complex64;
use std::f64::consts::TAU;
use std::fmt::{Display, Formatter};
use std::path::Path;

pub mod wav;

#[derive(Copy, Clone)]
pub enum NoiseColor {
    White,
    Pink,
    Brownian,
    Blue,
    Violet,
    Grey,
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

const FREQUENCIES_THRESHOLD: usize = 20;

/// Generate a WAV file with noise of specified color.
///
/// The frequencies below `FREQUENCIES_THRESHOLD` Hz are ignored.
///
/// # Errors
/// Can fail on writing WAV file.
///
pub fn generate_noise(
    color: NoiseColor,
    sample_rate: usize,
    seconds: usize,
    filename: &Path,
) -> Result<wav::WavInfo> {
    let frequencies_num = sample_rate / 2 + 1;
    let mut rng = rand::rng();

    let freq_to_amplitude_fn: fn(usize) -> f64 = match color {
        NoiseColor::White => |_| 1.0,
        NoiseColor::Pink => |frq| 1.0 / (frq as f64).sqrt(),
        NoiseColor::Brownian => |frq| 1.0 / (frq as f64),
        NoiseColor::Blue => |frq| (frq as f64).sqrt(),
        NoiseColor::Violet => |frq| frq as f64,
        NoiseColor::Grey => |frq| {
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
    };

    let spectrum = {
        let mut spectrum: Vec<Complex64> = Vec::with_capacity(frequencies_num);
        spectrum.resize(frequencies_num, Complex64::ZERO);
        for (frq, bin) in spectrum[..(frequencies_num - 1)]
            .iter_mut()
            .enumerate()
            .skip(FREQUENCIES_THRESHOLD)
        {
            *bin = Complex64::from_polar(freq_to_amplitude_fn(frq), rng.random::<f64>() * TAU);
        }
        spectrum
    };

    wav::write_with_spectrum(&spectrum, sample_rate, seconds, filename)
}
