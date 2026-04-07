use anyhow::{bail, Result};
use clap::{Parser, ValueEnum};
use log::{error, info, warn};
use std::path::PathBuf;
use std::process::ExitCode;

use noise_generator::noise::ifft_olap::{IFFTOverlapWithWindow, WindowFunction};
use noise_generator::wav::write_noise_to_wav_file;
use noise_generator::{NoiseColor, NormalizationDBFS};
use noise_generator::noise::ifft_phc::IFFTPhaseContinuation;
use noise_generator::noise::NoiseGenerator;

#[derive(ValueEnum, Clone)]
pub enum Generator {
    PHC,
    OLAP,
}

#[derive(ValueEnum, Clone)]
pub enum Color {
    White,
    Pink,
    Red,
    Blue,
    Violet,
    Grey,
}

#[allow(clippy::doc_markdown)]
#[derive(Parser)]
#[command(author, version, name = "noise-generator")]
struct Args {
    /// Noise color
    #[arg(value_enum)]
    color: Option<Color>,

    /// Noise generator [default: olap]
    #[arg(short, long, value_enum)]
    generator: Option<Generator>,

    /// Audio file to write
    #[arg(short, long, value_name = "WAV_FILE", default_value = "noise.wav")]
    output: PathBuf,

    /// Output duration in seconds [default: 10]
    #[arg(short, long)]
    seconds: Option<usize>,

    /// Sample rate [default: 44100]
    #[arg(short, long)]
    rate: Option<usize>,

    /// Normalize to RMS value in dBFS [default: -14.0]
    #[arg(long)]
    rms: Option<f64>,

    /// Normalize to peak value in dBFS (e.g.: -1.0)
    #[arg(long)]
    peak: Option<f64>,
    /// Overwrite if file already exists
    #[arg(short, long)]
    force: bool,
}

fn main() -> ExitCode {
    env_logger::init_from_env(
        env_logger::Env::default()
            .filter_or("LOG_LEVEL", "info")
            .write_style_or("LOG_STYLE", "always"),
    );
    if let Err(err) = run(&Args::parse()) {
        error!("{err}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run(args: &Args) -> Result<()> {
    let filename = &args.output;
    if filename.exists() && !args.force {
        bail!("File '{}' already exists", filename.display());
    }

    let seconds = args.seconds.unwrap_or(10);
    if seconds == 0 {
        bail!("File length must be >0 sec");
    }

    let sample_rate = {
        let mut rate = args.rate.unwrap_or(44100);
        if rate % 2 == 1 {
            rate += 1;
            warn!("Sample rate set to {rate}");
        }
        rate
    };

    let color = if let Some(color) = &args.color {
        match color {
            Color::White => NoiseColor::White,
            Color::Pink => NoiseColor::Pink,
            Color::Red => NoiseColor::Brownian,
            Color::Blue => NoiseColor::Blue,
            Color::Violet => NoiseColor::Violet,
            Color::Grey => NoiseColor::Grey,
        }
    } else {
        NoiseColor::White
    };

    let normalization = {
        if let Some(db) = args.rms {
            NormalizationDBFS::RMS(db)
        } else if let Some(db) = args.peak {
            NormalizationDBFS::Peak(db)
        } else {
            NormalizationDBFS::default()
        }
    };

    // A sine window is preferred here: in this stochastic overlap-add setup it produces lower
    // envelope modulation than Hann.
    let window = WindowFunction::Sine;

    let generator: Box<dyn NoiseGenerator> = match &args.generator {
        None | Some(Generator::OLAP) => {
            Box::new(IFFTOverlapWithWindow {
                color,
                normalization,
                window,
            })
        }
        Some(Generator::PHC) => {
            Box::new(IFFTPhaseContinuation {
                color,
                normalization,
            })
        }
    };

    info!("Noise generator: {}", generator.name());
    info!("Noise color: {color}");
    info!("Sample rate: {sample_rate}");
    info!("Seconds to generate: {seconds}");
    info!("Normalization: {normalization}");
    info!("Writing file: {}", filename.display());

    let info = write_noise_to_wav_file(generator.as_ref(), sample_rate, seconds, filename)?;
    info.print();
    Ok(())
}
