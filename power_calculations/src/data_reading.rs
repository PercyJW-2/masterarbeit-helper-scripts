use crate::data_reading_types::*;
use std::{
    collections::VecDeque,
    fs::{File, metadata},
    path::PathBuf,
};

use indicatif::{ProgressBar, ProgressIterator, ProgressState, ProgressStyle};
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::record::{Field, Row};

pub(crate) fn get_file_len(path: PathBuf) -> u64 {
    let file_metadata = metadata(path.clone()).unwrap_or_else(|_| panic!("Could not open File {:?}", path));
    file_metadata.len()
}

pub(crate) fn init_reader(
    filename: &str,
    root_path: PathBuf,
) -> std::io::Result<(u64, SerializedFileReader<File>)> {
    let mut filepath = root_path;
    filepath.push(filename);
    let file_len = get_file_len(filepath.clone());
    let file = File::open(filepath)?;
    let reader = SerializedFileReader::new(file)?;
    Ok((file_len, reader))
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
    file_reader: SerializedFileReader<File>,
    entry_handler: impl Fn(Row) -> std::io::Result<PowerSample>,
) -> std::io::Result<PowerVec> {
    let pb_style = get_pb_style();
    let pb = ProgressBar::new(file_len);
    pb.set_style(pb_style);

    let mut values_const = VecDeque::new();
    let mut values_varia = VecDeque::new();

    for row in file_reader.get_row_iter(None)?.progress_with(pb) {
        let entry = entry_handler(row?)?;
        match entry {
            PowerSample::Constant(value) => values_const.push_back(value),
            PowerSample::Variable(tstmp, value) => values_varia.push_back((tstmp, value)),
        }
    }

    if values_varia.is_empty() {
        Ok(PowerVec::Constant(values_const.into()))
    } else {
        Ok(PowerVec::Variable(values_varia.into()))
    }
}

pub(crate) fn field_to_u64(field: &Field) -> Option<u64> {
    match field {
        Field::ULong(value) => Some(*value),
        _ => None,
    }
}

pub(crate) fn field_to_f64(field: &Field) -> Option<f64> {
    match field {
        Field::Double(value) => Some(*value),
        _ => None,
    }
}

pub(crate) fn field_to_u32(field: &Field) -> Option<u32> {
    match field {
        Field::UInt(value) => Some(*value),
        _ => None,
    }
}

pub(crate) fn field_to_f32(field: &Field) -> Option<f32> {
    match field {
        Field::Float(value) => Some(*value),
        _ => None,
    }
}

pub(crate) fn field_to_u16(field: &Field) -> Option<u16> {
    match field {
        Field::UShort(value) => Some(*value),
        _ => None,
    }
}
