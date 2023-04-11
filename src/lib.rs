use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

use std::time::Duration;

// Stochastic impulse response.
//
// rt60: reverberation time [ms]
// edt: early decay time [ms]
// itdg: initial time delay gap [ms]
// er_duration: early reflections duration [ms]
// drr: direct to reverberant energy ratio [dB]
#[derive(Debug)]
pub struct ImpulseResponse {
    rt60: f32,
    edt: f32,
    itdg: f32,
    er_duration: f32,
    drr: f32,
}

impl ImpulseResponse {
    pub fn new(rt60: f32, edt: f32, itdg: f32, er_duration: f32, drr: f32) -> Self {
        Self {
            rt60,
            edt,
            itdg,
            er_duration,
            drr,
        }
    }

    pub fn generate(&self, sample_rate: u32) -> Vec<f32> {
        let mut noise = self.get_noise(sample_rate);
        let (dsi, ersi, erei) =
            self.get_edt_and_rt60_slope(&mut noise, sample_rate);
        self.randomize_reflections(&mut noise, dsi, ersi, erei, sample_rate);
        noise[dsi..].to_vec()
    }

    fn get_edt_and_rt60_slope(
        &self,
        data: &mut Vec<f32>,
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

        for i in 0..(edt_num_samples - 1) as usize {
            data[i] -= i as f32;
        }
        for i in (edt_num_samples - 1) as usize..data.len() {
            data[i] -= (edt_num_samples - 1) as f32;
        }
        for value in data.iter_mut() {
            *value *= 10.0 / edt_num_samples as f32;
        }

        // Shape the RT60 slope of the IR (after EDT)
        for i in edt_num_samples..rt60_num_samples {
            // Something like this (2205 - (2205 + 1)) * 50 / 22050
            data[i as usize] -= (i as f32 - (edt_num_samples as f32 + 1.0)) * 50.0
                / rt60_num_samples as f32;
        }

        let max_y = data.iter().cloned().fold(f32::MIN, f32::max);
        for value in data.iter_mut() {
            *value -= max_y;
            let gain = 10_f32.powf(*value / 20.0); // decibels to gain
            *value = gain * gain;
        }

        // Assign values to specific time points in the IR
        let direct_sound_idx = data
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);

        let er_start_idx = std::cmp::min(direct_sound_idx + 1, data.len() - 1);
        let er_end_idx = std::cmp::min(
            er_start_idx + er_duration_num_samples as usize,
            data.len() - 1,
        );
        (direct_sound_idx, er_start_idx, er_end_idx)
    }

    fn randomize_reflections(
        &self,
        data: &mut Vec<f32>,
        direct_sound_idx: usize,
        early_ref_start: usize,
        early_ref_end: usize,
        sampling_rate: u32,
    ) {
        self.create_initial_time_delay_gap(data, direct_sound_idx, sampling_rate);

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

    /// Random noize (white)
    fn get_noise(&self, sample_rate: u32) -> Vec<f32> {
        let num_samples = Self::get_num_samples(
            Duration::from_millis(self.rt60.round() as u64),
            sample_rate,
        );
        let mut rng = rand::thread_rng();
        (0..num_samples).map(|_| rng.gen_range(-5.0..5.0)).collect()
    }

    fn create_initial_time_delay_gap(
        &self,
        data: &mut Vec<f32>,
        direct_sound_idx: usize,
        sampling_rate: u32,
    ) {
        let itdg_num_samples = (self.itdg * sampling_rate as f32).round() as usize;
        let itdg_end_idx =
            std::cmp::min(direct_sound_idx + 1 + itdg_num_samples, data.len() - 1);
        for value in data
            .iter_mut()
            .take(itdg_end_idx)
            .skip(direct_sound_idx + 1)
        {
            *value = 0.0;
        }
    }

    fn calculate_drr_energy_ratio(data: &[f32], direct_sound_idx: usize) -> f32 {
        let direct = data.iter().take(direct_sound_idx + 1).sum::<f32>();
        let reverberant = data.iter().skip(direct_sound_idx + 1).sum::<f32>();
        let drr = 10.0 * (direct / reverberant).log10();
        drr
    }

    fn thin_out_reflections(
        data: &mut Vec<f32>,
        start_idx: usize,
        end_idx: usize,
        rate: f32,
    ) {
        let ray_indices: Vec<usize> = (start_idx..=end_idx)
            .filter(|&idx| data[idx] != 0.0)
            .collect();
        let num_rays = (ray_indices.len() as f32 * rate).round() as usize;
        assert!(num_rays >= 1);

        let mut rng = thread_rng();
        let random_subset: Vec<usize> = ray_indices
            .choose_multiple(&mut rng, num_rays)
            .cloned()
            .collect();

        for index in random_subset {
            data[index] = 0.0;
        }
    }

    fn get_num_samples(t: Duration, sample_rate: u32) -> u32 {
        (t.as_secs_f32() * sample_rate as f32).round() as u32
    }
}
