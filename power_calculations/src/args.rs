use bpaf::Bpaf;
use std::{fmt::Display, path::PathBuf, str::FromStr};
use serde::Serialize;

const DEFAULT_THRESHOLD: f64 = 1. / 10.;

#[derive(Bpaf, Debug, Clone)]
pub(crate) struct Firmware {
    /// expected maximum energy value of measurement window of duration determined in frame_size
    #[bpaf(short, long)]
    pub(crate) predicted_maximum: Option<f64>,
    /// expected minimum energy value of measurement window of duration determined in frame_size
    #[bpaf(short, long)]
    pub(crate) predicted_minimum: Option<f64>,
    /// averaging frame size - configures duration of frame size wich is used to detect the
    /// beginning of the dataset. unit is in seconds
    #[bpaf(short, long, fallback(DEFAULT_THRESHOLD), display_fallback)]
    pub(crate) frame_size: f64,
}

#[derive(Bpaf, Debug, Clone)]
pub(crate) enum FirmwareEnum {
    #[bpaf(command, adjacent)]
    Firmware(#[bpaf(external(firmware))] Firmware),
}

#[derive(Debug, Clone, Serialize)]
pub(crate) enum OscilloscopeMsmtType {
    UCurrent,
    CurrentRanger,
}

impl FromStr for OscilloscopeMsmtType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ucurrent" => Ok(OscilloscopeMsmtType::UCurrent),
            "currentranger" => Ok(OscilloscopeMsmtType::CurrentRanger),
            _ => Err(format!("String {s} is invalid")),
        }
    }
}

impl Display for OscilloscopeMsmtType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UCurrent => write!(f, "UCurrent"),
            Self::CurrentRanger => write!(f, "CurrentRanger"),
        }
    }
}

#[derive(Bpaf, Debug, Clone)]
pub(crate) struct Oscilloscope {
    /// expected maximum energy value of measurement window of duration determined in frame_size
    #[bpaf(short, long)]
    pub(crate) predicted_maximum: Option<f64>,
    /// expected minimum energy value of measurement window of duration determined in frame_size
    #[bpaf(short, long)]
    pub(crate) predicted_minimum: Option<f64>,
    /// averaging frame size - configures duration of frame size wich is used to detect the
    /// beginning of the dataset. unit is in seconds
    #[bpaf(short, long, fallback(DEFAULT_THRESHOLD), display_fallback)]
    pub(crate) frame_size: f64,
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
        fallback(OscilloscopeMsmtType::CurrentRanger),
        display_fallback
    )]
    pub(crate) measurement_type: OscilloscopeMsmtType,
}

#[derive(Bpaf, Debug, Clone)]
pub(crate) enum OscilloscopeEnum {
    #[bpaf(command, adjacent)]
    Oscilloscope(#[bpaf(external(oscilloscope))] Oscilloscope),
}

#[derive(Bpaf, Debug, Clone)]
pub(crate) struct Shelly {
    /// expected maximum energy value of measurement window of duration determined in frame_size
    #[bpaf(short, long)]
    pub(crate) predicted_maximum: Option<f64>,
    /// expected minimum energy value of measurement window of duration determined in frame_size
    #[bpaf(short, long)]
    pub(crate) predicted_minimum: Option<f64>,
    /// averaging frame size - configures duration of frame size wich is used to detect the
    /// beginning of the dataset. unit is in seconds
    #[bpaf(short, long, fallback(DEFAULT_THRESHOLD), display_fallback)]
    pub(crate) frame_size: f64,
}

#[derive(Bpaf, Debug, Clone)]
pub(crate) enum ShellyEnum {
    #[bpaf(command, adjacent)]
    Shelly(#[bpaf(external(shelly))] Shelly),
}

#[derive(Bpaf, Debug, Clone)]
pub(crate) struct Jetson {
    /// expected maximum energy value of measurement window of duration determined in frame_size
    #[bpaf(short, long)]
    pub(crate) predicted_maximum: Option<f64>,
    /// expected minimum energy value of measurement window of duration determined in frame_size
    #[bpaf(short, long)]
    pub(crate) predicted_minimum: Option<f64>,
    /// averaging frame size - configures duration of frame size wich is used to detect the
    /// beginning of the dataset. unit is in seconds
    #[bpaf(short, long, fallback(DEFAULT_THRESHOLD), display_fallback)]
    pub(crate) frame_size: f64,
}

#[derive(Bpaf, Debug, Clone)]
pub(crate) enum JetsonEnum {
    #[bpaf(command, adjacent)]
    Jetson(#[bpaf(external(jetson))] Jetson),
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
    /// Output Path location where all data and results are stored
    #[bpaf(short, long, fallback(PathBuf::from("./")), display_fallback)]
    pub(crate) output_path: PathBuf,
    /// store results in results.yaml file
    #[bpaf(short, long)]
    pub(crate) results_storage: bool,
    /// Settings for firmware measurements
    #[bpaf(external)]
    pub(crate) firmware_enum: Option<FirmwareEnum>,
    /// Settings for oscilloscope measurements
    #[bpaf(external)]
    pub(crate) oscilloscope_enum: Option<OscilloscopeEnum>,
    /// Settings for shelly measurements
    #[bpaf(external)]
    pub(crate) shelly_enum: Option<ShellyEnum>,
    /// Settings for jetson measurements
    #[bpaf(external)]
    pub(crate) jetson_enum: Option<JetsonEnum>,
}
