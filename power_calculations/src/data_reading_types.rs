use std::collections::VecDeque;
use log::info;

pub(crate) struct JetsonMeasurement {
    /// Unit in microseconds
    pub(crate) measurement_timestamp: u64,
    /// Unit in milliamps
    pub(crate) current: u32,
    /// Unit in millivolts
    pub(crate) voltage: u32,
}

#[allow(dead_code)]
pub(crate) struct ShellyPlug {
    /// Unit in microseconds
    pub(crate) measurement_timestamp: u64,
    /// Unit in volts
    pub(crate) voltage: f64,
    /// Unit in amps
    pub(crate) current: f64,
    /// Unit in watts
    pub(crate) power: f64,
}

pub(crate) struct FirmwareMeasruement {
    #[allow(dead_code)]
    pub(crate) measurement_index: u16,
    /// Unit in amps
    pub(crate) current: u16,
}

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
    Constant(Vec<Power>),
    Variable(Vec<(Timestamp, Power)>),
}

impl PowerVec {
    pub(crate) fn len(&self) -> usize {
        match self {
            PowerVec::Constant(data) => data.len(),
            PowerVec::Variable(data) => data.len(),
        }
    }

    fn get_first(&self) -> Option<PowerSample> {
        match self {
            PowerVec::Constant(data) => data.first().map(|val| PowerSample::Constant(*val)),
            PowerVec::Variable(data) => data
                .first()
                .map(|(tstmp, val)| PowerSample::Variable(*tstmp, *val)),
        }
    }

    pub(crate) fn iter(&self, start_stop_idx: Option<(usize, usize)>) -> PowerIter<'_> {
        PowerIter::new(self, start_stop_idx)
    }

    pub(crate) fn power_window_iter(
        &self,
        frame_size: f64,
        samplerate_opt: Option<f64>,
    ) -> WindowEnergyIter<'_> {
        let duration = match self {
            Self::Constant(data) => data.len() as f64 * (1.0 / samplerate_opt.unwrap()),
            Self::Variable(data) => {
                let (start, _) = *data.first().unwrap();
                let (end, _) = *data.last().unwrap();
                end - start
            }
        };
        WindowEnergyIter::new(self, frame_size, samplerate_opt, duration)
    }

    pub(crate) fn duration(&self, start_stop_idx: Option<(usize, usize)>, samplerate_opt: Option<f64>) -> f64 {
        match self {
            Self::Constant(data) => {
                let samplerate = samplerate_opt.unwrap();
                if let Some((start, stop)) = start_stop_idx {
                    ((stop - start) + 1) as f64 / samplerate
                } else {
                    data.len() as f64 / samplerate
                }
            },
            Self::Variable(data) => {
                if let Some((start, stop)) = start_stop_idx {
                    data[stop].0 - data[start].0
                } else {
                    data[data.len() - 1].0 - data[0].0
                }
            }
        }
    }

    pub(crate) fn fit_start_stop_to_duration(&self, initial_start_idx: usize, initial_stop_idx: usize, duration: f64, samplerate_opt: Option<f64>) -> (usize, usize) {
        let actual_duration = self.duration(Some((initial_start_idx, initial_stop_idx)), samplerate_opt);
        info!("Duration {actual_duration}");
        let duration_diff = duration - actual_duration;
        if duration_diff.abs() > duration * 0.1 {
            info!("Stopping fitting, duration deviation is too big");
            // if the difference between start and end is too big - this is done so external programs can detect that the measurement is not valid
            return (initial_start_idx, initial_stop_idx);
        }
        match self {
            PowerVec::Constant(data) => {
                let samplerate = samplerate_opt.unwrap();
                let sample_offset = ((duration_diff / 2.) * samplerate).round() as i64;
                let start_idx = if initial_start_idx as i64 - sample_offset < 0 {
                    0
                } else {
                    initial_start_idx as i64 - sample_offset
                };
                let stop_idx = if sample_offset + initial_stop_idx as i64 >= data.len() as i64 {
                    (data.len() - 1) as i64
                } else {
                    initial_stop_idx as i64 + sample_offset
                };
                (start_idx as usize, stop_idx as usize)
            }
            PowerVec::Variable(data) => {
                fn find_timestamp_from_pos(data: &[(f64, f64)], start_idx: usize, stop_timestamp: Timestamp) -> usize {
                    let (mut current_timestamp, _) = data[start_idx];
                    let mut current_idx = start_idx;
                    if current_timestamp < stop_timestamp {
                        while current_timestamp <= stop_timestamp {
                            if current_idx == data.len() -1 {
                               break;
                            }
                            current_idx += 1;
                            (current_timestamp, _) = data[current_idx];
                        }
                    } else {
                        while current_timestamp >= stop_timestamp {
                            if current_idx == 0 {
                                break;
                            }
                            current_idx -= 1;
                            (current_timestamp, _) = data[current_idx];
                        }
                    }
                    current_idx
                }

                let (start_timestamp, _) = data[initial_start_idx];
                let (end_timestamp, _) = data[initial_stop_idx];
                let start_idx = find_timestamp_from_pos(
                    data, initial_start_idx, start_timestamp - duration_diff / 2.);
                let stop_idx = find_timestamp_from_pos(
                    data, initial_stop_idx, end_timestamp + duration_diff / 2.);
                (start_idx, stop_idx)
            }
        }
    }

    pub(crate) fn cut_data(self, start_idx: usize, stop_idx: usize) -> Self {
        match self {
            Self::Constant(mut data) => {
                if stop_idx < data.len() - 2 {
                    data.drain(stop_idx + 1..data.len());
                }
                if start_idx > 0 {
                    data.drain(0..start_idx);
                }
                PowerVec::Constant(data)
            },
            Self::Variable(mut data) => {
                if stop_idx < data.len() - 2 {
                    data.drain(stop_idx + 1..data.len());
                }
                if start_idx > 0 {
                    data.drain(0..start_idx);
                }
                PowerVec::Variable(data)
            }
        }
    }
}

pub(crate) struct PowerIter<'a> {
    const_iter: Option<core::slice::Iter<'a, f64>>,
    var_iter: Option<core::slice::Iter<'a, (f64, f64)>>,
    iter_count: usize,
}

impl<'a> PowerIter<'a> {
    fn new(data: &'a PowerVec, start_stop_idx: Option<(usize, usize)>) -> Self {
        let (start, stop) = if let Some(start_stop_idx) = start_stop_idx {
            start_stop_idx
        } else {
            (0, data.len()-1)
        };
        let const_iter = if let PowerVec::Constant(raw_data) = data {
            Some(raw_data[start..=stop].iter())
        } else {
            None
        };
        let var_iter = if let PowerVec::Variable(raw_data) = data {
            Some(raw_data[start..=stop].iter())
        } else {
            None
        };
        Self {
            const_iter,
            var_iter,
            iter_count: 0,
        }
    }

    fn iter_count(&self) -> usize {
        self.iter_count
    }
}

impl<'a> Iterator for PowerIter<'a> {
    type Item = PowerSample;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter_count += 1;
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
        self.iter_count += 1;
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

impl<'a> ExactSizeIterator for PowerIter<'a> {
    fn len(&self) -> usize {
        if let Some(const_iter) = self.const_iter.as_ref() {
            return const_iter.len();
        }
        if let Some(var_iter) = self.var_iter.as_ref() {
            return var_iter.len();
        }
        0
    }
}

#[derive(Clone)]
pub(crate) enum PowerSample {
    Constant(Power),
    Variable(Timestamp, Power),
}

pub(crate) struct WindowEnergyIter<'a> {
    data: PowerIter<'a>,
    frame_size: f64,
    samplerate: f64,
    overshoot: f64,
    overshoot_time: f64,
    last_sample: Option<PowerSample>,
    frame_queue: VecDeque<(usize, f64)>,
    duration: f64,
}

impl<'a> WindowEnergyIter<'a> {
    pub(crate) fn new(
        data: &'a PowerVec,
        frame_size: f64,
        samplerate_opt: Option<f64>,
        duration: f64,
    ) -> Self {
        if let Some(PowerSample::Constant(_)) = data.get_first()
            && let None = samplerate_opt
        {
            unreachable!();
        }
        Self {
            data: data.iter(None),
            frame_size,
            samplerate: samplerate_opt.unwrap_or(0.0),
            overshoot: 0.0,
            overshoot_time: 0.0,
            last_sample: None,
            frame_queue: VecDeque::new(),
            duration,
        }
    }

    pub(crate) fn max_and_idle(self) -> (f64, f64) {
        let mut current_max = 0.;
        let mut idle_start = 0.;
        let mut idle_end = 0.;

        let frame_size = self.frame_size;
        let idle_frames = 5.0 / frame_size;
        let duration = self.duration;

        for (idx, (_, frame_power)) in self.enumerate() {
            if idx as f64 * frame_size < 5.0 {
                idle_start += frame_power;
            } else if idx as f64 * frame_size > duration - 5.0 {
                idle_end += frame_power;
            }
            if frame_power > current_max {
                current_max = frame_power;
            }
        }

        (
            current_max,
            (idle_start / idle_frames).max(idle_end / idle_frames),
        )
    }

    #[allow(dead_code)]
    pub(crate) fn mad(self) -> f64 {
        fn median(data: &mut [f64]) -> f64 {
            data.sort_unstable_by(|a, b| a.total_cmp(b));
            if data.len().is_multiple_of(2) {
                let left_idx = data.len() / 2 - 1;
                let right_idx = data.len() / 2;
                (data[left_idx] + data[right_idx]) / 2.0
            } else {
                data[data.len() / 2]
            }
        }
        let mut samples: Vec<f64> = self.map(|(_, val)| val).collect();
        let med = median(samples.as_mut_slice());
        samples.iter_mut().for_each(|val| {
            *val = (*val - med).abs();
        });
        median(samples.as_mut_slice())
    }

    fn calc_frame(&mut self, reverse: bool) -> Option<(usize, f64)> {
        if !self.frame_queue.is_empty() {
            return self.frame_queue.pop_back();
        }

        let mut frame_pos = self.overshoot_time;
        let mut last_power;
        let mut last_time = 0.0;

        if self.last_sample.is_none() {
            self.last_sample = if reverse {
                self.data.next_back()
            } else {
                self.data.next()
            };
        }
        if let Some(fst_sample) = &self.last_sample {
            match fst_sample {
                PowerSample::Constant(power) => {
                    last_power = *power;
                }
                PowerSample::Variable(time, power) => {
                    last_time = *time;
                    last_power = *power;
                }
            }
        } else {
            return None;
        }

        let mut energy = self.overshoot;

        while frame_pos < self.frame_size {
            let next_sample_opt = if reverse {
                self.data.next_back()
            } else {
                self.data.next()
            };
            self.last_sample = next_sample_opt.clone();
            if let Some(next_sample) = next_sample_opt {
                let (current_power, time_diff) = match next_sample {
                    PowerSample::Constant(power) => (power, 1. / self.samplerate),
                    PowerSample::Variable(time, power) => {
                        let diff = (time - last_time).abs();
                        last_time = time;
                        (power, diff)
                    }
                };
                let current_energy = ((current_power + last_power) / 2.) * time_diff;
                last_power = current_power;
                if frame_pos + time_diff > self.frame_size {
                    let time_to_fill_frame = self.frame_size - frame_pos;
                    let nrgy_to_fill_frame = current_energy * (time_to_fill_frame / time_diff);
                    self.frame_queue
                        .push_front((self.data.iter_count(), nrgy_to_fill_frame + energy));
                    let mut rem_time_diff = time_diff - time_to_fill_frame;
                    while rem_time_diff >= self.frame_size {
                        self.frame_queue.push_front((
                            self.data.iter_count(),
                            current_energy * (self.frame_size / time_diff),
                        ));
                        rem_time_diff -= self.frame_size;
                    }
                    self.overshoot = current_energy * (rem_time_diff / time_diff);
                    self.overshoot_time = rem_time_diff;
                    return self.frame_queue.pop_back();
                } else {
                    energy += current_energy;
                    self.overshoot = 0.0;
                }
                frame_pos += time_diff;
            } else {
                return Some((self.data.iter_count(), energy));
            }
        }

        Some((self.data.iter_count(), energy))
    }
}

impl<'a> Iterator for WindowEnergyIter<'a> {
    type Item = (usize, f64);

    fn next(&mut self) -> Option<Self::Item> {
        self.calc_frame(false)
    }
}

impl<'a> DoubleEndedIterator for WindowEnergyIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.calc_frame(true)
    }
}
