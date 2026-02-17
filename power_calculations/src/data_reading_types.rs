use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct JetsonMeasurement {
    /// Unit in microseconds
    pub(crate) measurement_timestamp: u128,
    /// Unit in milliamps
    pub(crate) current: u32,
    /// Unit in millivolts
    pub(crate) voltage: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct ShellyPlug {
    /// Unit in microseconds
    pub(crate) measurement_timestamp: u128,
    /// Unit in volts
    pub(crate) voltage: f64,
    /// Unit in amps
    pub(crate) current: f64,
    /// Unit in watts
    pub(crate) power: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct FirmwareMeasruement {
    #[allow(dead_code)]
    pub(crate) measurement_index: u16,
    /// Unit in amps
    pub(crate) current: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
pub(crate) struct PicoMeasurement {
    /// Unit in microseconds
    pub(crate) measurement_timestamp: u128,
    /// Sample Number in current data package
    pub(crate) sample_index: u32,
    /// Unit in volts
    pub(crate) voltage: f64,
    /// Unit in amps
    pub(crate) current: f64,
}

pub(crate) type Power = f64;
pub(crate) type Timestamp = f64;

pub(crate) enum PowerSample {
    Constant(Power),
    Variable(Timestamp, Power),
}
