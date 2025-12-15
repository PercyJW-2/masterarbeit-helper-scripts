use bpaf::Bpaf;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use serde::Serialize;
use std::{fmt::Write, fs::metadata, path::PathBuf};

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options, version)]
struct Args {
    /// Input file
    #[bpaf(short, long)]
    input_file: PathBuf,
    /// Output file
    #[bpaf(short, long)]
    output_file: PathBuf,
}

struct RawReading {
    measurement_timestamp: u128,
    voltage: f64,
    current: f64,
}

#[derive(Debug)]
struct ChunkEntry {
    voltage: f64,
    current: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct RetimedEntry {
    measurement_timestamp: u128,
    voltage: f64,
    current: f64,
}

fn main() -> std::io::Result<()> {
    let args = args().run();
    println!("{:?}", args);

    let input_file_len;
    {
        let file_metadata = metadata(args.input_file.clone())?;
        input_file_len = file_metadata.len();
    }

    let mut csv_reader = csv::Reader::from_path(args.input_file)?;
    let mut csv_writer = csv::Writer::from_path(args.output_file)?;

    let mut chunk_enties = vec![];
    let mut current_time = 0;

    let pb = ProgressBar::new(input_file_len);
    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-"));
    let mut pb_update_counter = 0;

    for reading_res in csv_reader.records() {
        let str_record = reading_res?;
        let reading = RawReading {
            measurement_timestamp: str_record.get(0).unwrap().parse::<u128>().unwrap(),
            voltage: str_record.get(2).unwrap().parse::<f64>().unwrap(),
            current: str_record.get(3).unwrap().parse::<f64>().unwrap(),
        };
        if current_time == 0 {
            current_time = reading.measurement_timestamp;
            chunk_enties.push(ChunkEntry {
                voltage: reading.voltage,
                current: reading.current,
            });
        } else if current_time != reading.measurement_timestamp {
            // there should be about 5 readings per us, thus we need to increase the scale
            let time_diff = (reading.measurement_timestamp - current_time) * 10;
            let time_per_sample = time_diff / (chunk_enties.len() as u128);
            for (idx, entry) in chunk_enties.iter().enumerate() {
                csv_writer.serialize(RetimedEntry {
                    measurement_timestamp: current_time * 10 + time_per_sample * (idx as u128),
                    voltage: entry.voltage,
                    current: entry.current,
                })?;
            }

            current_time = reading.measurement_timestamp;
            chunk_enties = vec![ChunkEntry {
                voltage: reading.voltage,
                current: reading.current,
            }]
        } else {
            chunk_enties.push(ChunkEntry {
                voltage: reading.voltage,
                current: reading.current,
            });
        }
        if pb_update_counter == 100_000 {
            pb.set_position(str_record.position().unwrap().byte());
            pb_update_counter = 0;
        }
        pb_update_counter += 1;
    }
    Ok(())
}
