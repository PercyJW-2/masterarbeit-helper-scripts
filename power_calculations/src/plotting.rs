use std::ffi::CString;
use log::error;
use pyo3::prelude::*;
use crate::args::Args;
use crate::output_types::Results;

pub(crate) fn plot_energy_diffs(
    args: &Args,
    firmware_results: Option<&Results>,
    osc_results: Option<&Results>,
    jetson_results: Option<&Results>,
    shelly_results: Option<&Results>,
) -> PyResult<()> {
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

        let firmware_sr = args.firmware.as_ref().map_or(2_000., |pref| pref.samplerate);
        let osc_sr = args.oscilloscope.as_ref().map_or(5_000_000., |pref| pref.samplerate);

        let get_idx = |res: Option<&Results>| res.map_or((0, 0), |r| r.start_stop_idx.unwrap_or((0, 0)));

        if args.dont_cut {
            script.call1(
                py,
                (
                    firmware_sr,
                    osc_sr,
                    &args.output_path,
                    get_idx(firmware_results),
                    get_idx(osc_results),
                    get_idx(jetson_results),
                    get_idx(shelly_results),
                ),
            )?;
        } else {
            script.call1(
                py,
                (
                    firmware_sr,
                    osc_sr,
                    &args.output_path,
                ),
            )?;
        }
        Ok(script)
    });

    match from_python {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Got Python error: {}", e);
            Err(e)
        }
    }
}
