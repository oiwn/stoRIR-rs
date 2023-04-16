/// Convert units from decibels to gain
pub fn decibels_to_gain(decibels: f32) -> f32 {
    10.0_f32.powf(decibels / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decibels_to_gain() {
        let input_values = vec![
            (-60.0, 0.001),
            (-20.0, 0.1),
            (0.0, 1.0),
            (20.0, 10.0),
            (60.0, 1000.0),
        ];

        for (input, expected) in input_values {
            let result = decibels_to_gain(input);
            assert!(
                (result - expected).abs() < f32::EPSILON,
                "Input: {}, Expected: {}, Result: {}",
                input,
                expected,
                result
            );
        }
    }
}
