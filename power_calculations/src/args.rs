use bpaf::Bpaf;
use std::{fmt::Display, path::PathBuf, str::FromStr};
use serde::Serialize;

const DEFAULT_THRESHOLD: f64 = 1. / 10.;

#[derive(Debug, Clone, Serialize)]
pub(crate) enum MeasurementEnvironment {
    Static,
    Jetson,
    M2,
}

impl MeasurementEnvironment {
    pub(crate) const fn get_scale_factor(&self) -> f64 {
        match self {
            Self::Static => 1.,
            Self::Jetson => 1.0 - 0.03127795823408493, //TODO check if value is valid
            Self::M2 => 1.,
        }
    }

    pub(crate) const fn get_resistance(&self) -> f64 {
        match self {
            Self::Static => 0.,
            Self::Jetson => 0.2934,
            Self::M2 => 0.0797,
        }
    }

    pub(crate) const fn get_initial_voltage(&self) -> f64 {
        match self {
            Self::Static | Self::Jetson => 18.95,
            Self::M2 => 3.313
        }
    }
}

impl FromStr for MeasurementEnvironment {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "static" => Ok(MeasurementEnvironment::Static),
            "jetson" => Ok(MeasurementEnvironment::Jetson),
            "m.2" => Ok(MeasurementEnvironment::M2),
            _ => Err(format!("String {s} is invalid"))
        }
    }
}

impl Display for MeasurementEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static => write!(f, "Static"),
            Self::Jetson => write!(f, "Jetson"),
            Self::M2 => write!(f, "M_2"),
        }
    }
}

#[derive(Bpaf, Debug, Clone)]
pub(crate) struct MsmtMethod {
    /// expected maximum energy value of measurement window of duration determined in frame_size
    #[bpaf(short, long)]
    pub(crate) predicted_maximum: Option<f64>,
    /// expected minimum energy value of measurement window of duration determined in frame_size
    #[bpaf(short, long)]
    pub(crate) predicted_minimum: Option<f64>,
    /// averaging frame size - configures duration of frame size which is used to detect the
    /// beginning of the dataset. unit is in seconds
    #[bpaf(short, long, fallback(DEFAULT_THRESHOLD), display_fallback)]
    pub(crate) frame_size: f64,
}

#[derive(Bpaf, Debug, Clone)]
#[bpaf(command("firmware"), adjacent)]
pub(crate) struct Firmware {
    #[bpaf(external)]
    pub(crate) msmt_method: MsmtMethod,
    /// samplerate that was used to record firmware data
    #[bpaf(short, long, fallback(2000.), display_fallback)]
    pub(crate) samplerate: f64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) enum OscilloscopeMsmtType {
    UCurrent,
    CurrentRanger,
    INA225,
}

impl FromStr for OscilloscopeMsmtType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ucurrent" => Ok(OscilloscopeMsmtType::UCurrent),
            "currentranger" => Ok(OscilloscopeMsmtType::CurrentRanger),
            "ina225" => Ok(OscilloscopeMsmtType::INA225),
            _ => Err(format!("String {s} is invalid")),
        }
    }
}

impl Display for OscilloscopeMsmtType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UCurrent => write!(f, "UCurrent"),
            Self::CurrentRanger => write!(f, "CurrentRanger"),
            Self::INA225 => write!(f, "INA225"),
        }
    }
}

#[derive(Bpaf, Debug, Clone)]
#[bpaf(command("oscilloscope"), adjacent)]
pub(crate) struct Oscilloscope {
    #[bpaf(external)]
    pub(crate) msmt_method: MsmtMethod,
    /// use osc-voltage measurement instead of voltage estimation
    #[bpaf(short('v'), long)]
    pub(crate) use_voltage: bool,
    /// oscilloscope samplerate, unit is in samples per second
    #[bpaf(short, long, fallback(5_000_000.), display_fallback)]
    pub(crate) samplerate: f64,
    /// set measurement type to configure which calibration is used, Options are UCurrent or
    /// CurrentRanger
    #[bpaf(
        short,
        long,
        fallback(OscilloscopeMsmtType::INA225),
        display_fallback
    )]
    pub(crate) measurement_type: OscilloscopeMsmtType,
}

#[derive(Bpaf, Debug, Clone)]
#[bpaf(command("tekscope"), adjacent)]
pub(crate) struct Tekscope {
    #[bpaf(external)]
    pub(crate) msmt_method: MsmtMethod,
    /// oscilloscope samplerate, unit is in samples per second
    #[bpaf(short, long, fallback(5_000_000.), display_fallback)]
    pub(crate) samplerate: f64,
}

#[derive(Bpaf, Debug, Clone)]
#[bpaf(command("shelly"), adjacent)]
pub(crate) struct Shelly {
    #[bpaf(external)]
    pub(crate) msmt_method: MsmtMethod
}

#[derive(Bpaf, Debug, Clone)]
#[bpaf(command("jetson"), adjacent)]
pub(crate) struct Jetson {
    #[bpaf(external)]
    pub(crate) msmt_method: MsmtMethod
}

#[derive(Bpaf, Debug, Clone)]
#[bpaf(command("hailo_rt"), adjacent)]
pub(crate) struct HailoRT {
    #[bpaf(external)]
    pub(crate) msmt_method: MsmtMethod
}

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options, version)]
pub(crate) struct Args {
    /// Measurement location
    #[bpaf(short, long)]
    pub(crate) measurement_location: PathBuf,
    /// plot final power
    #[bpaf(short, long)]
    pub(crate) plot: bool,
    /// plot intermediates
    #[bpaf(short('i'), long)]
    pub(crate) plot_intermediates: bool,
    /// per default the data is cut, enable this to output each start and end location instead
    #[bpaf(short('c'), long)]
    pub(crate) dont_cut: bool,
    /// Output Path location where all data and results are stored, if not provided the current
    /// folder is used
    #[bpaf(short, long, fallback(PathBuf::from("./")))]
    pub(crate) output_path: PathBuf,
    /// store results in results.yaml file
    #[bpaf(short, long)]
    pub(crate) results_storage: bool,
    /// provide an estimated duration to fit the detected action onto this duration
    /// normal data detection is still done, but the window is now enlarged or reduced to fit this duration
    /// Unit is in seconds
    #[bpaf(short, long)]
    pub(crate) estimated_duration: Option<f64>,
    /// Apply Lowpass-Filter on u.RECS and Oscilloscope data. Frequency=25%*Sample-Rate
    #[bpaf(short('f'), long)]
    pub(crate) apply_filter: bool,
    /// measurement environment to mitigate calibration errors
    #[bpaf(short, long, fallback(MeasurementEnvironment::Jetson), display_fallback)]
    pub(crate) environment: MeasurementEnvironment,
    /// Settings for firmware measurements
    #[bpaf(external, optional)]
    pub(crate) firmware: Option<Firmware>,
    /// Settings for oscilloscope measurements
    #[bpaf(external, optional)]
    pub(crate) oscilloscope: Option<Oscilloscope>,
    /// Settings for tekscope measurements
    #[bpaf(external, optional)]
    pub(crate) tekscope: Option<Tekscope>,
    /// Settings for shelly measurements
    #[bpaf(external, optional)]
    pub(crate) shelly: Option<Shelly>,
    /// Settings for jetson measurements
    #[bpaf(external, optional)]
    pub(crate) jetson: Option<Jetson>,
    /// Settings for hailort measurements
    #[bpaf(external, optional)]
    pub(crate) hailo_r_t: Option<HailoRT>
}
