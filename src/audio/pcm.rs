use std::io::Write;
use crate::config::OutputSampleFormat;

pub fn write_samples<W: Write>(writer: &mut W, samples: &[f32], output_format: OutputSampleFormat) {
    match output_format {
        OutputSampleFormat::F32Le => {
            for sample in samples {
                let _ = writer.write_all(&sample.to_le_bytes());
            }
        }
        OutputSampleFormat::S16Le => {
            for sample in samples {
                let scaled = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16;
                let _ = writer.write_all(&scaled.to_le_bytes());
            }
        }
        OutputSampleFormat::S24Le => {
            for sample in samples {
                let scaled =
                    (sample.clamp(-1.0, 1.0) * 8_388_607.0).round() as i32;
                let bytes = scaled.to_le_bytes();
                let _ = writer.write_all(&bytes[..3]);
            }
        }
    }

    let _ = writer.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_f32_samples_as_little_endian_bytes() {
        let samples = [0.0_f32, -1.5_f32, 1.0_f32];
        let mut out = Vec::<u8>::new();

        write_samples(&mut out, &samples, OutputSampleFormat::F32Le);

        let expected: Vec<u8> = samples
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect();
        assert_eq!(out, expected);
    }

    #[test]
    fn writes_s16_samples_as_little_endian_bytes() {
        let samples = [-1.0_f32, 0.0_f32, 1.0_f32];
        let mut out = Vec::<u8>::new();

        write_samples(&mut out, &samples, OutputSampleFormat::S16Le);

        let expected: Vec<u8> = [i16::MIN + 1, 0_i16, i16::MAX]
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect();
        assert_eq!(out, expected);
    }

    #[test]
    fn writes_s24_samples_as_three_little_endian_bytes() {
        let samples = [-1.0_f32, 0.0_f32, 1.0_f32];
        let mut out = Vec::<u8>::new();

        write_samples(&mut out, &samples, OutputSampleFormat::S24Le);

        let expected = vec![0x01, 0x00, 0x80, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x7F];
        assert_eq!(out, expected);
    }

    #[test]
    fn s16le_clamps_and_rounds_half_steps() {
        let samples = [-2.0_f32, -0.5_f32, 0.5_f32, 2.0_f32];
        let mut out = Vec::<u8>::new();

        write_samples(&mut out, &samples, OutputSampleFormat::S16Le);

        let expected: Vec<u8> = [i16::MIN + 1, -16_384_i16, 16_384_i16, i16::MAX]
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect();
        assert_eq!(out, expected);
    }

    #[test]
    fn s24le_clamps_and_rounds_half_steps() {
        let samples = [-2.0_f32, -0.5_f32, 0.5_f32, 2.0_f32];
        let mut out = Vec::<u8>::new();

        write_samples(&mut out, &samples, OutputSampleFormat::S24Le);

        let expected = vec![
            0x01, 0x00, 0x80, // -1.0 (clamped)
            0x00, 0x00, 0xC0, // -0.5
            0x00, 0x00, 0x40, // 0.5
            0xFF, 0xFF, 0x7F, // 1.0 (clamped)
        ];
        assert_eq!(out, expected);
    }
}
