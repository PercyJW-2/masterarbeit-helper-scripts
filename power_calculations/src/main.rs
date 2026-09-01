mod args;
mod data_actions;
mod data_reading;
mod data_reading_types;
mod output_types;

use crate::args::*;
use crate::data_actions::*;
use crate::data_reading::*;
use crate::data_reading_types::*;
use crate::output_types::{OscilloscopeResults, Output, TekScopeResults};
use pyo3::prelude::*;
use std::ffi::CString;
use std::{fs, io};
use log::{error, info};

fn main() -> io::Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .env()
        .init().unwrap();
    let args = args().run();

    let jetson_results = if let Some(jetson_prefs) = &args.jetson {
        info!("Calculating Jetson results");
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
                .msmt_method
                .predicted_maximum
                .zip(jetson_prefs.msmt_method.predicted_minimum),
            jetson_prefs.msmt_method.frame_size,
            None,
            "jetson.npy",
        )?;
        Some(results)
    } else {
        None
    };

    let shelly_results = if let Some(shelly_prefs) = &args.shelly {
        info!("Calculating shelly results");
        const SHELLY_TRIGGER_FACTOR: f64 = 0.05;
        let results = calculate_results(
            &args,
            "shellyPlug.parquet",
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
                let mut power = shelly_measurement.power - 41.36936767;
                power *= 0.795372365;
                Ok(PowerSample::Variable(
                    shelly_measurement.measurement_timestamp as f64 / 1_000_000.,
                    power,
                ))
            },
            false,
            SHELLY_TRIGGER_FACTOR,
            shelly_prefs
                .msmt_method
                .predicted_maximum
                .zip(shelly_prefs.msmt_method.predicted_minimum),
            shelly_prefs.msmt_method.frame_size,
            None,
            "shelly.npy",
        )?;
        Some(results)
    } else {
        None
    };

    let hailo_results = if let Some(hailo_prefs) = &args.hailo_r_t {
        info!("Calculating Hailo results");
        const HAILO_TRIGGER_FACTOR: f64 = 0.05; // TODO factor is not calibrated
        let results = calculate_results(
            &args,
            "hailo_rt.parquet",
            |raw_row| {
                let cols = raw_row.into_columns();
                let hailo_time = field_to_u64(&cols[0].1).expect("Could not parse Field");
                let hailo_power = field_to_f32(&cols[1].1).expect("Could not parse Field");
                Ok(PowerSample::Variable(
                    hailo_time as f64 / 1_000_000.,
                    hailo_power as f64
                ))
            },
            false,
            HAILO_TRIGGER_FACTOR,
            hailo_prefs.msmt_method.predicted_maximum.zip(hailo_prefs.msmt_method.predicted_minimum),
            hailo_prefs.msmt_method.frame_size,
            None,
            "hailo_rt.py",
        )?;
        Some(results)
    } else {
        None
    };

    let osc_results = if let Some(osc_prefs) = &args.oscilloscope {
        info!("Calculating OSC results");
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
                    OscilloscopeMsmtType::INA225 => {
                        (pico_measurement.current + 0.0004272598504) * 1.99000512058047
                    }
                };
                let voltage = if osc_prefs.use_voltage {
                    pico_measurement.voltage
                } else {
                    estimate_voltage_from_current(current * 1000., &args.environment)
                };
                let current_power = voltage * current;
                Ok(PowerSample::Constant(current_power))
            },
            args.apply_filter,
            OSC_TRIGGER_FACTOR,
            osc_prefs.msmt_method.predicted_maximum.zip(osc_prefs.msmt_method.predicted_minimum),
            osc_prefs.msmt_method.frame_size,
            Some(osc_prefs.samplerate),
            "oscilloscope.npy",
        )?;
        Some(results)
    } else {
        None
    };

    let tekscope_results = if let Some(tek_prefs) = &args.tekscope {
        info!("Calculating TekScope results");
        const TEK_TRIGGER_FACTOR: f64 = 0.25;
        let results = calculate_results(
            &args,
            "tek_hsi.parquet",
            |raw_row| {
                let cols = raw_row.into_columns();
                let tek_measurement = TekMeasurement {
                    current: field_to_f64(&cols[0].1).expect("Could not parse Field"),
                };
                let voltage = estimate_voltage_from_current(tek_measurement.current, &args.environment);
                let current_power = voltage * tek_measurement.current;
                Ok(PowerSample::Constant(current_power))
            },
            args.apply_filter,
            TEK_TRIGGER_FACTOR,
            tek_prefs.msmt_method.predicted_maximum.zip(tek_prefs.msmt_method.predicted_minimum),
            tek_prefs.msmt_method.frame_size,
            Some(tek_prefs.samplerate),
            "tekScope.npy",
        )?;
        Some(results)
    } else {
        None
    };

    let firmware_results = if let Some(firmware_prefs) = &args.firmware {
        info!("Calculating Firmware results");
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
                let current_current = ((firmware_measurement.current as f64 / 1000.) + 0.004704622)
                    * 0.997224237630222;
                let current_power =
                    current_current * estimate_voltage_from_current(current_current * 1000., &args.environment);
                let corrected_firmware_power = args.environment.get_scale_factor() * current_power;
                Ok(PowerSample::Constant(corrected_firmware_power))
            },
            args.apply_filter,
            FIRMWARE_TRIGGER_FACTOR,
            firmware_prefs
                .msmt_method
                .predicted_maximum
                .zip(firmware_prefs.msmt_method.predicted_minimum),
            firmware_prefs.msmt_method.frame_size,
            Some(firmware_prefs.samplerate),
            "firmware_power.npy",
        )?;
        Some(results)
    } else {
        None
    };

    let results = Output {
        measurement_environment: args.environment,
        jetson_results: jetson_results.clone(),
        shelly_results: shelly_results.clone(),
        oscilloscope_results: osc_results.clone().map(|osc_res| OscilloscopeResults {
            results: osc_res,
            sample_rate: args.oscilloscope.as_ref().unwrap().samplerate,
            use_voltage: args.oscilloscope.as_ref().unwrap().use_voltage,
            msmt_type: args.oscilloscope.as_ref().unwrap().measurement_type.clone(),
        }),
        tek_scope_results: tekscope_results.clone().map(|tek_res| TekScopeResults {
            results: tek_res,
            sample_rate: args.tekscope.as_ref().unwrap().samplerate
        }),
        firmware_results: firmware_results.clone(),
        hailo_rt_results: hailo_results.clone(),
    };

    info!("{}", results);

    if args.results_storage {
        let serialized_results = serde_saphyr::to_string(&results).unwrap();
        fs::write(
            args.output_path.clone().join("results.yaml"),
            serialized_results,
        )?;
    }

    if args.plot {
        let energy_diff_script = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../plot_energy_diffs.py"
        ));
        let energy_diff_script_cstr = CString::new(energy_diff_script)?;
        let from_python = Python::attach(|py| -> PyResult<Py<PyAny>> {
            let script: Py<PyAny> = PyModule::from_code(
                py,
                energy_diff_script_cstr.as_ref(),
                c"plot_energy_diffs.pyc",
                c"plot_energy_diffs.pyc",
            )?
            .getattr("main")?
            .into();
            if args.dont_cut {
                script.call1(
                    py,
                    (
                        args.firmware.as_ref().map_or(2_000., |pref| pref.samplerate),
                        args.oscilloscope.as_ref().map_or(5_000_000., |pref| pref.samplerate),
                        args.output_path,
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
                        args.firmware.as_ref().map_or(2_000., |pref| pref.samplerate),
                        args.oscilloscope.as_ref().map_or(5_000_000., |pref| pref.samplerate),
                        args.output_path,
                    ),
                )
            }
        });
        match from_python {
            Ok(_) => {}
            Err(e) => {
                error!("Got Python error: {}", e);
            }
        }
    }

    Ok(())
}
