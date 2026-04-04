use anyhow::Result;
use hound::{SampleFormat, WavSpec, WavWriter};
use log::{debug, info};
use realfft::RealFftPlanner;
use rustfft::num_complex::Complex64;
use std::f64::consts::TAU;
use std::path::Path;

pub struct WavInfo {
    pub rms: f64,
    pub peak: f64,
}

impl WavInfo {
    pub fn update_max(&mut self, rms: f64, peak: f64) {
        self.rms = self.rms.max(rms);
        self.peak = self.peak.max(peak);
    }

    pub fn print(&self) {
        let peak_db = value_to_db(self.peak);
        if peak_db.is_finite() {
            info!("Peak: {peak_db:.02} dBFS");
        } else {
            info!("Peak: -inf dBFS");
        }

        let rms_db = value_to_db(self.rms);
        if rms_db.is_finite() {
            info!("RMS: {rms_db:.02} dBFS");
        } else {
            info!("RMS: -inf dBFS");
        }
    }
}

/// Write a WAV file with provided spectrum.
///
/// The resulting signal is normalized to about -0.9 dBFS but only first second is used to
/// calculate normalization gain.
///
/// # Errors
/// Can fail on writing WAV file.
///
/// # Panics
/// Panics if `spectrum` vector's length is not `sample_rate` / 2 + 1.
///
pub fn write_with_spectrum(
    spectrum: &[Complex64],
    sample_rate: usize,
    seconds: usize,
    filename: &Path,
) -> Result<WavInfo> {
    assert_eq!(spectrum.len(), sample_rate / 2 + 1);

    let spec = WavSpec {
        channels: 1,
        #[allow(clippy::cast_possible_truncation)]
        sample_rate: sample_rate as u32,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    if filename.exists() {
        debug!("Overwriting existing file: {}", filename.display());
    }
    let mut writer = WavWriter::create(filename, spec)?;
    let mut results = WavInfo {
        rms: 0.0,
        peak: 0.0,
    };

    let mut planner = RealFftPlanner::<f64>::new();
    let ifft = planner.plan_fft_inverse(sample_rate); // 1 sec on current sample rate

    let mut spectrum_state = ifft.make_input_vec();
    let mut scratch = ifft.make_scratch_vec();
    let mut signal = ifft.make_output_vec();
    assert_eq!(spectrum_state.len(), sample_rate / 2 + 1);
    assert_eq!(signal.len(), sample_rate);

    // RustFFT does not normalize outputs. Callers must manually normalize the results.
    let mut normalization_gain: Option<f64> = None;
    let max_amplitude = f64::from(i16::MAX) * 0.9;

    // Generate signal second by second.
    for t_sec in 0..seconds {
        // Counters for RMS and Peak calculations.
        let mut sqr_sum: f64 = 0.0;
        let mut peak: f64 = 0.0;

        // The input buffer is used as scratch space, so the contents of input should be considered
        // garbage after calling ifft.process_with_scratch*(). Fill it with original spectrum data
        // but with phase corrected to the next second chunk.
        for (freq, (current, initial)) in spectrum_state.iter_mut().zip(spectrum).enumerate() {
            let (amplitude, phase) = initial.to_polar();
            *current = Complex64::from_polar(amplitude, phase + TAU * freq as f64 * t_sec as f64);
        }

        // Inverse transform a complex spectrum into signal.
        ifft.process_with_scratch(&mut spectrum_state, &mut signal, &mut scratch)?;

        // Get pre-calculated normalization gain or calculate one based on current signal interval.
        let gain = match normalization_gain {
            None => {
                let peak: f64 = signal.iter().fold(0.0, |acc, val| acc.max(val.abs()));
                let gain = 1.0 / peak;
                normalization_gain = Some(gain);
                gain
            }
            Some(gain) => gain,
        };

        // Normalize signal, write to file and calculate Peak and RMS.
        for sample in &signal {
            let sample = sample * gain; // normalize to [-1.0 .. 1.0]

            #[allow(clippy::cast_possible_truncation)]
            writer.write_sample((sample * max_amplitude) as i16)?;

            sqr_sum += sample.powi(2); // to calculate RMS later
            peak = peak.max(sample.abs()); // to calculate Peak
        }

        let rms = (sqr_sum / sample_rate as f64).sqrt();
        results.update_max(rms, peak);
    }
    writer.finalize()?;
    if let Some(gain) = normalization_gain {
        debug!("Normalization Gain: {gain}");
    }
    Ok(results)
}

/// Convert plain value to dBFS. Can return -inf.
fn value_to_db(value: f64) -> f64 {
    if value > 0.0 {
        20.0 * value.log10()
    } else {
        f64::NEG_INFINITY
    }
}
