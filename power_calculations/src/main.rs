use bpaf::Bpaf;
use csv::{Reader, StringRecord};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use serde::Deserialize;
use std::{
    fmt::Write,
    fs::{File, metadata},
    path::PathBuf,
};

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options, version)]
struct Args {
    /// Measurement location
    #[bpaf(short, long)]
    measurement_location: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct JetsonMeasurement {
    /// Unit in microseconds
    measurement_timestamp: u128,
    /// Unit in milliamps
    current: u32,
    /// Unit in millivolts
    voltage: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ShellyPlug {
    /// Unit in microseconds
    measurement_timestamp: u128,
    /// Unit in volts
    voltage: f64,
    /// Unit in amps
    current: f64,
    /// Unit in watts
    power: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct FirmwareMeasruement {
    #[allow(dead_code)]
    measurement_index: u16,
    /// Unit in amps
    current: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PicoMeasurement {
    /// Unit in 10ths of microseconds
    measurement_timestamp: u128,
    // Unit in volts
    voltage: f64,
    // Unit in amps
    current: f64,
}

fn get_file_len(path: PathBuf) -> u64 {
    let file_metadata = metadata(path).expect("Could not open File");
    file_metadata.len()
}

fn init_reader(filename: &str, root_path: PathBuf) -> std::io::Result<(u64, Reader<File>)> {
    let mut filepath = root_path;
    filepath.push(filename);
    let file_len = get_file_len(filepath.clone());
    let csv_reader = Reader::from_path(filepath)?;
    Ok((file_len, csv_reader))
}

fn calc_energy(
    file_len: u64,
    mut file_reader: Reader<File>,
    update_interval: u32,
    entry_handler: impl Fn(Option<(f64, u128)>, StringRecord) -> std::io::Result<(f64, u128, f64)>,
) -> std::io::Result<f64> {
    let pb_style = ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-");
    let pb = ProgressBar::new(file_len);
    pb.set_style(pb_style);

    let mut last_power_time_opt: Option<(f64, u128)> = None;
    let mut total_energy = 0.0;
    let mut last_pb_update = 0;
    let mut start = true;
    for read_res in file_reader.records() {
        let str_record = read_res?;
        if last_pb_update == update_interval {
            pb.set_position(str_record.position().unwrap().byte());
            last_pb_update = 0;
        }
        let (power, time, energy) = entry_handler(last_power_time_opt, str_record)?;
        last_power_time_opt = Some((power, time));
        if power <= 5.0 && start {
            start = false;
            continue;
        }
        total_energy += energy;
        last_pb_update += 1;
    }
    Ok(total_energy)
}

fn estimate_voltage_from_current(current: f64) -> f64 {
    const VOLTAGE_VALUES: [f64; 36] = [
        18.96, 18.89, 18.85, 18.81, 18.77, 18.73, 18.7, 18.66, 18.63, 18.59, 18.56, 18.52, 18.49,
        18.46, 18.42, 18.39, 18.35, 18.32, 18.29, 18.25, 18.22, 18.19, 18.16, 18.13, 18.1, 18.07,
        18.04, 18.01, 17.97, 17.94, 17.91, 17.88, 17.84, 17.81, 17.79, 17.73,
    ];
    let range_index = ((current / 100.0).floor() as usize).min(33); // limiting to max
    // current of 3.5 A
    let range_percentage = 1.0 - ((current - range_index as f64) / 100.0);
    let lower_voltage_val = VOLTAGE_VALUES[range_index];
    let current_voltage_diff = (lower_voltage_val - VOLTAGE_VALUES[range_index + 1]).abs();
    lower_voltage_val + current_voltage_diff * range_percentage
}

fn main() -> std::io::Result<()> {
    let args = args().run();

    let (jetson_len, jetson_reader) = init_reader("jetson.csv", args.measurement_location.clone())?;
    let (shelly_len, shelly_reader) =
        init_reader("shellyPlug.csv", args.measurement_location.clone())?;
    let (shelly_len_2, shelly_reader_2) =
        init_reader("shellyPlug.csv", args.measurement_location.clone())?;
    let (firmware_len, firmware_reader) =
        init_reader("fast_firmware.csv", args.measurement_location.clone())?;
    let (pico_len, pico_reader) = init_reader(
        "usb_osc_data_normalized_time.csv",
        args.measurement_location.clone(),
    )?;

    let jetson_energy = calc_energy(jetson_len, jetson_reader, 1, |last_entry, str_record| {
        let jetson_measurement: JetsonMeasurement = str_record.deserialize(None)?;
        let current_power = (jetson_measurement.current as f64 / 1000.0)
            * (jetson_measurement.voltage as f64 / 1000.0); // need to convert to Amp and Volt to get Watt
        let current_energy;
        if let Some((last_power, last_time)) = last_entry {
            let time_diff =
                (jetson_measurement.measurement_timestamp - last_time) as f64 / 1_000_000.0; // converting us to s
            current_energy = time_diff * ((current_power + last_power) / 2.0);
        } else {
            current_energy = 0.0;
        }
        Ok((
            current_power,
            jetson_measurement.measurement_timestamp,
            current_energy,
        ))
    })?;

    println!("Jetson Energy: {jetson_energy:.2} Joule");

    let shelly_energy = calc_energy(shelly_len, shelly_reader, 1, |last_entry, str_record| {
        let shelly_measurement: ShellyPlug = str_record.deserialize(None)?;
        let current_power = shelly_measurement.voltage * shelly_measurement.current;
        /*if current_power != shelly_measurement.power {
            println!(
                "Shelly Plug calculation differentiates from current calculation: {}\t{}",
                current_power, shelly_measurement.power
            );
        }*/
        let current_energy;
        if let Some((last_power, last_time)) = last_entry {
            let time_diff =
                (shelly_measurement.measurement_timestamp - last_time) as f64 / 1_000_000.0;
            current_energy = time_diff * ((current_power + last_power) / 2.0);
        } else {
            current_energy = 0.0;
        }
        Ok((
            current_power,
            shelly_measurement.measurement_timestamp,
            current_energy,
        ))
    })?;

    println!("Shelly Energy (Calculated with Current and Voltage): {shelly_energy:.2} Joule");

    let shelly_energy_2 = calc_energy(
        shelly_len_2,
        shelly_reader_2,
        1,
        |last_entry, str_record| {
            let shelly_measurement: ShellyPlug = str_record.deserialize(None)?;
            let current_power = shelly_measurement.power;
            let current_energy;
            if let Some((last_power, last_time)) = last_entry {
                let time_diff =
                    (shelly_measurement.measurement_timestamp - last_time) as f64 / 1_000_000.0;
                current_energy = time_diff * ((current_power + last_power) / 2.0);
            } else {
                current_energy = 0.0;
            }
            Ok((
                current_power,
                shelly_measurement.measurement_timestamp,
                current_energy,
            ))
        },
    )?;

    println!(
        "Shelly Energy (Calculated with internal Power calculation): {shelly_energy_2:.2} Joule"
    );

    let firmware_energy = calc_energy(
        firmware_len,
        firmware_reader,
        1,
        |last_entry, str_record| {
            let firmware_measurement: FirmwareMeasruement = str_record.deserialize(None)?;
            // apply calibration
            let current_current = ((firmware_measurement.current as f64) * 0.9071) - 161.6;
            let current_power = current_current * estimate_voltage_from_current(current_current);
            let current_energy;
            if let Some((last_power, _)) = last_entry {
                let time_diff = 1.0 / 2000.0; // firmware has fixed samplerate
                current_energy = time_diff * ((current_power + last_power) / 2.0);
            } else {
                current_energy = 0.0;
            }
            Ok((current_power, 0, current_energy))
        },
    )?;

    println!(
        "Firmware Energy (Estimated voltage from calculated curve): {firmware_energy:.2} Joule"
    );

    let osc_energy = calc_energy(pico_len, pico_reader, 100_000, |last_entry, str_record| {
        let pico_measurement: PicoMeasurement = str_record.deserialize(None)?;
        let current_power = pico_measurement.voltage * pico_measurement.current;
        let current_energy;
        if let Some((last_power, last_time)) = last_entry {
            let time_diff =
                (pico_measurement.measurement_timestamp - last_time) as f64 / 10_000_000.0;
            current_energy = time_diff * ((current_power + last_power) / 2.0);
        } else {
            current_energy = 0.0;
        }
        Ok((
            current_power,
            pico_measurement.measurement_timestamp,
            current_energy,
        ))
    })?;

    println!("Osc Energy: {osc_energy:.2} Joule");

    Ok(())
}
