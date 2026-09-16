use std::fmt::Display;
use std::path::Path;
use std::{fs, io};
use serde::Serialize;
use crate::args::{Args, MeasurementEnvironment, OscilloscopeMsmtType};

#[derive(Debug, Serialize)]
pub(crate) struct Output {
    pub(crate) measurement_environment: MeasurementEnvironment,
    pub(crate) jetson_results: Option<Results>,
    pub(crate) shelly_results: Option<Results>,
    pub(crate) oscilloscope_results: Option<OscilloscopeResults>,
    pub(crate) tek_scope_results: Option<TekScopeResults>,
    pub(crate) firmware_results: Option<Results>,
    pub(crate) hailo_rt_results: Option<Results>,
    pub(crate) nv_gpu_results: Option<Results>,
}

impl Output {
    pub(crate) fn build(
        args: &Args,
        jetson_results: Option<Results>,
        shelly_results: Option<Results>,
        osc_results: Option<Results>,
        tekscope_results: Option<Results>,
        firmware_results: Option<Results>,
        hailo_rt_results: Option<Results>,
        nv_gpu_results: Option<Results>,
    ) -> Self {
        Self {
            measurement_environment: args.environment.clone(),
            jetson_results,
            shelly_results,
            oscilloscope_results: osc_results.map(|osc_res| {
                let osc_args = args.oscilloscope.as_ref().expect("Oscilloscope arguments not found");
                OscilloscopeResults {
                    results: osc_res,
                    sample_rate: osc_args.samplerate,
                    use_voltage: osc_args.use_voltage,
                    msmt_type: osc_args.measurement_type.clone(),
                }
            }),
            tek_scope_results: tekscope_results.map(|tek_res| {
                let tek_args = args.tekscope.as_ref().expect("TekScope arguments not found");
                TekScopeResults {
                    results: tek_res,
                    sample_rate: tek_args.samplerate,
                }
            }),
            firmware_results,
            hailo_rt_results,
            nv_gpu_results,
        }
    }

    pub(crate) fn save_yaml(&self, output_path: &Path) -> io::Result<()> {
        let serialized_results = serde_saphyr::to_string(self)
            .map_err(io::Error::other)?;
        fs::write(output_path.join("results.yaml"), serialized_results)
    }
}

impl Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(jetson) = &self.jetson_results {
            writeln!(f, "Jetson:\t\t{}", jetson)?;
        }
        if let Some(shelly) = &self.shelly_results {
            writeln!(f, "Shelly:\t\t{}", shelly)?;
        }
        if let Some(osc) = &self.oscilloscope_results {
            writeln!(f, "Oscilloscope:\t{}", osc.results)?;
        }
        if let Some(firmware) = &self.firmware_results {
            writeln!(f, "Firmware:\t{}", firmware)?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[derive(Clone)]
pub(crate) struct Results {
    pub(crate) energy: f64,
    pub(crate) duration: f64,
    pub(crate) start_stop_idx: Option<(usize, usize)>,
    pub(crate) max_frame_energy: f64,
    pub(crate) idle_frame_energy: f64
}

impl Display for Results {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Energy:\t{:.2}J, Duration:\t{:.2}s", self.energy, self.duration)
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct OscilloscopeResults {
    pub(crate) results: Results,
    pub(crate) sample_rate: f64,
    pub(crate) use_voltage: bool,
    pub(crate) msmt_type: OscilloscopeMsmtType
}

#[derive(Debug, Serialize)]
pub(crate) struct TekScopeResults {
    pub(crate) results: Results,
    pub(crate) sample_rate: f64,
}
