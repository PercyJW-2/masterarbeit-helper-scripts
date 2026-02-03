use biquad::{Biquad, Coefficients, DirectForm1, Q_BUTTERWORTH_F64, ToHertz};
use bpaf::Bpaf;
use csv::{Reader, StringRecord};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use matplotlib as plt;
use serde::Deserialize;
use std::{
    collections::VecDeque,
    fs::{File, metadata},
    io::{self, Write},
    iter::Iterator,
    path::PathBuf,
};

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options, version)]
struct Args {
    /// Measurement location
    #[bpaf(short, long)]
    measurement_location: PathBuf,
    /// use osc-voltage measurement instead of voltage estimation
    #[bpaf(short, long)]
    osc_voltage: bool,
    /// value on which firmware measurement is triggered, if not provided complete dataset is used
    #[bpaf(short, long)]
    firmware_trigger_value: Option<f64>,
    /// value on which oscilloscope measurement is triggered, if not provided complete dataset is
    /// used
    #[bpaf(short, long)]
    osc_trigger_value: Option<f64>,
    /// averaging frame size - configures duration of frame size wich is used to detect the
    /// beginning of the dataset, the default value is 1/2000 (seconds)
    #[bpaf(short, long)]
    frame_size: Option<f64>,
    /// oscilloscope samplerate - default is 5_000_000
    #[bpaf(short, long)]
    osc_samplerate: Option<f64>,
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
#[allow(dead_code)]
struct PicoMeasurement {
    /// Unit in microseconds
    measurement_timestamp: u128,
    /// Sample Number in current data package
    sample_index: u32,
    /// Unit in volts
    voltage: f64,
    /// Unit in amps
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
    let csv_reader = Reader::from_path(filepath.clone())?;
    Ok((file_len, csv_reader))
}

fn get_pb_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| write!(w, "{}:{}", state.eta().as_secs() / 60, state.eta().as_secs() % 60).unwrap())
        .progress_chars("#>=")
}

type Power = f64;
type Timestamp = f64;

enum PowerSample {
    Constant(Power),
    Variable(Timestamp, Power),
}

/// Reads file and directly calculates power for current sample
fn read_to_power_vector(
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

fn filter_data(
    mut data: VecDeque<PowerSample>,
    samplerate: f64,
    cutoff_freq: Option<f64>,
) -> VecDeque<PowerSample> {
    let data_unwrapped = data.iter_mut().map(|sample| {
        match sample {
            PowerSample::Constant(unwrapped_sample) => unwrapped_sample,
            PowerSample::Variable(_, _) => unreachable!(), // data spacing needs to be constant
        }
    });
    let coeffs = Coefficients::<f64>::from_params(
        biquad::Type::LowPass,
        samplerate.hz(),
        cutoff_freq.unwrap_or(samplerate * 0.45).hz(),
        Q_BUTTERWORTH_F64,
    )
    .unwrap();
    let mut biquad = DirectForm1::<f64>::new(coeffs);

    for sample in data_unwrapped {
        let new_sample = biquad.run(*sample);
        *sample = new_sample;
    }

    data
}

fn cut_calculation<'a>(
    mut data: impl Iterator<Item = &'a PowerSample>,
    threshold: f64,
    msmt_frame_duration: f64,
    samplerate_opt: Option<f64>,
    plot: bool,
) -> u32 {
    let samplerate = samplerate_opt.unwrap_or(0.0);
    if let Some(PowerSample::Constant(_)) = data.next()
        && samplerate == 0.0
    {
        unreachable!()
    }
    let mut power_avg = 0.0;
    let mut current_time = 0.0;
    let mut start_index = 0;
    let mut current_sample_count = 0.;
    let mut last_timestamp = 0.;
    let mut avg_samples = vec![];
    let mut found_start = false;
    for sample in data {
        match sample {
            PowerSample::Constant(power) => {
                power_avg += power;
                current_time += samplerate;
            }
            PowerSample::Variable(t_stamp, power) => {
                if last_timestamp == 0. {
                    last_timestamp = *t_stamp;
                    continue;
                }
                power_avg += power;
                current_time += t_stamp - last_timestamp;
            }
        }
        current_sample_count += 1.;
        if current_time >= msmt_frame_duration {
            let power = power_avg / current_sample_count;
            if power >= threshold && !found_start {
                if plot {
                    found_start = true;
                } else {
                    break;
                }
            }
            avg_samples.push(power);
            power_avg = 0.;
            current_time = 0.;
            current_sample_count = 0.;
        }
        if !found_start {
            start_index += 1;
        }
    }
    if plot {
        let (_, [[mut ax]]) = plt::subplots().expect("Could not initiate matplotlib");
        ax.y(&avg_samples).plot();
        plt::show();
    }
    start_index
}

fn cut_start(
    mut data: VecDeque<PowerSample>,
    threshold: f64,
    msmt_frame_duration: f64,
    samplerate_opt: Option<f64>,
    plot: bool,
) -> VecDeque<PowerSample> {
    let cut_count = cut_calculation(
        data.iter(),
        threshold,
        msmt_frame_duration,
        samplerate_opt,
        plot,
    );
    println!("Removing {cut_count} samples");
    for _ in 0..cut_count {
        let _ = data.pop_front();
    }
    data
}

#[allow(dead_code)]
fn cut_end(
    mut data: VecDeque<PowerSample>,
    threshold: f64,
    msmt_frame_duration: f64,
    samplerate_opt: Option<f64>,
    plot: bool,
) -> VecDeque<PowerSample> {
    let cut_count = cut_calculation(
        data.iter().rev(),
        threshold,
        msmt_frame_duration,
        samplerate_opt,
        plot,
    );
    for _ in 0..cut_count {
        let _ = data.pop_back();
    }
    data
}

fn calc_energy(data: &VecDeque<PowerSample>, samplerate_opt: Option<f64>) -> f64 {
    let mut data_iter = data.iter();
    let first_elem = data_iter.next().unwrap();
    let samplerate = samplerate_opt.unwrap_or(0.0);
    if let PowerSample::Constant(_) = first_elem
        && samplerate == 0.0
    {
        unreachable!();
    }
    let (energy, _) = data_iter.fold(
        (0.0, first_elem),
        |(current_energy, last_sample), sample| {
            let next_energy = match sample {
                PowerSample::Constant(power) => {
                    let PowerSample::Constant(last_power) = last_sample else {
                        unreachable!()
                    };
                    current_energy + ((power + last_power) / 2.) * (1. / samplerate)
                }
                PowerSample::Variable(timestamp, power) => {
                    let PowerSample::Variable(last_timestamp, last_power) = last_sample else {
                        unreachable!()
                    };
                    current_energy + ((power + last_power) / 2.) * (timestamp - last_timestamp)
                }
            };
            (next_energy, sample)
        },
    );
    energy
}

fn estimate_voltage_from_current(current: f64) -> f64 {
    const VOLTAGE_VALUES: [f64; 36] = [
        18.96, 18.89, 18.85, 18.81, 18.77, 18.73, 18.7, 18.66, 18.63, 18.59, 18.56, 18.52, 18.49,
        18.46, 18.42, 18.39, 18.35, 18.32, 18.29, 18.25, 18.22, 18.19, 18.16, 18.13, 18.1, 18.07,
        18.04, 18.01, 17.97, 17.94, 17.91, 17.88, 17.84, 17.81, 17.79, 17.73,
    ];
    let range_index = ((current / 100.0).floor() as usize).min(34); // limiting to max
    // current of 3.5 A
    let range_percentage = 1.0 - ((current - range_index as f64) / 100.0);
    let lower_voltage_val = VOLTAGE_VALUES[range_index];
    let current_voltage_diff = (lower_voltage_val - VOLTAGE_VALUES[range_index + 1]).abs();
    lower_voltage_val + current_voltage_diff * range_percentage
}

fn main() -> std::io::Result<()> {
    let args = args().run();
    let avg_frame_duration = args.frame_size.unwrap_or(1. / 2_000.);

    let (jetson_len, jetson_reader) = init_reader("jetson.csv", args.measurement_location.clone())?;
    let (shelly_len, shelly_reader) =
        init_reader("shellyPlug.csv", args.measurement_location.clone())?;
    let (shelly_len_2, shelly_reader_2) =
        init_reader("shellyPlug.csv", args.measurement_location.clone())?;
    let (firmware_len, firmware_reader) =
        init_reader("fast_firmware.csv", args.measurement_location.clone())?;
    let (pico_len, pico_reader) =
        init_reader("usb_osc_data.csv", args.measurement_location.clone())?;

    let jetson_power = read_to_power_vector(jetson_len, jetson_reader, 1, |str_record| {
        let jetson_measurement: JetsonMeasurement = str_record.deserialize(None)?;
        let current_power = (jetson_measurement.current as f64 / 1000.)
            * (jetson_measurement.voltage as f64 / 1000.);
        Ok(PowerSample::Variable(
            jetson_measurement.measurement_timestamp as f64 / 1_000_000.,
            current_power,
        ))
    })?;
    let jetson_energy = calc_energy(&jetson_power, None);

    println!("Jetson Energy: {jetson_energy:.2} Joule");

    let shelly_power = read_to_power_vector(shelly_len, shelly_reader, 1, |str_record| {
        let shelly_measurement: ShellyPlug = str_record.deserialize(None)?;
        let current_power = shelly_measurement.voltage * shelly_measurement.current;
        Ok(PowerSample::Variable(
            shelly_measurement.measurement_timestamp as f64 / 1_000_000.,
            current_power,
        ))
    })?;
    let shelly_energy = calc_energy(&shelly_power, None);

    println!("Shelly Energy (Calculated with Current and Voltage): {shelly_energy:.2} Joule");

    let shelly_power_2 = read_to_power_vector(shelly_len_2, shelly_reader_2, 1, |str_record| {
        let shelly_measurement: ShellyPlug = str_record.deserialize(None)?;
        Ok(PowerSample::Variable(
            shelly_measurement.measurement_timestamp as f64 / 1_000_000.,
            shelly_measurement.power,
        ))
    })?;
    let shelly_energy_2 = calc_energy(&shelly_power_2, None);

    println!(
        "Shelly Energy (Calculated with internal Power calculation): {shelly_energy_2:.2} Joule"
    );

    let mut firmware_power =
        read_to_power_vector(firmware_len, firmware_reader, 1, |str_record| {
            let firmware_measurement: FirmwareMeasruement = str_record.deserialize(None)?;
            // apply calibration
            let mut current_current =
                ((firmware_measurement.current as f64) * 0.90710233) + 161.6623038;
            // apply second calibration
            current_current *= 0.99245570488;
            current_current -= 395.348969462;
            // apply third calibration (test)
            current_current *= 1.04;
            let current_power =
                (current_current / 1000.) * estimate_voltage_from_current(current_current);
            Ok(PowerSample::Constant(current_power))
        })?;

    firmware_power = if args.firmware_trigger_value.is_some() {
        cut_start(
            firmware_power,
            args.firmware_trigger_value.unwrap(),
            avg_frame_duration,
            Some(2000.),
            false,
        )
    } else {
        println!("Starting calibration assistant");
        let power = cut_start(firmware_power, 0.0, avg_frame_duration, Some(2000.), true);
        print!("Provide threshold: ");
        io::stdout().flush().expect("Could not flush stdout");
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;
        let threshold: f64 = buffer.trim().parse().expect("Could not parse float");
        cut_start(power, threshold, avg_frame_duration, Some(2000.), true)
    };

    let firmware_energy = calc_energy(&firmware_power, Some(2000.)); // placeholder
    println!(
        "Firmware Energy (Estimated voltage from calculated curve): {firmware_energy:.2} Joule"
    );

    let mut osc_power = read_to_power_vector(pico_len, pico_reader, 100_000, |str_record| {
        let pico_measurement: PicoMeasurement = str_record.deserialize(None)?;
        let current = pico_measurement.current * 0.9355;
        let voltage = if args.osc_voltage {
            pico_measurement.voltage
        } else {
            estimate_voltage_from_current(current * 1000.)
        };
        let current_power = voltage * current;
        Ok(PowerSample::Constant(current_power))
    })?;

    let osc_samplerate = args.osc_samplerate.unwrap_or(5_000_000.);
    osc_power = filter_data(osc_power, osc_samplerate, None);
    osc_power = if args.osc_trigger_value.is_some() {
        cut_start(
            osc_power,
            args.osc_trigger_value.unwrap(),
            avg_frame_duration,
            Some(osc_samplerate),
            false,
        )
    } else {
        println!("Starting calibration assistant");
        let power = cut_start(
            osc_power,
            0.0,
            avg_frame_duration,
            Some(osc_samplerate),
            true,
        );
        print!("Provide threshold: ");
        io::stdout().flush().expect("Could not flush stdout");
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;
        let threshold: f64 = buffer.trim().parse().expect("Could not parse float");
        cut_start(
            power,
            threshold,
            avg_frame_duration,
            Some(osc_samplerate),
            true,
        )
    };
    let osc_energy = calc_energy(&osc_power, Some(osc_samplerate));

    println!("Osc Energy: {osc_energy:.2} Joule");

    Ok(())
}
