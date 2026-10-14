//! One premultiplied BGRA surface for the entire dimmed background.
use super::{WindowBackdropCutout, WindowBackdropLayer};
use windows::Win32::Foundation::RECT;

fn blend(pixel: u32, layer: &WindowBackdropLayer) -> u32 {
    let alpha = u32::from(layer.alpha);
    let inverse = 255 - alpha;
    let channel = |value: u8, shift: u32| {
        (u32::from(value) * alpha + 127) / 255 + (((pixel >> shift) & 255) * inverse + 127) / 255
    };
    channel(layer.blue, 0)
        | (channel(layer.green, 8) << 8)
        | (channel(layer.red, 16) << 16)
        | ((alpha + ((pixel >> 24) * inverse + 127) / 255) << 24)
}

fn cutout_span(
    r: &WindowBackdropCutout,
    row: usize,
    scale: f64,
    width: usize,
) -> Option<(usize, usize)> {
    let top = r.y * scale;
    let height = r.height * scale;
    let y = row as f64 + 0.5;
    if height <= 0.0 || y < top || y >= top + height {
        return None;
    }
    let radius = (r.radius * scale)
        .min(r.width * scale / 2.0)
        .min(height / 2.0);
    let dy = (top + radius - y).max(y - (top + height - radius)).max(0.0);
    let inset = if radius > 0.0 {
        radius - (radius * radius - dy * dy).max(0.0).sqrt()
    } else {
        0.0
    };
    let left = (r.x * scale + inset - 0.5).ceil().clamp(0.0, width as f64) as usize;
    let right = ((r.x + r.width) * scale - inset - 0.5)
        .ceil()
        .clamp(0.0, width as f64) as usize;
    (right > left).then_some((left, right))
}

fn paint_span(
    row: &mut [u32],
    from: usize,
    to: usize,
    cuts: &[(usize, usize)],
    layer: &WindowBackdropLayer,
) {
    let mut start = from;
    for &(left, right) in cuts {
        if right <= start || left >= to {
            continue;
        }
        for pixel in &mut row[start..left.min(to).max(start)] {
            *pixel = blend(*pixel, layer);
        }
        start = start.max(right.min(to));
    }
    for pixel in &mut row[start..to] {
        *pixel = blend(*pixel, layer);
    }
}

pub(super) fn compose(
    width: usize,
    height: usize,
    scale: f64,
    layers: &[WindowBackdropLayer],
    caption: &[RECT],
) -> Result<Vec<u32>, String> {
    let count = width.checked_mul(height).ok_or("Backdrop size overflow")?;
    if width == 0 || height == 0 || width > 16_384 || height > 16_384 || count > 32_000_000 {
        return Err("Backdrop size outside supported bounds".into());
    }
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(count)
        .map_err(|_| "Cannot allocate backdrop pixels")?;
    pixels.resize(count, 0);
    for layer in layers.iter().filter(|l| l.alpha > 0) {
        for y in 0..height {
            let mut cuts: Vec<_> = layer
                .exclusions
                .iter()
                .filter_map(|r| cutout_span(r, y, scale, width))
                .collect();
            cuts.sort_unstable();
            let row = &mut pixels[y * width..(y + 1) * width];
            if layer.full_window {
                paint_span(row, 0, width, &cuts, layer);
            } else {
                for r in caption
                    .iter()
                    .filter(|r| y as i32 >= r.top && (y as i32) < r.bottom)
                {
                    let left = r.left.clamp(0, width as i32) as usize;
                    let right = r.right.clamp(0, width as i32) as usize;
                    if right > left {
                        paint_span(row, left, right, &cuts, layer);
                    }
                }
            }
        }
    }
    Ok(pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn layer(
        alpha: u8,
        full_window: bool,
        exclusions: Vec<WindowBackdropCutout>,
    ) -> WindowBackdropLayer {
        WindowBackdropLayer {
            red: 31,
            green: 28,
            blue: 24,
            alpha,
            full_window,
            exclusions,
        }
    }
    fn cut(x: f64, y: f64, width: f64, height: f64, radius: f64) -> WindowBackdropCutout {
        WindowBackdropCutout {
            x,
            y,
            width,
            height,
            radius,
        }
    }
    #[test]
    fn header_and_background_share_one_surface_while_rounded_card_remains_clear() {
        let p = compose(
            10,
            10,
            1.0,
            &[layer(102, true, vec![cut(2.0, 2.0, 6.0, 6.0, 2.0)])],
            &[],
        )
        .unwrap();
        assert_eq!(p[0], p[99]);
        assert_eq!(p[0] >> 24, 102);
        assert_eq!(p[44], 0);
        assert_eq!(p[22] >> 24, 102); // Rounded corner is background, not a rectangular cut.
    }
    #[test]
    fn nested_mask_preserves_background_parent_card_and_front_card_depth() {
        let front = cut(4.0, 3.0, 2.0, 2.0, 0.0);
        let p = compose(
            10,
            10,
            1.0,
            &[
                layer(102, true, vec![cut(2.0, 2.0, 6.0, 6.0, 0.0), front.clone()]),
                layer(166, true, vec![front]),
            ],
            &[],
        )
        .unwrap();
        assert_eq!(p[0] >> 24, 202);
        assert_eq!(p[22] >> 24, 166);
        assert_eq!(p[34], 0);
    }
    #[test]
    fn legacy_dom_masks_only_shade_the_actual_native_exclusion() {
        let p = compose(
            10,
            10,
            1.0,
            &[layer(102, false, vec![])],
            &[
                RECT {
                    left: 0,
                    top: 0,
                    right: 10,
                    bottom: 2,
                },
                RECT {
                    left: 8,
                    top: 2,
                    right: 10,
                    bottom: 4,
                },
            ],
        )
        .unwrap();
        assert_eq!(p[0] >> 24, 102);
        assert_eq!(p[29] >> 24, 102);
        assert_eq!(p[22], 0);
        assert_eq!(p[99], 0);
    }
    #[test]
    fn oversize_surfaces_are_rejected_before_allocation() {
        assert!(compose(16_384, 16_384, 1.0, &[], &[]).is_err());
        assert!(compose(0, 10, 1.0, &[], &[]).is_err());
    }
}
