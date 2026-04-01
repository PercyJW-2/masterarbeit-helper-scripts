mod args;
mod data_actions;
mod data_reading;
mod data_reading_types;

use pyo3::prelude::*;

use crate::args::*;
use crate::data_actions::*;
use crate::data_reading::*;
use crate::data_reading_types::*;

fn main() -> std::io::Result<()> {
    let args = args().run();

    let FirmwareEnum::Firmware(firmware_prefs) = args.firmware_enum;
    let OscilloscopeEnum::Oscilloscope(osc_prefs) = args.oscilloscope_enum;
    let ShellyEnum::Shelly(shelly_prefs) = args.shelly_enum;
    let JetsonEnum::Jetson(jetson_prefs) = args.jetson_enum;

    let mut jetson_idx: Option<(usize, usize)> = None;
    let mut shelly_idx: Option<(usize, usize)> = None;
    let mut osc_idx: Option<(usize, usize)> = None;
    let mut firmware_idx: Option<(usize, usize)> = None;
    {
        let (jetson_len, jetson_reader) =
            init_reader("jetson.csv", args.measurement_location.clone())?;
        let (shelly_len_2, shelly_reader_2) =
            init_reader("shellyPlug.csv", args.measurement_location.clone())?;
        let (firmware_len, firmware_reader) =
            init_reader("fast_firmware.csv", args.measurement_location.clone())?;
        let (pico_len, pico_reader) =
            init_reader("usb_osc_data.csv", args.measurement_location.clone())?;

        let mut jetson_power = read_to_power_vector(jetson_len, jetson_reader, 1, |str_record| {
            let jetson_measurement: JetsonMeasurement = str_record.deserialize(None)?;
            let current_power = (jetson_measurement.current as f64 / 1000.)
                * (jetson_measurement.voltage as f64 / 1000.);
            Ok(PowerSample::Variable(
                jetson_measurement.measurement_timestamp as f64 / 1_000_000.,
                current_power,
            ))
        })?;
        const JETSON_TRIGGER_FACTOR: f64 = 0.2;
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
        save_vec_to_npy(&jetson_power, "jetson.npy")?;
        let jetson_energy = calc_energy(&jetson_power, None);

        let mut shelly_power_2 =
            read_to_power_vector(shelly_len_2, shelly_reader_2, 1, |str_record| {
                let shelly_measurement: ShellyPlug = str_record.deserialize(None)?;
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
                &shelly_power_2,
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
            (shelly_power_2, shelly_max, shelly_min) = cut_data_start_and_end(
                shelly_power_2,
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
        save_vec_to_npy(&shelly_power_2, "shelly.npy")?;
        let shelly_energy_2 = calc_energy(&shelly_power_2, None);

        let mut osc_power = read_to_power_vector(pico_len, pico_reader, 100_000, |str_record| {
            let pico_measurement: PicoMeasurement = str_record.deserialize(None)?;
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
        save_vec_to_npy(&osc_power, "oscilloscope.npy")?;
        let osc_energy = calc_energy(&osc_power, Some(osc_samplerate));

        let mut firmware_power =
            read_to_power_vector(firmware_len, firmware_reader, 1, |str_record| {
                let firmware_measurement: FirmwareMeasruement = str_record.deserialize(None)?;
                // apply calibration
                let current_current = ((firmware_measurement.current as f64 / 1000.) + 0.004704622)
                    * 0.997224237630222;
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

        save_vec_to_npy(&firmware_power, "firmware_power.npy")?;

        let firmware_energy = calc_energy(&firmware_power, Some(2000.));

        let osc_duration = if let Some((start_idx, end_idx)) = osc_idx {
            ((end_idx - start_idx) + 1) as f64 / osc_samplerate
        } else {
            osc_power.len() as f64 / osc_samplerate
        };
        let firmware_duration = if let Some((start_idx, end_idx)) = firmware_idx {
            ((end_idx - start_idx) + 1) as f64 / 2000.
        } else {
            firmware_power.len() as f64 / 2000.
        };
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
        let shelly_duration = if let Some((start_idx, end_idx)) = shelly_idx {
            let unwrapped = if let PowerVec::Variable(unwrapped) = shelly_power_2 {
                unwrapped
            } else {
                unreachable!()
            };
            unwrapped[end_idx].0 - unwrapped[start_idx].0
        } else {
            let unwrapped = if let PowerVec::Variable(unwrapped) = shelly_power_2 {
                unwrapped
            } else {
                unreachable!()
            };
            unwrapped[unwrapped.len() - 1].0 - unwrapped[0].0
        };

        println!(
            "
        Oscilloscope Energy:                                       {osc_energy:.2} Joule\t Max: {osc_max:.2}\t Min: {osc_min:.2}
        Firmware Energy (Estimated voltage from calculated curve): {firmware_energy:.2} Joule\t Max: {firmware_max:.2}\t Min: {firmware_min:.2}
        Jetson Energy:                                             {jetson_energy:.2} Joule\t Max: {jetson_max:.2}\t Min: {jetson_min:.2}
        Shelly Energy:                                             {shelly_energy_2:.2} Joule\t Max: {shelly_max:.2}\t Min: {shelly_min:.2}

        Oscilloscope Duration:                                     {osc_duration:.2} Seconds
        Firmware Duration:                                         {firmware_duration:.2} Seconds
        Jetson Duration:                                           {jetson_duration:.2} Seconds
        Shelly Duration:                                           {shelly_duration:.2} Seconds
        ",
        );
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
                        osc_prefs.samplerate,
                        firmware_idx.unwrap(),
                        osc_idx.unwrap(),
                        jetson_idx.unwrap(),
                        shelly_idx.unwrap(),
                    ),
                )
            } else {
                script.call1(py, (2000., osc_prefs.samplerate))
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
