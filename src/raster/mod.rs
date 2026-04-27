//! 라스터화 백엔드 trait + 헬퍼.

pub mod pdfium;

/// `PDF` 1pt = 1/72 in. `DPI`를 적용해 픽셀 폭 산출.
///
/// `f64`로 계산해 `u32::MAX` 입력에서도 정밀도 손실을 회피한다 (실용 범위에서는 무관).
#[must_use]
pub fn pixels_from_points(points: f32, dpi: u32) -> u32 {
    let pixels = f64::from(points) * f64::from(dpi) / 72.0;
    let rounded = pixels.round().max(1.0);
    // 0.0..=u32::MAX 범위에서 round() 결과는 무손실로 u32에 들어간다.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        rounded.min(f64::from(u32::MAX)) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_width_at_150_dpi() {
        // A4 width: 595.28pt → 595.28 * 150 / 72 ≈ 1240
        assert_eq!(pixels_from_points(595.28, 150), 1240);
    }

    #[test]
    fn dpi_72_is_identity() {
        assert_eq!(pixels_from_points(100.0, 72), 100);
    }

    #[test]
    fn rounds_to_nearest() {
        // 100pt * 150/72 = 208.33 → 208
        assert_eq!(pixels_from_points(100.0, 150), 208);
    }

    #[test]
    fn never_zero_for_positive_input() {
        assert_eq!(pixels_from_points(0.1, 1), 1);
    }
}
