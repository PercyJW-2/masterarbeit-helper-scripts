mod args;
mod data_actions;
mod data_reading;
mod data_reading_types;
mod output_types;

use pyo3::prelude::*;
use std::{fs, io};

use crate::args::*;
use crate::data_actions::*;
use crate::data_reading::*;
use crate::data_reading_types::*;
use crate::output_types::{OscilloscopeResults, Output};

fn main() -> io::Result<()> {
    let args = args().run();

    let firmware_prefs = match &args.firmware_enum {
        FirmwareEnum::None => None,
        FirmwareEnum::Firmware(firmware) => Some(firmware),
    };
    let osc_prefs = match &args.oscilloscope_enum {
        OscilloscopeEnum::None => None,
        OscilloscopeEnum::Oscilloscope(oscilloscope) => Some(oscilloscope),
    };
    let shelly_prefs = match &args.shelly_enum {
        ShellyEnum::None => None,
        ShellyEnum::Shelly(shelly) => Some(shelly),
    };
    let jetson_prefs = match &args.jetson_enum {
        JetsonEnum::None => None,
        JetsonEnum::Jetson(jetson) => Some(jetson),
    };


    let jetson_results = if let Some(jetson_prefs) = jetson_prefs {
        println!("Calculating Jetson results");
        const JETSON_TRIGGER_FACTOR: f64 = 0.1;
        let results = calculate_results(
            &args,
            "jetson.parquet",
            |raw_row| {
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
            },
            false,
            JETSON_TRIGGER_FACTOR,
            jetson_prefs
                .predicted_maximum
                .zip(jetson_prefs.predicted_minimum),
            jetson_prefs.frame_size,
            None,
            "jetson.npy"
        )?;
        Some(results)
    } else {
        None
    };

    let shelly_results = if let Some(shelly_prefs) = shelly_prefs {
        const SHELLY_TRIGGER_FACTOR: f64 = 0.05;
        let results = calculate_results(
            &args,
            "shelly.parquet",
            |raw_row| {
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
            },
            false,
            SHELLY_TRIGGER_FACTOR,
            shelly_prefs
                .predicted_maximum
                .zip(shelly_prefs.predicted_minimum),
            shelly_prefs.frame_size,
            None,
            "shelly.npy"
        )?;
        Some(results)
    } else {
        None
    };

    let osc_results = if let Some(osc_prefs) = &osc_prefs {
        const OSC_TRIGGER_FACTOR: f64 = 0.25;
        let results = calculate_results(
            &args,
            "usb_osc_data.parquet",
            |raw_row| {
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
            },
            true,
            OSC_TRIGGER_FACTOR,
            osc_prefs
                .predicted_maximum
                .zip(osc_prefs.predicted_minimum),
            osc_prefs.frame_size,
            Some(osc_prefs.samplerate),
            "oscilloscope.npy"
        )?;
        Some(results)
    } else {
        None
    };

    let firmware_results = if let Some(firmware_prefs) = &firmware_prefs {
        const FIRMWARE_TRIGGER_FACTOR: f64 = 0.25;
        let results = calculate_results(
            &args,
            "fast_firmware.parquet",
            |raw_row| {
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
            },
            true,
            FIRMWARE_TRIGGER_FACTOR,
            firmware_prefs
                .predicted_maximum
                .zip(firmware_prefs.predicted_minimum),
            firmware_prefs.frame_size,
            Some(2000.),
            "firmware_power.npy"
        )?;
        Some(results)
    } else {
        None
    };

    let results = Output {
        jetson_results: jetson_results.clone(),
        shelly_results: shelly_results.clone(),
        oscilloscope_results: osc_results.clone().map(|osc_res| OscilloscopeResults {
                results: osc_res,
                sample_rate: osc_prefs.unwrap().samplerate,
                use_voltage: osc_prefs.unwrap().use_voltage,
                msmt_type: osc_prefs.unwrap().measurement_type.clone(),
            }),
        firmware_results: firmware_results.clone(),
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
                        osc_prefs.map_or(5_000_000., |pref| pref.samplerate),
                        firmware_results.map_or((0, 0), |res| res.start_stop_idx.unwrap_or((0, 0))),
                        osc_results.map_or((0, 0), |res| res.start_stop_idx.unwrap_or((0, 0))),
                        jetson_results.map_or((0, 0), |res| res.start_stop_idx.unwrap_or((0, 0))),
                        shelly_results.map_or((0, 0), |res| res.start_stop_idx.unwrap_or((0, 0))),
                    ),
                )
            } else {
                script.call1(
                    py,
                    (
                        2000.,
                        osc_prefs.map_or(5_000_000., |pref| pref.samplerate),
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
