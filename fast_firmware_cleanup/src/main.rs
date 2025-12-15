use bpaf::Bpaf;
use csv::StringRecord;
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
    measurement_time: u16,
    current: u16,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct RetimedEntry {
    measurement_time: i64,
    current: u16,
}

fn get_reading_from_record(rec: StringRecord) -> RawReading {
    RawReading {
        measurement_time: rec.get(0).unwrap().parse::<u16>().unwrap(),
        current: rec.get(1).unwrap().parse::<u16>().unwrap(),
    }
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

    let pb = ProgressBar::new(input_file_len);
    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-")
    );

    let mut csv_reader_iter = csv_reader.records();
    let first = get_reading_from_record(csv_reader_iter.next().unwrap()?);
    let mut last_measurement_time = first.measurement_time;
    let mut total_time = 0;

    csv_writer.serialize(RetimedEntry {
        measurement_time: total_time,
        current: first.current,
    });

    let mut large_time_diff_count = 0;
    let mut item_count = 0;
    let mut last_large_time_diff = 0;
    let mut time_gap_diffs = vec![];

    for reading_res in csv_reader_iter {
        let str_record = reading_res?;
        pb.set_position(str_record.position().unwrap().byte());
        let reading = get_reading_from_record(str_record);

        let time_diff;
        if last_measurement_time > reading.measurement_time {
            time_diff = ((u16::MAX - last_measurement_time) + reading.measurement_time) as i64
        } else {
            time_diff = (reading.measurement_time - last_measurement_time) as i64
        }
        if time_diff > 20 {
            large_time_diff_count += 1;
            time_gap_diffs.push((total_time + time_diff) - last_large_time_diff);
            last_large_time_diff = total_time + time_diff;
        }
        if time_diff > 55 {
            println!(
                "Time diff is large {time_diff}, {}, {}",
                last_measurement_time, reading.measurement_time
            );
        } else if time_diff < 0 {
            println!(
                "Time diff is negative wtf {time_diff}, {}, {}",
                last_measurement_time, reading.measurement_time
            );
        }
        last_measurement_time = reading.measurement_time;
        total_time += time_diff;
        csv_writer.serialize(RetimedEntry {
            measurement_time: total_time,
            current: reading.current,
        });
        item_count += 1;
    }

    let percentage = large_time_diff_count as f64 / item_count as f64;

    println!("Large time_diffs: {large_time_diff_count}/{item_count} {percentage}%");

    let minimal_large_time_diff = time_gap_diffs.iter().min().unwrap();
    let maximal_large_time_diff = time_gap_diffs.iter().max().unwrap();
    let avg_large_time_diff =
        time_gap_diffs.iter().sum::<i64>() as f64 / time_gap_diffs.len() as f64;
    let std_large_time_diff = (time_gap_diffs
        .iter()
        .map(|value| {
            let diff = avg_large_time_diff - *value as f64;
            diff * diff
        })
        .sum::<f64>()
        / time_gap_diffs.len() as f64)
        .sqrt();

    println!(
        "Time difference between large time diffs:\n Max: {maximal_large_time_diff}us Min: {minimal_large_time_diff}us Avg: {avg_large_time_diff}us Std: {std_large_time_diff}"
    );
    Ok(())
}
