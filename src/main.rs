use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use ndarray_rand::rand::Rng;
use storir::ImpulseResponse;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Sample rate
    #[arg(short, long, default_value = "44100")]
    sample_rate: u32,
    /// Folder to store wav files
    #[arg(short, long, default_value = "impulses")]
    folder: String,
    /// Number of impulses to generate
    #[arg(short, long, default_value = "5")]
    num_impulses: u32,
    /// Reverberation time in [ms]
    #[arg(long, default_value = "500")]
    rt60: u32,
    /// Early decay time [ms]
    #[arg(long, default_value = "50")]
    edt: u32,
    /// Initial time delay gap [ms]
    #[arg(long, default_value = "4")]
    itdg: u32,
    /// Early reflections duration [ms]
    #[arg(long, default_value = "100")]
    er_duration: u32,
}

fn create_wav_file<P: AsRef<Path>>(
    data: Vec<f32>,
    sample_rate: u32,
    file_name: P,
) -> Result<(), hound::Error> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let max_amplitude = i16::MAX as f32;
    let mut writer = hound::WavWriter::create(file_name, spec)?;
    for sample in data {
        let amplitude = (sample * max_amplitude).round() as i16;
        writer.write_sample(amplitude)?;
    }

    writer.finalize()
}

fn main() {
    let args = Args::parse();

    // Save to folder
    println!("Saving impulses to {}!", args.folder);
    if !Path::new(&args.folder).exists() {
        match fs::create_dir(&args.folder) {
            Ok(_) => println!(
                "No such folder found, crate new one '{}' ...",
                args.folder
            ),
            Err(err) => eprint!("Error creating folder {} : {}", args.folder, err),
        }
    } else {
        println!("'{}' folder already exists...", args.folder)
    };

    let mut rng = ndarray_rand::rand::thread_rng();
    let drr = (args.rt60 as f32 * (-1.0 / 100.0))
        + rng.gen_range(0.0..args.rt60 as f32 * (1.0 / 100.0));

    let rir = ImpulseResponse::new(
        args.rt60 as f32,
        args.edt as f32,
        args.itdg as f32,
        args.er_duration as f32,
        drr,
    );
    for index in 1..=args.num_impulses {
        // Platform independent filepath
        let mut path_buf = PathBuf::new();
        let file_name = format!(
            "rt60_{}_edt_{}_itdg_{}_erd_{}_i{}.wav",
            args.rt60, args.edt, args.itdg, args.er_duration, index
        );
        path_buf.push(args.folder.clone());
        path_buf.push(file_name);

        let impulse = rir.generate(args.sample_rate);
        match create_wav_file(impulse, args.sample_rate, &path_buf) {
            Ok(()) => {
                println!(
                    "WAV file '{}' created successfully.",
                    path_buf.as_path().to_str().unwrap()
                )
            }
            Err(e) => eprintln!("Error: {}", e),
        };
    }
}
