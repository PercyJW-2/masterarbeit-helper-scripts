mod args;
mod data_actions;
mod data_reading;
mod data_reading_types;
mod output_types;

use std::{fs, io};
use pyo3::prelude::*;

use crate::args::*;
use crate::data_actions::*;
use crate::data_reading::*;
use crate::data_reading_types::*;
use crate::output_types::{OscilloscopeResults, Output, Results};

fn main() -> std::io::Result<()> {
    let args = args().run();

    let firmware_prefs = if let Some(firmware_pref_enum) = args.firmware_enum {
        let FirmwareEnum::Firmware(prefs) = firmware_pref_enum;
        Some(prefs)
    } else {
        None
    };
    let osc_prefs = if let Some(osc_pref_enum) = args.oscilloscope_enum {
        let OscilloscopeEnum::Oscilloscope(prefs) = osc_pref_enum;
        Some(prefs)
    } else {
        None
    };
    let shelly_prefs = if let Some(shelly_pref_enum) = args.shelly_enum {
        let ShellyEnum::Shelly(prefs) = shelly_pref_enum;
        Some(prefs)
    } else {
        None
    };
    let jetson_prefs = if let Some(jetson_pref_enum) = args.jetson_enum {
        let JetsonEnum::Jetson(prefs) = jetson_pref_enum;
        Some(prefs)
    } else {
        None
    };

    let mut jetson_idx: Option<(usize, usize)> = None;
    let mut shelly_idx: Option<(usize, usize)> = None;
    let mut osc_idx: Option<(usize, usize)> = None;
    let mut firmware_idx: Option<(usize, usize)> = None;
    let (jetson_len, jetson_reader) =
        init_reader("jetson.parquet", args.measurement_location.clone())?;
    let (shelly_len_2, shelly_reader_2) =
        init_reader("shellyPlug.parquet", args.measurement_location.clone())?;
    let (firmware_len, firmware_reader) =
        init_reader("fast_firmware.parquet", args.measurement_location.clone())?;
    let (pico_len, pico_reader) =
        init_reader("usb_osc_data.parquet", args.measurement_location.clone())?;

    let jetson_results = if let Some(jetson_prefs) = &jetson_prefs {
        let mut jetson_power = read_to_power_vector(jetson_len, jetson_reader, |raw_row| {
            let cols = raw_row.into_columns();
            let jetson_measurement = JetsonMeasurement {
                measurement_timestamp: field_to_u64(&cols[0].1).expect("Could not parse Field"),
                current: field_to_u32(&cols[1].1).expect("Could not parse Field"),
                voltage: field_to_u32(&cols[2].1).expect("Could not parse Field"),
            };
            let current_power = (jetson_measurement.current as f64 / 1000.)
                * (jetson_measurement.voltage as f64 / 1000.);
            Ok(PowerSample::Variable(
                jetson_measurement.measurement_timestamp as f64 / 1_000_000.,
                current_power,
            ))
        })?;
        const JETSON_TRIGGER_FACTOR: f64 = 0.1;
        let (jetson_max, jetson_min);
        if args.dont_cut {
            let (jetson_start_idx, jetson_end_idx);
            (jetson_start_idx, jetson_end_idx, jetson_max, jetson_min) = find_data_start_and_end(
                &jetson_power,
                JETSON_TRIGGER_FACTOR,
                jetson_prefs
                    .predicted_maximum
                    .zip(jetson_prefs.predicted_minimum),
                jetson_prefs.frame_size,
                None,
                "Jetson",
                args.plot_intermediates,
            );
            jetson_idx = Some((jetson_start_idx, jetson_end_idx));
        } else {
            (jetson_power, jetson_max, jetson_min) = cut_data_start_and_end(
                jetson_power,
                0.2,
                jetson_prefs
                    .predicted_maximum
                    .zip(jetson_prefs.predicted_minimum),
                jetson_prefs.frame_size,
                None,
                "Jetson",
                args.plot_intermediates,
            );
        }
        save_vec_to_npy(&jetson_power, args.output_path.clone(), "jetson.npy")?;
        let jetson_energy = calc_energy(&jetson_power, None, jetson_idx);

        let jetson_duration = if let Some((start_idx, end_idx)) = jetson_idx {
            let unwrapped = if let PowerVec::Variable(unwrapped) = jetson_power {
                unwrapped
            } else {
                unreachable!()
            };
            unwrapped[end_idx].0 - unwrapped[start_idx].0
        } else {
            let unwrapped = if let PowerVec::Variable(unwrapped) = jetson_power {
                unwrapped
            } else {
                unreachable!()
            };
            unwrapped[unwrapped.len() - 1].0 - unwrapped[0].0
        };

        Some((jetson_energy, jetson_duration))
    } else {
        None
    };

    let shelly_results = if let Some(shelly_prefs) = &shelly_prefs {
        let mut shelly_power = read_to_power_vector(shelly_len_2, shelly_reader_2, |raw_row| {
            let cols = raw_row.into_columns();
            let shelly_measurement = ShellyPlug {
                measurement_timestamp: field_to_u64(&cols[0].1).expect("Could not parse Field"),
                voltage: field_to_f32(&cols[1].1)
                    .expect("Could not parse Field")
                    .into(),
                current: field_to_f32(&cols[2].1)
                    .expect("Could not parse Field")
                    .into(),
                power: field_to_f32(&cols[3].1)
                    .expect("Could not parse Field")
                    .into(),
            };
            // apply calibration
            let mut power = shelly_measurement.power - 40.40749136;
            power *= 0.796818078;
            Ok(PowerSample::Variable(
                shelly_measurement.measurement_timestamp as f64 / 1_000_000.,
                power,
            ))
        })?;
        const SHELLY_TRIGGER_FACTOR: f64 = 0.05;
        let (shelly_max, shelly_min);
        if args.dont_cut {
            let (shelly_start_idx, shelly_end_idx);
            (shelly_start_idx, shelly_end_idx, shelly_max, shelly_min) = find_data_start_and_end(
                &shelly_power,
                SHELLY_TRIGGER_FACTOR,
                shelly_prefs
                    .predicted_maximum
                    .zip(shelly_prefs.predicted_minimum),
                shelly_prefs.frame_size,
                None,
                "Shelly",
                args.plot_intermediates,
            );
            shelly_idx = Some((shelly_start_idx, shelly_end_idx));
        } else {
            (shelly_power, shelly_max, shelly_min) = cut_data_start_and_end(
                shelly_power,
                SHELLY_TRIGGER_FACTOR,
                shelly_prefs
                    .predicted_maximum
                    .zip(shelly_prefs.predicted_minimum),
                shelly_prefs.frame_size,
                None,
                "Shelly",
                args.plot_intermediates,
            );
        }
        save_vec_to_npy(&shelly_power, args.output_path.clone(), "shelly.npy")?;
        let shelly_energy = calc_energy(&shelly_power, None, shelly_idx);
        let shelly_duration = if let Some((start_idx, end_idx)) = shelly_idx {
            let unwrapped = if let PowerVec::Variable(unwrapped) = shelly_power {
                unwrapped
            } else {
                unreachable!()
            };
            unwrapped[end_idx].0 - unwrapped[start_idx].0
        } else {
            let unwrapped = if let PowerVec::Variable(unwrapped) = shelly_power {
                unwrapped
            } else {
                unreachable!()
            };
            unwrapped[unwrapped.len() - 1].0 - unwrapped[0].0
        };
        Some((shelly_energy, shelly_duration))
    } else {
        None
    };

    let osc_results = if let Some(osc_prefs) = &osc_prefs {
        let mut osc_power = read_to_power_vector(pico_len, pico_reader, |raw_row| {
            let cols = raw_row.into_columns();
            let pico_measurement = PicoMeasurement {
                voltage: field_to_f64(&cols[0].1).expect("Could not parse Field"),
                current: field_to_f64(&cols[1].1).expect("Could not parse Field"),
            };
            let current = match osc_prefs.measurement_type {
                OscilloscopeMsmtType::UCurrent => {
                    (pico_measurement.current + 0.003326916) * 0.998687605682019
                }
                OscilloscopeMsmtType::CurrentRanger => {
                    (pico_measurement.current + 0.00226039126953639) * 0.991674394344991
                }
            };
            let voltage = if osc_prefs.use_voltage {
                pico_measurement.voltage
            } else {
                estimate_voltage_from_current(current * 1000.)
            };
            let current_power = voltage * current;
            Ok(PowerSample::Constant(current_power))
        })?;

        let osc_samplerate = osc_prefs.samplerate;
        osc_power = filter_data(osc_power, osc_samplerate, None);
        const OSC_TRIGGER_FACTOR: f64 = 0.25;
        let (osc_max, osc_min);
        if args.dont_cut {
            let (osc_start_idx, osc_end_idx);
            (osc_start_idx, osc_end_idx, osc_max, osc_min) = find_data_start_and_end(
                &osc_power,
                OSC_TRIGGER_FACTOR,
                osc_prefs.predicted_maximum.zip(osc_prefs.predicted_minimum),
                osc_prefs.frame_size,
                Some(osc_samplerate),
                "Picoscope",
                args.plot_intermediates,
            );
            osc_idx = Some((osc_start_idx, osc_end_idx));
        } else {
            (osc_power, osc_max, osc_min) = cut_data_start_and_end(
                osc_power,
                OSC_TRIGGER_FACTOR,
                osc_prefs.predicted_maximum.zip(osc_prefs.predicted_minimum),
                osc_prefs.frame_size,
                Some(osc_prefs.samplerate),
                "Picoscope",
                args.plot_intermediates,
            );
        }
        save_vec_to_npy(&osc_power, args.output_path.clone(), "oscilloscope.npy")?;
        let osc_energy = calc_energy(&osc_power, Some(osc_samplerate), osc_idx);
        let osc_duration = if let Some((start_idx, end_idx)) = osc_idx {
            ((end_idx - start_idx) + 1) as f64 / osc_samplerate
        } else {
            osc_power.len() as f64 / osc_samplerate
        };
        Some((osc_energy, osc_duration))
    } else {
        None
    };

    let firmware_results = if let Some(firmware_prefs) = &firmware_prefs {
        let mut firmware_power = read_to_power_vector(firmware_len, firmware_reader, |raw_row| {
            let cols = raw_row.into_columns();
            let firmware_measurement = FirmwareMeasruement {
                measurement_index: field_to_u16(&cols[0].1).expect("Could not parse Field"),
                current: field_to_u16(&cols[1].1).expect("Could not parse Field"),
            };
            // apply calibration
            let current_current =
                ((firmware_measurement.current as f64 / 1000.) + 0.004704622) * 0.997224237630222;
            let current_power =
                current_current * estimate_voltage_from_current(current_current * 1000.);
            Ok(PowerSample::Constant(current_power))
        })?;

        firmware_power = filter_data(firmware_power, 2000., None);
        const FIRMWARE_TRIGGER_FACTOR: f64 = 0.25;
        let (firmware_max, firmware_min);
        if args.dont_cut {
            let (firmware_start_idx, firmware_end_idx);
            (
                firmware_start_idx,
                firmware_end_idx,
                firmware_max,
                firmware_min,
            ) = find_data_start_and_end(
                &firmware_power,
                FIRMWARE_TRIGGER_FACTOR,
                firmware_prefs
                    .predicted_maximum
                    .zip(firmware_prefs.predicted_minimum),
                firmware_prefs.frame_size,
                Some(2000.),
                "Firmware",
                args.plot_intermediates,
            );
            firmware_idx = Some((firmware_start_idx, firmware_end_idx));
        } else {
            (firmware_power, firmware_max, firmware_min) = cut_data_start_and_end(
                firmware_power,
                0.25,
                firmware_prefs
                    .predicted_maximum
                    .zip(firmware_prefs.predicted_minimum),
                firmware_prefs.frame_size,
                Some(2000.),
                "Firmware",
                args.plot_intermediates,
            );
        }

        save_vec_to_npy(&firmware_power, args.output_path.clone(), "firmware_power.npy")?;
        let firmware_energy = calc_energy(&firmware_power, Some(2000.), firmware_idx);
        let firmware_duration = if let Some((start_idx, end_idx)) = firmware_idx {
            ((end_idx - start_idx) + 1) as f64 / 2000.
        } else {
            firmware_power.len() as f64 / 2000.
        };
        Some((firmware_energy, firmware_duration))
    } else {
        None
    };

    let results = Output {
        jetson_results: jetson_results.map(|(energy, duration)| Results {
            energy,
            duration,
            start_stop_idx: jetson_idx
        }),
        shelly_results: shelly_results.map(|(energy, duration)| Results {
            energy,
            duration,
            start_stop_idx: shelly_idx
        }),
        oscilloscope_results: osc_results.map(|(energy, duration)| OscilloscopeResults {
            results: Results {
                energy,
                duration,
                start_stop_idx: osc_idx
            },
            sample_rate: osc_prefs.clone().unwrap().samplerate,
            use_voltage: osc_prefs.clone().unwrap().use_voltage,
            msmt_type: osc_prefs.unwrap().measurement_type,
        }),
        firmware_results: firmware_results.map(|(energy, duration)| Results {
            energy,
            duration,
            start_stop_idx: firmware_idx
        }),
    };

    println!("{}", results);

    if args.results_storage {
        let serialized_results = serde_saphyr::to_string(&results).unwrap();
        fs::write(args.output_path.clone().join("results.yaml"), serialized_results)?;
    }

    if args.plot {
        let energy_diff_script = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../plot_energy_diffs.py"
        ));
        let from_python = Python::with_gil(|py| -> PyResult<Py<PyAny>> {
            let script: Py<PyAny> = PyModule::from_code_bound(
                py,
                energy_diff_script,
                "plot_energy_diffs.pyc",
                "plot_energy_diffs.pyc",
            )?
            .getattr("main")?
            .into();
            if args.dont_cut {
                script.call1(
                    py,
                    (
                        2000.,
                        osc_prefs.map_or_else(5_000_000, |pref| pref.samplerate),
                        firmware_idx.unwrap_or((0, 0)),
                        osc_idx.unwrap_or((0, 0)),
                        jetson_idx.unwrap_or((0, 0)),
                        shelly_idx.unwrap_or((0, 0)),
                    ),
                )
            } else {
                script.call1(
                    py,
                    (
                        2000.,
                        osc_prefs.map_or_else(5_000_000, |pref| pref.samplerate),
                    ),
                )
            }
        });
        match from_python {
            Ok(_) => {}
            Err(e) => {
                println!("Got Python error: {}", e);
            }
        }
    }

    Ok(())
}
