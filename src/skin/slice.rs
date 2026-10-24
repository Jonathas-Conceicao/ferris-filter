//! Nine-slice and three-slice painting.
//!
//! The geometry is pure (unit tested); the painters turn it into one textured mesh. Every cut
//! line in screen space is snapped to the physical pixel grid, so adjacent pieces never leave a
//! hairline gap or overlap, and the outer UV edges are inset by half a texel so bilinear
//! filtering never samples outside the texture.

use egui::{Color32, Mesh, Painter, Pos2, Rect, Shape, TextureHandle, pos2};

/// Snaps a coordinate (in points) to the nearest physical pixel.
pub fn snap(v: f32, pixels_per_point: f32) -> f32 {
    (v * pixels_per_point).round() / pixels_per_point
}

/// Snaps every edge of `rect` to the physical pixel grid.
pub fn snap_rect(rect: Rect, ppp: f32) -> Rect {
    Rect::from_min_max(
        pos2(snap(rect.min.x, ppp), snap(rect.min.y, ppp)),
        pos2(snap(rect.max.x, ppp), snap(rect.max.y, ppp)),
    )
}

/// Margins of a sliced texture, in texels: left, top, right, bottom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margins {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Margins {
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
}

/// One piece of a sliced draw: where it lands and which part of the texture it shows (UVs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piece {
    pub dest: Rect,
    pub uv: Rect,
    /// The centre cell (index 4 of a nine-slice).
    pub center: bool,
}

/// Cut positions along one axis: `[start, start+a, end-b, end]`, with the two margins shrunk
/// proportionally when they do not fit, then snapped.
fn cuts(start: f32, end: f32, a: f32, b: f32, ppp: f32) -> [f32; 4] {
    let len = (end - start).max(0.0);
    let k = if a + b > len && a + b > 0.0 {
        len / (a + b)
    } else {
        1.0
    };
    [
        snap(start, ppp),
        snap(start + a * k, ppp),
        snap(end - b * k, ppp),
        snap(end, ppp),
    ]
}

/// UV cut positions along one axis for a texture `size` texels long with margins `a` and `b`,
/// the two outer edges inset by half a texel.
fn uv_cuts(size: f32, a: f32, b: f32) -> [f32; 4] {
    let half = 0.5 / size;
    [half, a / size, (size - b) / size, 1.0 - half]
}

/// The nine pieces of a nine-slice draw of a `tex_size` texture into `dest`. Margins are in
/// texels and drawn at `scale` points per texel. Zero-sized pieces are dropped.
pub fn nine_slice(
    dest: Rect,
    tex_size: [usize; 2],
    m: Margins,
    scale: f32,
    ppp: f32,
) -> Vec<Piece> {
    let (tw, th) = (tex_size[0] as f32, tex_size[1] as f32);
    let xs = cuts(dest.min.x, dest.max.x, m.left * scale, m.right * scale, ppp);
    let ys = cuts(dest.min.y, dest.max.y, m.top * scale, m.bottom * scale, ppp);
    let us = uv_cuts(tw, m.left, m.right);
    let vs = uv_cuts(th, m.top, m.bottom);
    let mut out = Vec::with_capacity(9);
    for row in 0..3 {
        for col in 0..3 {
            let d = Rect::from_min_max(pos2(xs[col], ys[row]), pos2(xs[col + 1], ys[row + 1]));
            if d.width() <= 0.0 || d.height() <= 0.0 {
                continue;
            }
            let uv = Rect::from_min_max(pos2(us[col], vs[row]), pos2(us[col + 1], vs[row + 1]));
            out.push(Piece {
                dest: d,
                uv,
                center: row == 1 && col == 1,
            });
        }
    }
    out
}

/// Paints a nine-slice. `draw_center = false` leaves the middle untouched (the Options frame's
/// middle is transparent anyway; skipping it saves overdraw).
pub fn paint_nine(
    painter: &Painter,
    tex: &TextureHandle,
    dest: Rect,
    m: Margins,
    scale: f32,
    tint: Color32,
    draw_center: bool,
) {
    let ppp = painter.pixels_per_point();
    let mut mesh = Mesh::with_texture(tex.id());
    for p in nine_slice(dest, tex.size(), m, scale, ppp) {
        if p.center && !draw_center {
            continue;
        }
        mesh.add_rect_with_uv(p.dest, p.uv, tint);
    }
    painter.add(Shape::mesh(mesh));
}

/// Horizontal three-slice: the caps keep their aspect ratio at `dest`'s height, the middle
/// stretches. Each piece is its own texture (as the game ships them).
pub fn three_h_rects(dest: Rect, left: [usize; 2], right: [usize; 2], ppp: f32) -> [Rect; 3] {
    let h = dest.height();
    let lw = left[0] as f32 * h / left[1].max(1) as f32;
    let rw = right[0] as f32 * h / right[1].max(1) as f32;
    let xs = cuts(dest.min.x, dest.max.x, lw, rw, ppp);
    let (y0, y1) = (snap(dest.min.y, ppp), snap(dest.max.y, ppp));
    [
        Rect::from_min_max(pos2(xs[0], y0), pos2(xs[1], y1)),
        Rect::from_min_max(pos2(xs[1], y0), pos2(xs[2], y1)),
        Rect::from_min_max(pos2(xs[2], y0), pos2(xs[3], y1)),
    ]
}

/// Vertical three-slice: caps keep their aspect at `dest`'s width, the middle stretches.
pub fn three_v_rects(dest: Rect, top: [usize; 2], bottom: [usize; 2], ppp: f32) -> [Rect; 3] {
    let w = dest.width();
    let th = top[1] as f32 * w / top[0].max(1) as f32;
    let bh = bottom[1] as f32 * w / bottom[0].max(1) as f32;
    let ys = cuts(dest.min.y, dest.max.y, th, bh, ppp);
    let (x0, x1) = (snap(dest.min.x, ppp), snap(dest.max.x, ppp));
    [
        Rect::from_min_max(pos2(x0, ys[0]), pos2(x1, ys[1])),
        Rect::from_min_max(pos2(x0, ys[1]), pos2(x1, ys[2])),
        Rect::from_min_max(pos2(x0, ys[2]), pos2(x1, ys[3])),
    ]
}

/// UVs covering a whole texture of `size`, inset by half a texel.
pub fn full_uv(size: [usize; 2]) -> Rect {
    let (hx, hy) = (0.5 / size[0].max(1) as f32, 0.5 / size[1].max(1) as f32);
    Rect::from_min_max(pos2(hx, hy), pos2(1.0 - hx, 1.0 - hy))
}

/// Paints three textures into three rects (from [`three_h_rects`] / [`three_v_rects`]).
pub fn paint_three(painter: &Painter, tex: [&TextureHandle; 3], rects: [Rect; 3], tint: Color32) {
    for (t, r) in tex.into_iter().zip(rects) {
        if r.width() > 0.0 && r.height() > 0.0 {
            painter.image(t.id(), r, full_uv(t.size()), tint);
        }
    }
}

/// Paints a whole texture into `dest` (pixel-snapped).
pub fn paint_image(painter: &Painter, tex: &TextureHandle, dest: Rect, tint: Color32) {
    let r = snap_rect(dest, painter.pixels_per_point());
    painter.image(tex.id(), r, full_uv(tex.size()), tint);
}

/// UVs of the texel box `[x0, y0, x1, y1)` of a texture of `size`, inset half a texel on each
/// side (so bilinear filtering never samples outside the box). `size` is clamped to at least 1.
pub fn box_uv(size: [usize; 2], b: [u32; 4]) -> Rect {
    let (w, h) = (size[0].max(1) as f32, size[1].max(1) as f32);
    Rect::from_min_max(
        pos2((b[0] as f32 + 0.5) / w, (b[1] as f32 + 0.5) / h),
        pos2((b[2] as f32 - 0.5) / w, (b[3] as f32 - 0.5) / h),
    )
}

/// Paints only the opaque box `opaque` (`[x0, y0, x1, y1)` in texels) of `tex`, stretched to fill
/// `dest` (pixel-snapped): the visible plate fills the rect, so art with a transparent gutter is
/// positioned by what the eye sees, never by the texture rect.
pub fn paint_visible(
    painter: &Painter,
    tex: &TextureHandle,
    opaque: [u32; 4],
    dest: Rect,
    tint: Color32,
) {
    let r = snap_rect(dest, painter.pixels_per_point());
    painter.image(tex.id(), r, box_uv(tex.size(), opaque), tint);
}

/// A rect of `size` centred on `center`.
pub fn centered(center: Pos2, size: egui::Vec2) -> Rect {
    Rect::from_center_size(center, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(pos2(x0, y0), pos2(x1, y1))
    }

    #[test]
    fn box_uv_insets_half_a_texel() {
        let uv = box_uv([92, 92], [0, 0, 84, 84]);
        assert_eq!(uv.min, pos2(0.5 / 92.0, 0.5 / 92.0));
        assert_eq!(uv.max, pos2(83.5 / 92.0, 83.5 / 92.0));
    }

    #[test]
    fn box_uv_of_the_whole_texture_is_full_uv() {
        let uv = box_uv([54, 56], [0, 0, 54, 54]);
        assert_eq!(uv.min, full_uv([54, 56]).min);
        assert_eq!(uv.max.y, 53.5 / 56.0);
        assert_eq!(uv.max.x, full_uv([54, 56]).max.x);
    }

    #[test]
    fn box_uv_honours_a_non_zero_origin() {
        let uv = box_uv([76, 88], [4, 5, 58, 69]);
        assert_eq!(uv.min, pos2(4.5 / 76.0, 5.5 / 88.0));
    }

    #[test]
    fn nine_slice_tiles_the_destination_without_gaps() {
        let dest = r(10.0, 20.0, 310.0, 520.0);
        let pieces = nine_slice(
            dest,
            [100, 100],
            Margins::new(20.0, 30.0, 20.0, 10.0),
            0.5,
            1.0,
        );
        assert_eq!(pieces.len(), 9);
        let area: f32 = pieces.iter().map(|p| p.dest.area()).sum();
        assert!(
            (area - dest.area()).abs() < 0.01,
            "pieces must cover dest exactly"
        );
        // Corners keep margin * scale; the centre stretches.
        assert_eq!(pieces[0].dest, r(10.0, 20.0, 20.0, 35.0));
        assert_eq!(pieces[8].dest, r(300.0, 515.0, 310.0, 520.0));
        assert!(pieces[4].center);
    }

    #[test]
    fn nine_slice_uvs_inset_outer_edges_by_half_a_texel() {
        let pieces = nine_slice(
            r(0.0, 0.0, 100.0, 100.0),
            [200, 100],
            Margins::new(10.0, 10.0, 10.0, 10.0),
            1.0,
            1.0,
        );
        assert_eq!(pieces[0].uv.min, pos2(0.5 / 200.0, 0.5 / 100.0));
        assert_eq!(pieces[0].uv.max, pos2(10.0 / 200.0, 10.0 / 100.0));
        assert_eq!(pieces[8].uv.max, pos2(1.0 - 0.5 / 200.0, 1.0 - 0.5 / 100.0));
    }

    #[test]
    fn nine_slice_cuts_land_on_physical_pixels() {
        // At 1.25x a margin of 7 texels * 0.5 = 3.5pt = 4.375px must be snapped to whole pixels.
        let ppp = 1.25;
        let pieces = nine_slice(
            r(0.3, 0.7, 200.1, 90.9),
            [64, 64],
            Margins::new(7.0, 7.0, 7.0, 7.0),
            0.5,
            ppp,
        );
        for p in pieces {
            for v in [p.dest.min.x, p.dest.min.y, p.dest.max.x, p.dest.max.y] {
                let px = v * ppp;
                assert!((px - px.round()).abs() < 1e-3, "{v} is not on a pixel edge");
            }
        }
    }

    #[test]
    fn margins_shrink_when_the_destination_is_too_small() {
        let pieces = nine_slice(
            r(0.0, 0.0, 10.0, 10.0),
            [100, 100],
            Margins::new(40.0, 40.0, 40.0, 40.0),
            1.0,
            1.0,
        );
        // No centre fits; the caps share the space instead of overlapping.
        assert!(pieces.iter().all(|p| p.dest.width() <= 5.0 + 1e-4));
        assert!(!pieces.iter().any(|p| p.center));
    }

    #[test]
    fn three_slice_caps_keep_their_aspect() {
        let [l, m, rr] = three_h_rects(r(0.0, 0.0, 300.0, 40.0), [20, 80], [30, 80], 1.0);
        assert_eq!(l, r(0.0, 0.0, 10.0, 40.0));
        assert_eq!(rr, r(285.0, 0.0, 300.0, 40.0));
        assert_eq!(m, r(10.0, 0.0, 285.0, 40.0));
        let [t, _, b] = three_v_rects(r(0.0, 0.0, 10.0, 100.0), [20, 10], [20, 10], 1.0);
        assert_eq!(t.height(), 5.0);
        assert_eq!(b.height(), 5.0);
    }
}
