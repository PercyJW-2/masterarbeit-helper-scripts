mod args;
mod data_actions;
mod data_reading;
mod data_reading_types;

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

    let (jetson_len, jetson_reader) = init_reader("jetson.csv", args.measurement_location.clone())?;
    let (shelly_len, shelly_reader) =
        init_reader("shellyPlug.csv", args.measurement_location.clone())?;
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
    jetson_power = cut_data_start_and_end(
        jetson_power,
        jetson_prefs.beginning_trigger_value,
        jetson_prefs.end_trigger_value,
        1. / 500.,
        None,
        "Jetson",
    );
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

    let mut shelly_power_2 =
        read_to_power_vector(shelly_len_2, shelly_reader_2, 1, |str_record| {
            let shelly_measurement: ShellyPlug = str_record.deserialize(None)?;
            // apply calibration
            let power = -0.001090707 * shelly_measurement.power.powf(2.)
                + 0.903935016 * shelly_measurement.power;
            Ok(PowerSample::Variable(
                shelly_measurement.measurement_timestamp as f64 / 1_000_000.,
                power,
            ))
        })?;
    shelly_power_2 = cut_data_start_and_end(
        shelly_power_2,
        shelly_prefs.beginning_trigger_value,
        shelly_prefs.end_trigger_value,
        1.0,
        None,
        "Shelly",
    );
    let shelly_energy_2 = calc_energy(&shelly_power_2, None);

    println!(
        "Shelly Energy (Calculated with internal Power calculation): {shelly_energy_2:.2} Joule"
    );

    let mut osc_power = read_to_power_vector(pico_len, pico_reader, 100_000, |str_record| {
        let pico_measurement: PicoMeasurement = str_record.deserialize(None)?;
        let current = pico_measurement.current * 0.88607;
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
    osc_power = cut_data_start_and_end(
        osc_power,
        osc_prefs.beginning_trigger_value,
        osc_prefs.end_trigger_value,
        osc_prefs.frame_size,
        Some(osc_prefs.samplerate),
        "Picoscope",
    );
    /*osc_power = if let Some(threshold) = osc_prefs.beginning_trigger_value {
        cut_start(
            osc_power,
            threshold,
            osc_prefs.frame_size,
            Some(osc_samplerate),
            false,
        )
    } else {
        println!("Starting calibration assistant (Oscilloscope Beginning)");
        let power = cut_start(
            osc_power,
            0.0,
            osc_prefs.frame_size,
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
            osc_prefs.frame_size,
            Some(osc_samplerate),
            true,
        )
    };
    osc_power = if let Some(threshold) = osc_prefs.end_trigger_value {
        cut_end(
            osc_power,
            threshold,
            osc_prefs.frame_size,
            Some(osc_samplerate),
            false,
        )
    } else {
        println!("Starting calibration assistant (Oscilloscope Ending)");
        let power = cut_end(
            osc_power,
            0.0,
            osc_prefs.frame_size,
            Some(osc_samplerate),
            true,
        );
        print!("Provide threshold: ");
        io::stdout().flush().expect("Sould not flush stdout");
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;
        let threshold: f64 = buffer.trim().parse().expect("Could not parse float");
        cut_end(
            power,
            threshold,
            osc_prefs.frame_size,
            Some(osc_samplerate),
            true,
        )
    };*/
    save_vec_to_npy(&osc_power, "oscilloscope.npy")?;
    let osc_energy = calc_energy(&osc_power, Some(osc_samplerate));

    println!("Osc Energy: {osc_energy:.2} Joule");

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
            //current_current *= 1.03;
            let current_power =
                (current_current / 1000.) * estimate_voltage_from_current(current_current);
            Ok(PowerSample::Constant(current_power))
        })?;

    firmware_power = filter_data(firmware_power, 2000., None);
    firmware_power = cut_data_start_and_end(
        firmware_power,
        firmware_prefs.beginning_trigger_value,
        firmware_prefs.end_trigger_value,
        firmware_prefs.frame_size,
        Some(2000.),
        "Firmware",
    );

    /*firmware_power = if let Some(threshold) = firmware_prefs.beginning_trigger_value {
        cut_start(
            firmware_power,
            threshold,
            firmware_prefs.frame_size,
            Some(2000.),
            false,
        )
    } else {
        println!("Starting calibration assistant (Firmware Beginning)");
        let power = cut_start(
            firmware_power,
            0.0,
            firmware_prefs.frame_size,
            Some(2000.),
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
            firmware_prefs.frame_size,
            Some(2000.),
            true,
        )
    };
    firmware_power = if let Some(threshold) = firmware_prefs.end_trigger_value {
        cut_end(
            firmware_power,
            threshold,
            firmware_prefs.frame_size,
            Some(2000.),
            false,
        )
    } else {
        println!("Starting calibration assistant (Firmware Ending)");
        let power = cut_end(
            firmware_power,
            0.0,
            firmware_prefs.frame_size,
            Some(2000.),
            true,
        );
        print!("Provide threshold: ");
        io::stdout().flush().expect("Could not flush stdout");
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;
        let threshold: f64 = buffer.trim().parse().expect("Could not parse float");
        cut_end(
            power,
            threshold,
            firmware_prefs.frame_size,
            Some(2000.),
            true,
        )
    };*/

    let actual_firmware_samplerate = {
        let diff_percentage = firmware_power.len() as f64 / osc_power.len() as f64;
        osc_samplerate * diff_percentage
    };
    println!("Actual firmware samplerate: {actual_firmware_samplerate}");

    save_vec_to_npy(&firmware_power, "firmware_power.npy")?;

    let firmware_energy = calc_energy(&firmware_power, Some(actual_firmware_samplerate)); // placeholder
    println!(
        "Firmware Energy (Estimated voltage from calculated curve): {firmware_energy:.2} Joule"
    );

    Ok(())
}
