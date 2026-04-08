use std::fmt::Display;
use serde::Serialize;
use crate::args::OscilloscopeMsmtType;

#[derive(Debug, Serialize)]
pub(crate) struct Output {
    pub(crate) jetson_results: Option<Results>,
    pub(crate) shelly_results: Option<Results>,
    pub(crate) oscilloscope_results: Option<OscilloscopeResults>,
    pub(crate) firmware_results: Option<Results>
}

impl Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(jetson) = &self.jetson_results {
            writeln!(f, "Jetson:\t{}\n", jetson)?;
        }
        if let Some(shelly) = &self.shelly_results {
            writeln!(f, "Shelly:\t{}\n", shelly)?;
        }
        if let Some(osc) = &self.oscilloscope_results {
            writeln!(f, "Oscilloscope:\t{}\n", osc.results)?;
        }
        if let Some(firmware) = &self.firmware_results {
            writeln!(f, "Firmware:\t{}\n", firmware)?;
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
