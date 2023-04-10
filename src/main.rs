use clap::Parser;
use rand::Rng;
use storir_rs::ImpulseResponse;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    folder: String,
}

fn create_wav_file(
    data: Vec<f32>,
    sample_rate: u32,
    file_name: &str,
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

    println!("Saving impulses to {}!", args.folder);

    let rt60: f32 = 500.0;
    let mut rng = rand::thread_rng();
    let drr = (rt60 * (-1.0 / 100.0)) + rng.gen_range(0.0..rt60 * (1.0 / 100.0));

    let rir = ImpulseResponse::new(rt60, 50.0, 3.0, 80.0, drr);
    for index in 1..=5 {
        let output = rir.generate(44100);
        match create_wav_file(output, 44100, format!("tmp/{}.wav", index).as_str())
        {
            Ok(()) => println!("WAV file created successfully."),
            Err(e) => eprintln!("Error: {}", e),
        }

        // println!("{:?}", output);
    }
}
