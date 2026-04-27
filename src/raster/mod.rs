//! 라스터화 백엔드 trait + 헬퍼.

pub mod pdfium;

use image::DynamicImage;

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

/// `max_edge`(긴 변 픽셀 상한)에 맞춰 비율 보존 다운스케일.
///
/// `None`이거나 이미 한도 이하면 입력을 그대로 반환. Lanczos3 필터 사용.
/// `max_edge`가 0 또는 너무 작으면 1px로 clamp (panic 방지).
#[must_use]
pub fn apply_max_edge(img: DynamicImage, max_edge: Option<u32>) -> DynamicImage {
    let Some(target_max) = max_edge else {
        return img;
    };
    let (width, height) = (img.width(), img.height());
    let longest = width.max(height);
    if longest <= target_max {
        return img;
    }
    let scale = f64::from(target_max.max(1)) / f64::from(longest);
    let target_w = scale_dim(width, scale);
    let target_h = scale_dim(height, scale);
    img.resize_exact(target_w, target_h, image::imageops::FilterType::Lanczos3)
}

fn scale_dim(value: u32, scale: f64) -> u32 {
    let scaled = (f64::from(value) * scale)
        .round()
        .clamp(1.0, f64::from(u32::MAX));
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        scaled as u32
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

    fn solid_image(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(image::RgbImage::from_pixel(w, h, image::Rgb([10, 20, 30])))
    }

    #[test]
    fn apply_max_edge_none_is_identity() {
        let img = solid_image(800, 600);
        let out = apply_max_edge(img.clone(), None);
        assert_eq!((out.width(), out.height()), (800, 600));
    }

    #[test]
    fn apply_max_edge_already_within_limit_is_identity() {
        let img = solid_image(800, 600);
        let out = apply_max_edge(img, Some(1024));
        assert_eq!((out.width(), out.height()), (800, 600));
    }

    #[test]
    fn apply_max_edge_landscape_downscales_long_edge() {
        // 2000x1500 → max_edge=1024 → 1024x768 (보장 정확)
        let img = solid_image(2000, 1500);
        let out = apply_max_edge(img, Some(1024));
        assert_eq!(out.width(), 1024);
        assert_eq!(out.height(), 768);
    }

    #[test]
    fn apply_max_edge_portrait_downscales_long_edge() {
        // 1500x2000 → max_edge=1024 → 768x1024
        let img = solid_image(1500, 2000);
        let out = apply_max_edge(img, Some(1024));
        assert_eq!(out.width(), 768);
        assert_eq!(out.height(), 1024);
    }

    #[test]
    fn apply_max_edge_zero_clamps_to_one_px() {
        let img = solid_image(100, 200);
        let out = apply_max_edge(img, Some(0));
        // 0은 1로 clamp → 0.005 비율 → 0.5/1 round → 1x1 또는 1x2
        assert!(out.width() >= 1 && out.height() >= 1);
        assert!(out.width().max(out.height()) <= 2);
    }
}
