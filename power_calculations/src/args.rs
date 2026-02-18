use bpaf::Bpaf;
use std::path::PathBuf;

#[derive(Bpaf, Debug, Clone)]
pub(crate) struct Firmware {
    /// value on which measurement beginning is triggered, unit is in watts
    #[bpaf(short, long)]
    pub(crate) beginning_trigger_value: Option<f64>,
    /// value on which measurement ending is determined, unit is in watts
    #[bpaf(short, long)]
    pub(crate) end_trigger_value: Option<f64>,
    /// averaging frame size - configures duration of frame size wich is used to detect the
    /// beginning of the dataset. unit is in seconds
    #[bpaf(short, long, fallback(1./2000.), display_fallback)]
    pub(crate) frame_size: f64,
}

#[derive(Bpaf, Debug, Clone)]
pub(crate) enum FirmwareEnum {
    #[bpaf(command, adjacent)]
    Firmware(#[bpaf(external(firmware))] Firmware),
}

#[derive(Bpaf, Debug, Clone)]
pub(crate) struct Oscilloscope {
    /// value on which measurement beginning is triggered, unit is in watts
    #[bpaf(short, long)]
    pub(crate) beginning_trigger_value: Option<f64>,
    /// value on which measurement ending is determined, unit is in watts
    #[bpaf(short, long)]
    pub(crate) end_trigger_value: Option<f64>,
    #[bpaf(short('v'), long)]
    /// use osc-voltage measurement instead of voltage estimation
    pub(crate) use_voltage: bool,
    /// oscilloscope samplerate, unit is in samples per second
    #[bpaf(short, long, fallback(5_000_000.), display_fallback)]
    pub(crate) samplerate: f64,
    /// averaging frame size - configures duration of frame size wich is used to detect the
    /// beginning of the dataset. unit is in seconds
    #[bpaf(short, long, fallback(1./2000.), display_fallback)]
    pub(crate) frame_size: f64,
}

#[derive(Bpaf, Debug, Clone)]
pub(crate) enum OscilloscopeEnum {
    #[bpaf(command, adjacent)]
    Oscilloscope(#[bpaf(external(oscilloscope))] Oscilloscope),
}

#[derive(Bpaf, Debug, Clone)]
pub(crate) struct Shelly {
    /// value on which measurement beginning is triggered, unit is in watts
    #[bpaf(short, long)]
    pub(crate) beginning_trigger_value: Option<f64>,
    /// value on which measurement ending is determined, unit is in watts
    #[bpaf(short, long)]
    pub(crate) end_trigger_value: Option<f64>,
}

#[derive(Bpaf, Debug, Clone)]
pub(crate) enum ShellyEnum {
    #[bpaf(command, adjacent)]
    Shelly(#[bpaf(external(shelly))] Shelly),
}

#[derive(Bpaf, Debug, Clone)]
pub(crate) struct Jetson {
    /// value on which measurement beginning is triggered, unit is in watts
    #[bpaf(short, long)]
    pub(crate) beginning_trigger_value: Option<f64>,
    /// value on which measurement ending is determined, unit is in watts
    #[bpaf(short, long)]
    pub(crate) end_trigger_value: Option<f64>,
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
    /// Settings for firmware measurements
    #[bpaf(external)]
    pub(crate) firmware_enum: FirmwareEnum,
    /// Settings for oscilloscope measurements
    #[bpaf(external)]
    pub(crate) oscilloscope_enum: OscilloscopeEnum,
    /// Settings for shelly measurements
    #[bpaf(external)]
    pub(crate) shelly_enum: ShellyEnum,
    /// Settings for jetson measurements
    #[bpaf(external)]
    pub(crate) jetson_enum: JetsonEnum,
}
