use crate::data_reading_types::PowerSample;
use biquad::{Biquad, Coefficients, DirectForm1, Q_BUTTERWORTH_F64, ToHertz};
use matplotlib as plt;
use npyz::WriterBuilder;
use std::{
    collections::VecDeque,
    fs::File,
    io::{BufWriter, Result, Write, stdin, stdout},
};

pub(crate) fn filter_data(
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
                current_time += 1. / samplerate;
            }
            PowerSample::Variable(t_stamp, power) => {
                if last_timestamp == 0. {
                    last_timestamp = *t_stamp;
                    continue;
                }
                power_avg += power;
                current_time += (t_stamp - last_timestamp).abs();
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
            if found_start {
                avg_samples.push(power);
            }
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
    for _ in 0..cut_count {
        let _ = data.pop_front();
    }
    data
}

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

enum Side {
    Start,
    End,
}

impl Side {
    fn to_str(&self) -> &'static str {
        match self {
            Self::Start => "Start",
            Self::End => "End",
        }
    }

    fn cut_on_side(
        &self,
        data: VecDeque<PowerSample>,
        threshold: f64,
        msmt_frame_duration: f64,
        samplerate_opt: Option<f64>,
        plot: bool,
    ) -> VecDeque<PowerSample> {
        match self {
            Self::Start => cut_start(data, threshold, msmt_frame_duration, samplerate_opt, plot),
            Self::End => cut_end(data, threshold, msmt_frame_duration, samplerate_opt, plot),
        }
    }
}

fn data_cut_calibration(
    data: VecDeque<PowerSample>,
    threshold_opt: Option<f64>,
    msmt_frame_duration: f64,
    samplerate_opt: Option<f64>,
    side: Side,
) -> VecDeque<PowerSample> {
    println!("Cutting on {}", side.to_str());
    if let Some(threshold) = threshold_opt {
        side.cut_on_side(data, threshold, msmt_frame_duration, samplerate_opt, false)
    } else {
        let power = side.cut_on_side(data, 0.0, msmt_frame_duration, samplerate_opt, true);
        print!("Provide threshold: ");
        stdout().flush().expect("Could not flush stdout");
        let mut buffer = String::new();
        stdin()
            .read_line(&mut buffer)
            .expect("Could not read from stdin");
        let threshold: f64 = buffer.trim().parse().expect("Could not parse float");
        side.cut_on_side(power, threshold, msmt_frame_duration, samplerate_opt, true)
    }
}

pub(crate) fn cut_data_start_and_end(
    mut data: VecDeque<PowerSample>,
    threshold_start: Option<f64>,
    threshold_end: Option<f64>,
    msmt_frame_duration: f64,
    samplerate_opt: Option<f64>,
    data_name: &'static str,
) -> VecDeque<PowerSample> {
    println!("Starting data cutting of {data_name}");
    data = data_cut_calibration(
        data,
        threshold_start,
        msmt_frame_duration,
        samplerate_opt,
        Side::Start,
    );
    data = data_cut_calibration(
        data,
        threshold_end,
        msmt_frame_duration,
        samplerate_opt,
        Side::End,
    );
    //cut_calculation(data.iter(), 0.0, msmt_frame_duration, samplerate_opt, true);
    data
}

pub(crate) fn calc_energy(data: &VecDeque<PowerSample>, samplerate_opt: Option<f64>) -> f64 {
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

pub(crate) fn estimate_voltage_from_current(current: f64) -> f64 {
    const VOLTAGE_VALUES: [f64; 36] = [
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
    lower_voltage_val + current_voltage_diff * range_percentage
}

pub(crate) fn save_vec_to_npy(data: &VecDeque<PowerSample>, filename: &'static str) -> Result<()> {
    let buf_wtr = BufWriter::new(File::create(filename)?);
    let mut npy_wtr = npyz::WriteOptions::new()
        .default_dtype()
        .shape(&[data.len() as u64])
        .writer(buf_wtr)
        .begin_nd()?;
    npy_wtr.extend(data.iter().map(|elem| {
        if let PowerSample::Constant(num) = elem {
            *num
        } else {
            unreachable!();
        }
    }))?;
    npy_wtr.finish()
}
