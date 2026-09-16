mod args;
mod data_actions;
mod data_reading;
mod data_reading_types;
mod output_types;
mod plotting;

use crate::args::args;
use crate::data_actions::{
    process_firmware, process_hailo, process_jetson, process_oscilloscope, process_shelly,
    process_tekscope, process_nvgpu
};
use crate::output_types::Output;
use log::{error, info};
use std::io;

fn main() -> io::Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .env()
        .init()
        .map_err(io::Error::other)?;

    let args = args().run();

    let jetson_results = process_jetson(&args)?;
    let shelly_results = process_shelly(&args)?;
    let hailo_results = process_hailo(&args)?;
    let nvgpu_results = process_nvgpu(&args)?;
    let osc_results = process_oscilloscope(&args)?;
    let tekscope_results = process_tekscope(&args)?;
    let firmware_results = process_firmware(&args)?;

    let results = Output::build(
        &args,
        jetson_results.clone(),
        shelly_results.clone(),
        osc_results.clone(),
        tekscope_results.clone(),
        firmware_results.clone(),
        hailo_results.clone(),
        nvgpu_results.clone(),
    );

    info!("{results}");

    if args.results_storage {
        results.save_yaml(&args.output_path)?;
    }

    if args.plot
        && let Err(e) = plotting::plot_energy_diffs(
            &args,
            firmware_results.as_ref(),
            osc_results.as_ref(),
            jetson_results.as_ref(),
            shelly_results.as_ref(),
        )
    {
        error!("Got Python error: {e}");
    }

    Ok(())
}
