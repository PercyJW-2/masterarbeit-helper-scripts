use crate::data_reading_types::{PowerSample, PowerVec, WindowEnergyIter};
use biquad::{Biquad, Coefficients, DirectForm1, Q_BUTTERWORTH_F64, ToHertz};
use matplotlib::pyplot as plt;
use npyz::WriterBuilder;
use std::{
    fs::File,
    io::{BufWriter, Result},
};
use std::path::PathBuf;
use log::info;
use parquet::record::Row;
use crate::args::Args;
use crate::data_reading::{init_reader, read_to_power_vector};
use crate::output_types::Results;

pub(crate) fn calculate_results(
    args: &Args,
    file_name: &'static str,
    entry_handler: impl Fn(Row) -> Result<PowerSample>,
    do_filter: bool,
    trigger_factor: f64,
    pred_max_min: Option<(f64, f64)>,
    frame_size: f64,
    sample_rate: Option<f64>,
    output_file_name: &'static str,
) -> Result<Results> {
    let (file_len, file_reader) =
        init_reader(file_name, args.measurement_location.clone())?;
    let mut power = read_to_power_vector(file_len, file_reader, entry_handler)?;
    if do_filter {
        power = filter_data(power, sample_rate.unwrap(), None);
    }
    let (max, min);
    let mut idx = None;
    if args.dont_cut {
        let (start_idx, end_idx);
        (start_idx, end_idx, max, min) = find_data_start_and_end(
            &power,
            trigger_factor,
            pred_max_min,
            frame_size,
            sample_rate,
            args.plot_intermediates,
            args.estimated_duration,
        );
        idx = Some((start_idx, end_idx));
    } else {
        (power, min, max) = cut_data_start_and_end(
            power,
            trigger_factor,
            pred_max_min,
            frame_size,
            sample_rate,
            args.plot_intermediates,
            args.estimated_duration,
        );
    }
    save_vec_to_npy(&power, args.output_path.clone(), output_file_name)?;
    let energy = calc_energy(&power, sample_rate, idx);
    let duration = power.duration(idx, sample_rate);
    Ok(Results {
        energy,
        duration,
        start_stop_idx: idx,
        max_frame_energy: min,
        idle_frame_energy: max,
    })
}

pub(crate) fn filter_data(
    mut data: PowerVec,
    samplerate: f64,
    cutoff_freq: Option<f64>,
) -> PowerVec {
    let data_unwrapped_vec = match &mut data {
        PowerVec::Constant(unwrapped_sample) => unwrapped_sample,
        PowerVec::Variable(_) => unreachable!(), // data spacing needs to be constant
    };
    let data_unwrapped = data_unwrapped_vec.iter_mut();
    let coeffs = Coefficients::<f64>::from_params(
        biquad::Type::LowPass,
        samplerate.hz(),
        cutoff_freq.unwrap_or(samplerate * 0.25).hz(),
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

enum Side {
    Start,
    End,
}

impl Side {
    fn cut_calculation_power(
        data: impl Iterator<Item = (usize, f64)>,
        trigger_value: f64,
        plot: bool,
    ) -> usize {
        let mut energy_window_samples = vec![];
        let mut stop_idx = None;
        let mut window_idx = None;
        for (idx, (data_idx, win_energy)) in data.enumerate() {
            if win_energy > trigger_value && stop_idx.is_none() {
                stop_idx = Some(data_idx);
                window_idx = Some(idx);
                if !plot {
                    return data_idx;
                }
            }
            if plot {
                energy_window_samples.push(win_energy);
            }
        }
        if plot {
            let (_, [[mut ax]]) = plt::subplots().expect("could not initialize");
            ax.y(&energy_window_samples).plot();
            ax.xy(
                &[
                    window_idx.unwrap_or(0) as f64,
                    window_idx.unwrap_or(0) as f64,
                ],
                &[0.0, trigger_value],
            )
            .plot();
            plt::show();
        }
        info!("Index of Trigger-Point: {}", stop_idx.unwrap_or(0));
        stop_idx.unwrap_or(0)
    }

    fn iterations_until_trigger(
        &self,
        data: WindowEnergyIter,
        trigger_value: f64,
        plot: bool,
    ) -> usize {
        match self {
            Self::Start => Self::cut_calculation_power(data, trigger_value, plot),
            Self::End => Self::cut_calculation_power(data.rev(), trigger_value, plot),
        }
    }
}

pub(crate) fn cut_data_start_and_end(
    mut data: PowerVec,
    trigger_factor: f64,
    pred_max_min: Option<(f64, f64)>,
    window_size: f64,
    samplerate_opt: Option<f64>,
    plot: bool,
    estimated_duration_opt: Option<f64>
) -> (PowerVec, f64, f64) {
    let (start_idx, stop_idx, max, idle_value) = find_data_start_and_end(
        &data,
        trigger_factor,
        pred_max_min,
        window_size,
        samplerate_opt,
        plot,
        estimated_duration_opt,
    );
    data = data.cut_data(start_idx, stop_idx);
    (data, max, idle_value)
}

/// Finds start and end of measurement, either self-determines idle and max values or uses the
/// provided values.
/// Returns Tuple: (start_idx, end_idx, max_value, idle_value)
pub(crate) fn find_data_start_and_end(
    data: &PowerVec,
    trigger_factor: f64,
    pred_max_min: Option<(f64, f64)>,
    window_size: f64,
    samplerate_opt: Option<f64>,
    plot: bool,
    estimated_duration_opt: Option<f64>,
) -> (usize, usize, f64, f64) {
    let (max, idle_value) = if let Some((p_max, p_min)) = pred_max_min {
        (p_max, p_min)
    } else {
        info!("Searching maximum and idle values");
        data.power_window_iter(window_size, samplerate_opt)
            .max_and_idle()
    };
    let trigger_value = (max - idle_value) * trigger_factor + idle_value;
    info!("Trigger value: {trigger_value}");
    info!("Searching on start");
    let start_idx = Side::Start.iterations_until_trigger(
        data.power_window_iter(window_size, samplerate_opt),
        trigger_value,
        plot,
    );
    info!("Searching on end");
    let end_idx = data.len()
        - 1
        - Side::End.iterations_until_trigger(
            data.power_window_iter(window_size, samplerate_opt),
            trigger_value,
            plot,
        );
    if let Some(estimated_duration) = estimated_duration_opt {
        let (start, stop) = data.fit_start_stop_to_duration(start_idx, end_idx, estimated_duration, samplerate_opt);
        (start, stop, max, idle_value)
    } else {
        (start_idx, end_idx, max, idle_value)
    }
}

pub(crate) fn calc_energy(data: &PowerVec, samplerate_opt: Option<f64>, start_end_idx: Option<(usize, usize)>) -> f64 {
    let mut data_iter = data.iter(start_end_idx);
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

/// current -> unit is in mA
pub(crate) fn estimate_voltage_from_current(current: f64) -> f64 {
    let curve_pos = current / 100.;
    curve_pos * (-0.007444582) + 19.062607082705
    /*const VOLTAGE_VALUES: [f64; 36] = [
        18.9060, 18.8433, 18.8036, 18.7648, 18.7275, 18.6921, 18.6550, 18.6207, 18.5848, 18.5501,
        18.5169, 18.4835, 18.4498, 18.4162, 18.3825, 18.3494, 18.3166, 18.2837, 18.2672, 18.2335,
        18.2006, 18.1690, 18.1356, 18.1021, 18.0692, 18.0360, 18.0048, 17.9707, 17.9370, 17.9056,
        17.8714, 17.8374, 17.8041, 17.7691, 17.7327, 17.6952,
    ];
    let range_index = ((current / 100.0).floor() as usize).min(34); // limiting to max
    // current of 3.5 A
    let range_percentage = 1.0 - ((current - range_index as f64) / 100.0);
    let lower_voltage_val = VOLTAGE_VALUES[range_index];
    let current_voltage_diff = (lower_voltage_val - VOLTAGE_VALUES[range_index + 1]).abs();
    lower_voltage_val + current_voltage_diff * range_percentage*/
}

pub(crate) fn save_vec_to_npy(data: &PowerVec, location: PathBuf, filename: &'static str) -> Result<()> {
    let buf_wtr = BufWriter::new(File::create(location.join(filename))?);
    match data {
        PowerVec::Constant(constant_data) => {
            let mut npy_wtr = npyz::WriteOptions::new()
                .default_dtype()
                .shape(&[constant_data.len() as u64])
                .writer(buf_wtr)
                .begin_nd()?;
            npy_wtr.extend(constant_data.iter().copied())?;
            npy_wtr.finish()?;
        }
        PowerVec::Variable(variable_data) => {
            let mut npy_wtr = npyz::WriteOptions::new()
                .default_dtype()
                .shape(&[variable_data.len() as u64, 2])
                .writer(buf_wtr)
                .begin_nd()?;
            npy_wtr.extend(variable_data.iter().flat_map(|(tstmp, pow)| [*tstmp, *pow]))?;
            npy_wtr.finish()?;
        }
    }
    Ok(())
}
