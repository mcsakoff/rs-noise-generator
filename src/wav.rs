use anyhow::Result;
use hound::{SampleFormat, WavSpec, WavWriter};
use log::{debug, info};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use crate::noise::NoizeGenerator;
use crate::utils::amplitude_to_db;

/// Store and display RMS and Peak values.
#[derive(Default, Debug)]
pub struct WavMeters {
    pub rms: f64,
    pub peak: f64,
}

impl WavMeters {
    pub fn update_max(&mut self, rms: f64, peak: f64) {
        self.rms = self.rms.max(rms);
        self.peak = self.peak.max(peak);
    }

    #[must_use]
    pub fn rms_db(&self) -> f64 {
        amplitude_to_db(self.rms)
    }

    #[must_use]
    pub fn peak_db(&self) -> f64 {
        amplitude_to_db(self.peak)
    }

    pub fn print(&self) {
        let peak_db = self.peak_db();
        if peak_db.is_finite() {
            info!("Peak: {peak_db:.02} dBFS");
        } else {
            info!("Peak: -inf dBFS");
        }

        let rms_db = self.rms_db();
        if rms_db.is_finite() {
            info!("RMS: {rms_db:.02} dBFS");
        } else {
            info!("RMS: -inf dBFS");
        }
    }
}

/// Write a WAV file with noise from generator.
///
/// # Errors
/// Can fail on noise generation or writing WAV file.
///
pub fn write_noise_to_wav_file(
    generator: &impl NoizeGenerator,
    sample_rate: usize,
    seconds: usize,
    filename: &Path,
) -> Result<WavMeters> {
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
    let meters = generator.generate(&mut writer, seconds)?;
    writer.finalize()?;
    Ok(meters)
}

/// Write samples to a WAV file.
///
/// # Errors
/// Can fail on writing to file.
///
pub(crate) fn write_samples(
    writer: &mut WavWriter<BufWriter<File>>,
    samples: &[f64],
    meters: Option<&mut WavMeters>,
) -> Result<()> {
    let mut sqr_sum: f64 = 0.0;
    let mut peak: f64 = 0.0;

    // Write samples and calculate RMS and Peak if required.
    for sample in samples {
        if meters.is_some() {
            sqr_sum += sample.powi(2);
            peak = peak.max(sample.abs());
        }
        #[allow(clippy::cast_possible_truncation)]
        writer.write_sample((sample * f64::from(i16::MAX)) as i16)?;
    }

    // Update RMS and Peak meters.
    if let Some(meters) = meters {
        let rms = (sqr_sum / samples.len() as f64).sqrt();
        meters.update_max(rms, peak);
    }
    Ok(())
}
