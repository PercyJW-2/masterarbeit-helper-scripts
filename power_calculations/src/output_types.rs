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
        if let Some(jetson) = self.jetson_results {
            write!(f, "Jetson:\t{}\n", jetson)?;
        }
        if let Some(shelly) = self.shelly_results {
            write!(f, "Shelly:\t{}\n", shelly)?;
        }
        if let Some(osc) = self.oscilloscope_results {
            write!(f, "Oscilloscope:\t{}\n", osc.results)?;
        }
        if let Some(firmware) = self.firmware_results {
            write!(f, "Firmware:\t{}\n", firmware)?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct Results {
    pub(crate) energy: f64,
    pub(crate) duration: f64,
    pub(crate) start_stop_idx: Option<(usize, usize)>,
}

impl Display for Results {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Energy:\t{:.2}J, Duration:\t{:.2}s", self.energy, self.duration)?;
        if let Some((start, stop)) = self.start_stop_idx {
            write!(f, ", start_idx:\t{start}, stop_idx:\t{stop}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct OscilloscopeResults {
    pub(crate) results: Results,
    pub(crate) sample_rate: f64,
    pub(crate) use_voltage: bool,
    pub(crate) msmt_type: OscilloscopeMsmtType
}
