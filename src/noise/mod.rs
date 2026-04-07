use anyhow::Result;
use hound::WavWriter;
use std::fs::File;
use std::io::BufWriter;

use crate::wav::WavMeters;

pub mod ifft_phc;
pub mod ifft_olap;

/// Frequencies below this threshold are ignored.
const LOW_FREQUENCIES_THRESHOLD: usize = 15;

pub trait NoiseGenerator {
    fn name(&self) -> &'static str;
    
    #[allow(clippy::missing_errors_doc)]
    fn generate(&self, writer: &mut WavWriter<BufWriter<File>>, seconds: usize) -> Result<WavMeters>;
}
