use crate::data_reading_types::*;
use csv::{Reader, StringRecord};
use std::{
    collections::VecDeque,
    fs::{File, metadata},
    path::PathBuf,
};

use indicatif::{ProgressBar, ProgressState, ProgressStyle};

pub(crate) fn get_file_len(path: PathBuf) -> u64 {
    let file_metadata = metadata(path).expect("Could not open File");
    file_metadata.len()
}

pub(crate) fn init_reader(
    filename: &str,
    root_path: PathBuf,
) -> std::io::Result<(u64, Reader<File>)> {
    let mut filepath = root_path;
    filepath.push(filename);
    let file_len = get_file_len(filepath.clone());
    let csv_reader = Reader::from_path(filepath.clone())?;
    Ok((file_len, csv_reader))
}

pub(crate) fn get_pb_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| write!(w, "{}:{}", state.eta().as_secs() / 60, state.eta().as_secs() % 60).unwrap())
        .progress_chars("#>=")
}

/// Reads file and directly calculates power for current sample
pub(crate) fn read_to_power_vector(
    file_len: u64,
    mut file_reader: Reader<File>,
    update_interval: u32,
    entry_handler: impl Fn(StringRecord) -> std::io::Result<PowerSample>,
) -> std::io::Result<VecDeque<PowerSample>> {
    let pb_style = get_pb_style();
    let pb = ProgressBar::new(file_len);
    pb.set_style(pb_style);

    let mut values = VecDeque::new();

    let mut last_pb_update = 0;

    for read_res in file_reader.records() {
        let str_record = read_res?;

        if last_pb_update == update_interval {
            pb.set_position(str_record.position().unwrap().byte());
            last_pb_update = 0;
        } else {
            last_pb_update += 1;
        }

        values.push_back(entry_handler(str_record)?);
    }

    Ok(values)
}
