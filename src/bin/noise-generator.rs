use anyhow::{bail, Result};
use clap::{Parser, ValueEnum};
use log::{error, info, warn};
use std::path::PathBuf;
use std::process::ExitCode;

use noise_generator::{generate_noise, NoiseColor};

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

    /// Output file
    #[arg(short, long, value_name = "WAV_FILE", default_value = "noise.wav")]
    output: PathBuf,

    /// Audio file length in seconds [default: 10]
    #[arg(short, long)]
    seconds: Option<usize>,

    /// Audio file sample rate [default: 44100]
    #[arg(short, long)]
    rate: Option<usize>,

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

    info!("Noise color: {color}");
    info!("Sample rate: {sample_rate}");
    info!("Seconds to generate: {seconds}");
    info!("Writing file: {}", filename.display());
    let info = generate_noise(color, sample_rate, seconds, filename)?;
    info.print();
    Ok(())
}
