use crate::{decibels_to_gain, ImpulseResponseGenerator};
use ndarray::prelude::*;
use ndarray_rand::{
    rand::seq::SliceRandom, rand::thread_rng, rand_distr::Uniform, RandomExt,
};
use ndarray_stats::QuantileExt;
use std::cmp::Ordering;
use std::time::Duration;

/// Stochastic impulse response.
///
/// rt60: reverberation time [ms]
/// edt: early decay time [ms]
/// itdg: initial time delay gap [ms]
/// er_duration: early reflections duration [ms]
/// drr: direct to reverberant energy ratio [dB]
#[derive(Debug)]
pub struct ImpulseResponseImproved {
    rt60: f32,
    edt: f32,
    itdg: f32,
    er_duration: f32,
    drr: f32,
}

impl ImpulseResponseGenerator for ImpulseResponseImproved {
    /// Generate impulse response
    fn generate(&self, sample_rate: u32) -> Vec<f32> {
        let mut noise = self.get_noise(sample_rate);
        let (dsi, ersi, erei) =
            self.get_edt_and_rt60_slope(&mut noise, sample_rate);
        self.randomize_reflections(&mut noise, dsi, ersi, erei, sample_rate);
        noise.into_raw_vec()[dsi..].to_vec()
    }
}

impl ImpulseResponseImproved {
    /// Random noize (white)
    fn get_noise(&self, sample_rate: u32) -> Array1<f32> {
        let num_samples = Self::get_num_samples(
            Duration::from_millis(self.rt60.round() as u64),
            sample_rate,
        );
        Array1::random(num_samples as usize, Uniform::new(-5.0, 5.0))
    }

    fn get_edt_and_rt60_slope(
        &self,
        data: &mut Array1<f32>,
        sample_rate: u32,
    ) -> (usize, usize, usize) {
        let edt_num_samples = Self::get_num_samples(
            Duration::from_millis(self.edt.round() as u64),
            sample_rate,
        );
        let rt60_num_samples = Self::get_num_samples(
            Duration::from_millis(self.rt60.round() as u64),
            sample_rate,
        );
        let er_duration_num_samples = Self::get_num_samples(
            Duration::from_millis(self.er_duration.round() as u64),
            sample_rate,
        );

        // Shape the EDT slope of the IR
        for i in 0..(edt_num_samples - 1) as usize {
            data[i] -= i as f32;
        }
        for i in (edt_num_samples - 1) as usize..data.len() {
            data[i] -= (edt_num_samples - 1) as f32;
        }
        *data *= 10.0 / edt_num_samples as f32;

        // Shape the RT60 slope of the IR (after EDT)
        for i in edt_num_samples..rt60_num_samples {
            data[i as usize] -= (i as f32 - (edt_num_samples + 1) as f32) * 50.0
                / rt60_num_samples as f32;
        }

        // Change scale to dBFS (0 dB becomes the maximal level)
        let max_val = *data.max().unwrap_or(&0.0);
        *data -= max_val;
        data.mapv_inplace(decibels_to_gain);
        data.mapv_inplace(|x| x.powi(2));

        // Assign values to specific time points in the IR
        let direct_sound_idx = data
            .iter()
            .cloned()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        // If any of the parameters like er_duration set in config exceed the length
        // of the whole IR, then we just treat the last idx of the IR as the start/end point
        let er_start_idx = (direct_sound_idx + 1).min(data.len() - 1);
        let er_end_idx =
            (er_start_idx + er_duration_num_samples as usize).min(data.len() - 1);

        (direct_sound_idx, er_start_idx, er_end_idx)
    }

    fn randomize_reflections(
        &self,
        data: &mut Array1<f32>,
        direct_sound_idx: usize,
        early_ref_start: usize,
        early_ref_end: usize,
        sample_rate: u32,
    ) {
        self.create_initial_time_delay_gap(data, direct_sound_idx, sample_rate);

        let drr_low = self.drr - 0.5;
        let drr_high = self.drr + 0.5;

        let mut current_drr =
            Self::calculate_drr_energy_ratio(data, direct_sound_idx);

        if current_drr > drr_high {
            return;
        }

        while drr_low > current_drr {
            // Thin out early reflections
            Self::thin_out_reflections(
                data,
                early_ref_start,
                early_ref_end,
                1.0 / 8.0,
            );

            // Thin out reverberation tail
            Self::thin_out_reflections(
                data,
                early_ref_end,
                data.len() - 1,
                1.0 / 10.0,
            );

            let previous_drr = current_drr;
            current_drr = Self::calculate_drr_energy_ratio(data, direct_sound_idx);

            // If thinning out reflections did not decrease the DRR, it means
            // that the maximal DRR possible has been reached
            if (previous_drr - current_drr).abs() < std::f32::EPSILON {
                break;
            }
        }
    }

    fn create_initial_time_delay_gap(
        &self,
        data: &mut Array1<f32>,
        direct_sound_idx: usize,
        sample_rate: u32,
    ) {
        let itdg_num_samples = Self::get_num_samples(
            Duration::from_millis(self.itdg.round() as u64),
            sample_rate,
        );
        let itdg_end_idx = usize::min(
            direct_sound_idx + 1 + itdg_num_samples as usize,
            data.len() - 1,
        );
        for elem in data
            .slice_mut(s![direct_sound_idx + 1..itdg_end_idx])
            .iter_mut()
        {
            *elem = 0.0;
        }
    }

    fn calculate_drr_energy_ratio(
        data: &Array1<f32>,
        direct_sound_idx: usize,
    ) -> f32 {
        let direct = data.slice(s![..=direct_sound_idx]).sum();
        let reverberant = data.slice(s![direct_sound_idx + 1..]).sum();
        10.0 * ((direct / reverberant).log10())
    }

    fn thin_out_reflections(
        data: &mut Array1<f32>,
        start_idx: usize,
        end_idx: usize,
        rate: f32,
    ) {
        let ray_indices: Vec<usize> = (start_idx..=end_idx)
            .filter(|&idx| data[idx] != 0.0)
            .collect();
        let num_rays = ((ray_indices.len() as f32) * rate).round() as usize;

        // assert!(num_rays >= 1);
        if num_rays >= 1 {
            let mut rng = thread_rng();
            let random_subset: Vec<usize> = ray_indices
                .choose_multiple(&mut rng, num_rays)
                .cloned()
                .collect();

            for &index in random_subset.iter() {
                data[index] = 0.0;
            }
        };
    }

    fn get_num_samples(t: Duration, sample_rate: u32) -> u32 {
        (t.as_secs_f32() * sample_rate as f32).round() as u32
    }

    pub fn new(rt60: f32, edt: f32, itdg: f32, er_duration: f32, drr: f32) -> Self {
        if rt60 <= edt {
            panic!("Reverb time (rt60) can't be lower than Early decay time (edt)")
        };
        Self {
            rt60,
            edt,
            itdg,
            er_duration,
            drr,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generation_process() {
        let rir = ImpulseResponseImproved::new(500.0, 50.0, 5.0, 50.0, -1.0);
        let impulse = rir.generate(16000);
        // find non zero elements
        let mut non_zero_elements: u32 = 0;
        for sample in &impulse {
            if sample > &0.0 {
                non_zero_elements += 1;
            };
        }
        assert!(non_zero_elements > 0);
    }
}
