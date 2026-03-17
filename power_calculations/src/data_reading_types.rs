use std::collections::{VecDeque, vec_deque::Iter};

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
    /// Unit in volts
    pub(crate) voltage: f64,
    /// Unit in amps
    pub(crate) current: f64,
}

pub(crate) type Power = f64;
pub(crate) type Timestamp = f64;

pub(crate) enum PowerVec {
    Constant(VecDeque<Power>),
    Variable(VecDeque<(Timestamp, Power)>),
}

impl PowerVec {
    pub(crate) fn len(&self) -> usize {
        match self {
            PowerVec::Constant(data) => data.len(),
            PowerVec::Variable(data) => data.len(),
        }
    }

    pub(crate) fn pop_front(&mut self) -> Option<PowerSample> {
        match self {
            PowerVec::Constant(data) => data.pop_front().map(PowerSample::Constant),
            PowerVec::Variable(data) => data
                .pop_front()
                .map(|(tstmp, val)| PowerSample::Variable(tstmp, val)),
        }
    }

    pub(crate) fn pop_back(&mut self) -> Option<PowerSample> {
        match self {
            PowerVec::Constant(data) => data.pop_back().map(PowerSample::Constant),
            PowerVec::Variable(data) => data
                .pop_back()
                .map(|(tstmp, val)| PowerSample::Variable(tstmp, val)),
        }
    }

    pub(crate) fn iter<'a>(&'a self) -> PowerIter<'a> {
        PowerIter::new(self)
    }
}

pub(crate) struct PowerIter<'a> {
    const_iter: Option<Iter<'a, f64>>,
    var_iter: Option<Iter<'a, (f64, f64)>>,
}

impl<'a> PowerIter<'a> {
    fn new(data: &'a PowerVec) -> Self {
        let const_iter = if let PowerVec::Constant(raw_data) = data {
            Some(raw_data.iter())
        } else {
            None
        };
        let var_iter = if let PowerVec::Variable(raw_data) = data {
            Some(raw_data.iter())
        } else {
            None
        };
        Self {
            const_iter,
            var_iter,
        }
    }
}

impl<'a> Iterator for PowerIter<'a> {
    type Item = PowerSample;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(const_iter) = self.const_iter.as_mut() {
            return const_iter.next().map(|elem| Self::Item::Constant(*elem));
        }
        if let Some(var_iter) = self.var_iter.as_mut() {
            return var_iter
                .next()
                .map(|(tstmp, elem)| Self::Item::Variable(*tstmp, *elem));
        }
        None
    }
}

impl<'a> DoubleEndedIterator for PowerIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(const_iter) = self.const_iter.as_mut() {
            return const_iter
                .next_back()
                .map(|elem| Self::Item::Constant(*elem));
        }
        if let Some(var_iter) = self.var_iter.as_mut() {
            return var_iter
                .next_back()
                .map(|(tstmp, elem)| Self::Item::Variable(*tstmp, *elem));
        }
        None
    }
}

pub(crate) enum PowerSample {
    Constant(Power),
    Variable(Timestamp, Power),
}
