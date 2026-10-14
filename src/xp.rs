//! Windows XP "Luna" (blue) look, painted by hand.
//!
//! Every colour here is a plain RGB value of the Luna visual style; nothing is copied from
//! any binary or image. The gradient stops follow the well-known XP.css reconstruction.

use egui::{Align2, Color32, FontId, Mesh, Pos2, Rect, Stroke, StrokeKind, Vec2};

pub const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

// --- system colours (XP default scheme) ---------------------------------------------------
pub const FACE: Color32 = rgb(0xEC, 0xE9, 0xD8); // ButtonFace: dialogs, menus, bars
pub const LIGHT: Color32 = rgb(0xF1, 0xEF, 0xE2); // 3D light
pub const HILIGHT: Color32 = rgb(0xFF, 0xFF, 0xFF); // 3D highlight
pub const SHADOW: Color32 = rgb(0xAC, 0xA8, 0x99); // 3D shadow
pub const DKSHADOW: Color32 = rgb(0x71, 0x6F, 0x64); // 3D dark shadow
pub const WINDOW: Color32 = rgb(0xFF, 0xFF, 0xFF);
pub const TEXT: Color32 = rgb(0x00, 0x00, 0x00);
pub const GRAYTEXT: Color32 = rgb(0xAC, 0xA8, 0x99);
pub const HIGHLIGHT: Color32 = rgb(0x31, 0x6A, 0xC5); // selection / menu highlight (navy)
pub const HIGHLIGHT_TEXT: Color32 = rgb(0xFF, 0xFF, 0xFF);
pub const EDIT_BORDER: Color32 = rgb(0x7F, 0x9D, 0xB9); // themed edit/list border
pub const BUTTON_BORDER: Color32 = rgb(0x00, 0x3C, 0x74);
pub const TOOLTIP: Color32 = rgb(0xFF, 0xFF, 0xE1);
pub const MARK: Color32 = rgb(0xFF, 0x00, 0x00); // Total Commander's default "mark" colour

// --- window frame -------------------------------------------------------------------------
pub const FRAME: Color32 = rgb(0x08, 0x31, 0xD9);
pub const FRAME_LIGHT: Color32 = rgb(0x16, 0x6A, 0xEE);
pub const FRAME_DARK: Color32 = rgb(0x00, 0x13, 0x8C);
pub const FRAME_INACTIVE: Color32 = rgb(0x7A, 0x96, 0xDF);

pub const TITLE_H: f32 = 29.0;
pub const BORDER_W: f32 = 4.0;

pub const CAPTION_ACTIVE: &[(f32, Color32)] = &[
    (0.00, rgb(0x09, 0x97, 0xFF)),
    (0.08, rgb(0x00, 0x53, 0xEE)),
    (0.40, rgb(0x00, 0x50, 0xEE)),
    (0.88, rgb(0x00, 0x66, 0xFF)),
    (0.93, rgb(0x00, 0x66, 0xFF)),
    (0.95, rgb(0x00, 0x5B, 0xFF)),
    (0.96, rgb(0x00, 0x3D, 0xD7)),
    (1.00, rgb(0x00, 0x3D, 0xD7)),
];
pub const CAPTION_INACTIVE: &[(f32, Color32)] = &[
    (0.00, rgb(0x9D, 0xB9, 0xEB)),
    (0.08, rgb(0x7C, 0x9C, 0xE4)),
    (0.40, rgb(0x76, 0x96, 0xE3)),
    (0.88, rgb(0x7B, 0x9F, 0xEB)),
    (1.00, rgb(0x6E, 0x8D, 0xD8)),
];

pub fn font() -> FontId {
    FontId::new(11.0, egui::FontFamily::Proportional)
}
pub fn font_bold() -> FontId {
    FontId::new(11.0, egui::FontFamily::Name("bold".into()))
}
pub fn font_caption() -> FontId {
    FontId::new(13.0, egui::FontFamily::Name("bold".into()))
}
pub fn font_mono() -> FontId {
    FontId::new(12.0, egui::FontFamily::Monospace)
}

pub fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

pub fn stop_color(stops: &[(f32, Color32)], t: f32) -> Color32 {
    if stops.is_empty() {
        return Color32::BLACK;
    }
    if t <= stops[0].0 {
        return stops[0].1;
    }
    for w in stops.windows(2) {
        let (t0, c0) = w[0];
        let (t1, c1) = w[1];
        if t <= t1 {
            let f = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
            return lerp(c0, c1, f);
        }
    }
    stops[stops.len() - 1].1
}

/// Crisp 1-px aligned filled rectangle.
pub fn fill(p: &egui::Painter, r: Rect, c: Color32) {
    p.rect_filled(r, 0.0, c);
}

pub fn px(p: &egui::Painter, x: f32, y: f32, w: f32, h: f32, c: Color32) {
    let r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
    if w < 2.0 && h < 2.0 {
        let mut mesh = Mesh::default();
        mesh.add_colored_rect(r, c);
        p.add(mesh);
    } else {
        p.rect_filled(r, 0.0, c);
    }
}

/// Vertical gradient as a two-triangle mesh per stop segment.
pub fn vgradient(p: &egui::Painter, r: Rect, stops: &[(f32, Color32)]) {
    if stops.len() < 2 {
        if let Some(s) = stops.first() {
            fill(p, r, s.1);
        }
        return;
    }
    let mut mesh = Mesh::default();
    for w in stops.windows(2) {
        let y0 = r.top() + r.height() * w[0].0;
        let y1 = r.top() + r.height() * w[1].0;
        let i = mesh.vertices.len() as u32;
        mesh.colored_vertex(Pos2::new(r.left(), y0), w[0].1);
        mesh.colored_vertex(Pos2::new(r.right(), y0), w[0].1);
        mesh.colored_vertex(Pos2::new(r.right(), y1), w[1].1);
        mesh.colored_vertex(Pos2::new(r.left(), y1), w[1].1);
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i, i + 2, i + 3);
    }
    p.add(mesh);
}

pub fn hgradient(p: &egui::Painter, r: Rect, a: Color32, b: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(r.left_top(), a);
    mesh.colored_vertex(r.right_top(), b);
    mesh.colored_vertex(r.right_bottom(), b);
    mesh.colored_vertex(r.left_bottom(), a);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(mesh);
}

/// A 1-px frame (inside the rect).
pub fn frame(p: &egui::Painter, r: Rect, c: Color32) {
    p.rect_stroke(r, 0.0, Stroke::new(1.0, c), StrokeKind::Inside);
}

/// Classic 3D sunken border (edit boxes in non-themed look); themed XP uses a flat blue line.
pub fn sunken(p: &egui::Painter, r: Rect) {
    fill(p, r, WINDOW);
    frame(p, r, EDIT_BORDER);
}

/// The XP pixel-rounded caption: each row of the gradient is inset near the top corners.
pub fn caption(p: &egui::Painter, r: Rect, active: bool) {
    let stops = if active { CAPTION_ACTIVE } else { CAPTION_INACTIVE };
    let radius = 8.0_f32;
    let rows = r.height().round() as i32;
    for i in 0..rows {
        let y = r.top() + i as f32;
        let t = i as f32 / (rows.max(1) - 1) as f32;
        let c = stop_color(stops, t);
        let inset = if (i as f32) < radius {
            let dy = radius - i as f32 - 0.5;
            radius - (radius * radius - dy * dy).max(0.0).sqrt()
        } else {
            0.0
        };
        let inset = inset.round();
        px(p, r.left() + inset, y, r.width() - 2.0 * inset, 1.0, c);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CaptionButton {
    Minimize,
    Maximize,
    Restore,
    Close,
}

/// One of the three caption buttons (21×21).
pub fn caption_button(p: &egui::Painter, r: Rect, kind: CaptionButton, hover: bool, down: bool) {
    let (top, bottom, border) = match kind {
        CaptionButton::Close => (rgb(0xE6, 0x86, 0x5F), rgb(0xC8, 0x3C, 0x1C), rgb(0xFF, 0xFF, 0xFF)),
        _ => (rgb(0x5A, 0xA5, 0xFF), rgb(0x1F, 0x5D, 0xE0), rgb(0xFF, 0xFF, 0xFF)),
    };
    let (top, bottom) = if down {
        (lerp(top, Color32::BLACK, 0.25), lerp(bottom, Color32::BLACK, 0.25))
    } else if hover {
        (lerp(top, Color32::WHITE, 0.25), lerp(bottom, Color32::WHITE, 0.15))
    } else {
        (top, bottom)
    };
    p.rect_filled(r, 3.0, bottom);
    vgradient(p, r.shrink(1.0), &[(0.0, top), (1.0, bottom)]);
    p.rect_stroke(r, 3.0, Stroke::new(1.0, border), StrokeKind::Inside);
    let c = Color32::WHITE;
    let cx = r.center().x.round();
    let cy = r.center().y.round();
    match kind {
        CaptionButton::Minimize => {
            px(p, cx - 4.0, cy + 3.0, 6.0, 3.0, c);
        }
        CaptionButton::Maximize => {
            px(p, cx - 5.0, cy - 5.0, 10.0, 3.0, c);
            px(p, cx - 5.0, cy - 2.0, 1.0, 7.0, c);
            px(p, cx + 4.0, cy - 2.0, 1.0, 7.0, c);
            px(p, cx - 5.0, cy + 4.0, 10.0, 1.0, c);
        }
        CaptionButton::Restore => {
            px(p, cx - 2.0, cy - 6.0, 8.0, 2.0, c);
            px(p, cx + 5.0, cy - 4.0, 1.0, 5.0, c);
            px(p, cx - 2.0, cy - 4.0, 1.0, 1.0, c);
            px(p, cx + 2.0, cy, 4.0, 1.0, c);
            px(p, cx - 6.0, cy - 2.0, 8.0, 2.0, c);
            px(p, cx - 6.0, cy, 1.0, 5.0, c);
            px(p, cx + 1.0, cy, 1.0, 5.0, c);
            px(p, cx - 6.0, cy + 4.0, 8.0, 1.0, c);
        }
        CaptionButton::Close => {
            for i in 0..8 {
                let i = i as f32;
                px(p, cx - 4.0 + i, cy - 4.0 + i, 2.0, 1.0, c);
                px(p, cx + 3.0 - i, cy - 4.0 + i, 2.0, 1.0, c);
            }
        }
    }
}

/// Themed push button face.
pub fn button_face(p: &egui::Painter, r: Rect, hover: bool, down: bool, default: bool, enabled: bool) {
    let r = Rect::from_min_size(r.min.round(), r.size().round());
    if !enabled {
        p.rect_filled(r, 3.0, rgb(0xF5, 0xF4, 0xEA));
        p.rect_stroke(r, 3.0, Stroke::new(1.0, rgb(0xC9, 0xC7, 0xBA)), StrokeKind::Inside);
        return;
    }
    if down {
        vgradient(p, r, &[(0.0, rgb(0xCD, 0xCA, 0xC3)), (0.15, rgb(0xE3, 0xE3, 0xDB)), (1.0, rgb(0xF2, 0xF2, 0xEE))]);
    } else {
        vgradient(p, r, &[(0.0, rgb(0xFF, 0xFF, 0xFF)), (0.86, rgb(0xEC, 0xEB, 0xE5)), (1.0, rgb(0xD8, 0xD0, 0xC4))]);
    }
    p.rect_stroke(r, 3.0, Stroke::new(1.0, BUTTON_BORDER), StrokeKind::Inside);
    let inner = r.shrink(1.0);
    if hover && !down {
        // orange glow
        p.rect_stroke(inner, 2.0, Stroke::new(1.0, rgb(0xFD, 0xD8, 0x89)), StrokeKind::Inside);
        p.rect_stroke(inner.shrink(1.0), 1.0, Stroke::new(1.0, rgb(0xFB, 0xC7, 0x61)), StrokeKind::Inside);
        px(p, inner.left() + 2.0, inner.bottom() - 2.0, inner.width() - 4.0, 1.0, rgb(0xE5, 0xA0, 0x1A));
    } else if default && !down {
        // blue glow
        p.rect_stroke(inner, 2.0, Stroke::new(1.0, rgb(0xCE, 0xE7, 0xFF)), StrokeKind::Inside);
        p.rect_stroke(inner.shrink(1.0), 1.0, Stroke::new(1.0, rgb(0x98, 0xB8, 0xEA)), StrokeKind::Inside);
        px(p, inner.left() + 2.0, inner.bottom() - 2.0, inner.width() - 4.0, 1.0, rgb(0x89, 0xAD, 0xE4));
    }
}

/// Button text with a mnemonic underline for `&x`.
pub fn mnemonic_text(p: &egui::Painter, pos: Pos2, align: Align2, label: &str, font: FontId, color: Color32) -> Rect {
    let (text, under) = strip_mnemonic(label);
    let galley = p.layout_no_wrap(text.clone(), font.clone(), color);
    let rect = align.anchor_size(pos, galley.size());
    p.galley(rect.min, galley.clone(), color);
    if let Some(i) = under {
        let before = p.layout_no_wrap(text.chars().take(i).collect(), font.clone(), color);
        let ch = p.layout_no_wrap(text.chars().nth(i).map(|c| c.to_string()).unwrap_or_default(), font, color);
        let x = rect.left() + before.size().x;
        px(p, x.round(), (rect.bottom() - 1.0).round(), ch.size().x.round().max(1.0), 1.0, color);
    }
    rect
}

pub fn strip_mnemonic(label: &str) -> (String, Option<usize>) {
    let mut out = String::new();
    let mut under = None;
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            if let Some(&n) = chars.peek() {
                if n == '&' {
                    out.push('&');
                    chars.next();
                    continue;
                }
                under = Some(out.chars().count());
                continue;
            }
        }
        out.push(c);
    }
    (out, under)
}

/// Dotted focus rectangle (every other pixel), like a Win32 DrawFocusRect.
/// Drawn as a mesh: single-pixel `rect_filled`s are culled by the tessellator.
pub fn focus_rect(p: &egui::Painter, r: Rect, c: Color32) {
    let r = Rect::from_min_size(r.min.round(), r.size().round());
    let mut mesh = Mesh::default();
    let dot = |m: &mut Mesh, x: f32, y: f32| m.add_colored_rect(Rect::from_min_size(Pos2::new(x, y), Vec2::splat(1.0)), c);
    let mut x = r.left();
    while x < r.right() {
        dot(&mut mesh, x, r.top());
        dot(&mut mesh, x, r.bottom() - 1.0);
        x += 2.0;
    }
    let mut y = r.top() + 2.0;
    while y < r.bottom() - 1.0 {
        dot(&mut mesh, r.left(), y);
        dot(&mut mesh, r.right() - 1.0, y);
        y += 2.0;
    }
    p.add(mesh);
}

/// XP list-view header cell.
pub fn header_cell(p: &egui::Painter, r: Rect, hover: bool, down: bool) {
    if down {
        fill(p, r, rgb(0xE8, 0xE8, 0xE0));
    } else {
        vgradient(p, r, &[(0.0, rgb(0xFF, 0xFF, 0xFF)), (0.8, rgb(0xF7, 0xF7, 0xF3)), (1.0, rgb(0xEE, 0xED, 0xE5))]);
    }
    // separator at the right edge
    px(p, r.right() - 2.0, r.top() + 3.0, 1.0, r.height() - 6.0, rgb(0xC5, 0xC3, 0xBB));
    px(p, r.right() - 1.0, r.top() + 3.0, 1.0, r.height() - 6.0, rgb(0xFF, 0xFF, 0xFF));
    if hover {
        px(p, r.left(), r.bottom() - 3.0, r.width() - 2.0, 3.0, rgb(0xF9, 0xB1, 0x4F));
    }
}

pub fn header_bottom(p: &egui::Painter, r: Rect) {
    px(p, r.left(), r.bottom() - 1.0, r.width(), 1.0, rgb(0xD5, 0xD2, 0xCA));
}

/// Small sort-order triangle used in TC's column headers.
pub fn sort_arrow(p: &egui::Painter, pos: Pos2, up: bool, c: Color32) {
    for i in 0..4 {
        let w = (i * 2 + 1) as f32;
        let y = if up { pos.y + i as f32 } else { pos.y + 3.0 - i as f32 };
        px(p, pos.x - i as f32, y, w, 1.0, c);
    }
}

// --- scrollbar ------------------------------------------------------------------------------
pub const SB_W: f32 = 17.0;

pub fn scrollbar_track(p: &egui::Painter, r: Rect) {
    hgradient(p, r, rgb(0xF4, 0xF3, 0xF0), rgb(0xFA, 0xFA, 0xF8));
    px(p, r.left(), r.top(), 1.0, r.height(), rgb(0xEC, 0xEB, 0xE7));
}

pub fn scrollbar_button(p: &egui::Painter, r: Rect, hover: bool, down: bool) {
    let r = Rect::from_min_size(r.min.round(), r.size().round());
    let (a, b) = if down {
        (rgb(0x9A, 0xBA, 0xEC), rgb(0x7E, 0x9F, 0xD6))
    } else if hover {
        (rgb(0xDB, 0xE7, 0xFB), rgb(0xB2, 0xCD, 0xF5))
    } else {
        (rgb(0xC6, 0xD8, 0xF6), rgb(0xA3, 0xC0, 0xF0))
    };
    p.rect_filled(r, 2.0, b);
    hgradient(p, r.shrink(1.0), a, b);
    p.rect_stroke(r, 2.0, Stroke::new(1.0, rgb(0x4D, 0x61, 0x85)), StrokeKind::Inside);
    p.rect_stroke(r.shrink(1.0), 1.0, Stroke::new(1.0, rgb(0xFF, 0xFF, 0xFF)), StrokeKind::Inside);
}

pub fn scrollbar_thumb(p: &egui::Painter, r: Rect, hover: bool, down: bool) {
    scrollbar_button(p, r, hover, down);
    if r.height() > 20.0 {
        let cy = r.center().y.round();
        let cx = r.center().x.round();
        for i in 0..4 {
            let y = cy - 4.0 + i as f32 * 2.0;
            px(p, cx - 3.0, y, 5.0, 1.0, rgb(0x4D, 0x61, 0x85));
            px(p, cx - 2.0, y + 1.0, 5.0, 1.0, rgb(0xFF, 0xFF, 0xFF));
        }
    }
}

pub fn arrow_glyph(p: &egui::Painter, center: Pos2, dir: Dir, c: Color32) {
    let cx = center.x.round();
    let cy = center.y.round();
    for i in 0..4 {
        let w = (i * 2 + 1) as f32;
        match dir {
            Dir::Up => px(p, cx - i as f32, cy - 2.0 + i as f32, w, 1.0, c),
            Dir::Down => px(p, cx - i as f32, cy + 1.0 - i as f32, w, 1.0, c),
            Dir::Left => px(p, cx - 2.0 + i as f32, cy - i as f32, 1.0, w, c),
            Dir::Right => px(p, cx + 1.0 - i as f32, cy - i as f32, 1.0, w, c),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

/// Toolbar separator (etched vertical line).
pub fn vseparator(p: &egui::Painter, x: f32, y0: f32, y1: f32) {
    px(p, x, y0, 1.0, y1 - y0, SHADOW);
    px(p, x + 1.0, y0, 1.0, y1 - y0, HILIGHT);
}

pub fn hseparator(p: &egui::Painter, x0: f32, x1: f32, y: f32) {
    px(p, x0, y, x1 - x0, 1.0, SHADOW);
    px(p, x0, y + 1.0, x1 - x0, 1.0, HILIGHT);
}

/// Flat toolbar button that gets a raised frame on hover (XP toolbars).
pub fn tool_button(p: &egui::Painter, r: Rect, hover: bool, down: bool, checked: bool) {
    if down || checked {
        fill(p, r, rgb(0xE1, 0xE2, 0xF5));
        frame(p, r, rgb(0x31, 0x6A, 0xC5));
    } else if hover {
        fill(p, r, rgb(0xF6, 0xF7, 0xFD));
        frame(p, r, rgb(0x31, 0x6A, 0xC5));
    }
}

/// XP progress bar (segmented green blocks).
pub fn progress_bar(p: &egui::Painter, r: Rect, frac: f32) {
    sunken(p, r);
    let inner = r.shrink(2.0);
    let w = (inner.width() * frac.clamp(0.0, 1.0)).round();
    let mut x = inner.left();
    while x + 8.0 <= inner.left() + w + 0.1 {
        let seg = Rect::from_min_size(Pos2::new(x, inner.top()), Vec2::new(6.0, inner.height()));
        vgradient(p, seg, &[(0.0, rgb(0xB6, 0xE2, 0xA8)), (0.5, rgb(0x4A, 0xB7, 0x2B)), (1.0, rgb(0x8B, 0xD5, 0x79))]);
        x += 8.0;
    }
}

/// XP check box (13×13).
pub fn checkbox(p: &egui::Painter, r: Rect, checked: bool, hover: bool) {
    let r = Rect::from_min_size(r.min.round(), Vec2::splat(13.0));
    vgradient(p, r, &[(0.0, rgb(0xDC, 0xDC, 0xD7)), (1.0, rgb(0xFF, 0xFF, 0xFF))]);
    frame(p, r, rgb(0x1C, 0x5E, 0x9C));
    if hover {
        frame(p, r.shrink(1.0), rgb(0xFF, 0xDF, 0x9A));
    }
    if checked {
        let c = rgb(0x21, 0xA1, 0x21);
        let x0 = r.left() + 3.0;
        let y0 = r.top() + 5.0;
        for i in 0..3 {
            px(p, x0 + i as f32, y0 + i as f32, 1.0, 3.0, c);
        }
        for i in 0..5 {
            px(p, x0 + 3.0 + i as f32, y0 + 2.0 - i as f32, 1.0, 3.0, c);
        }
    }
}

/// XP radio button (13×13).
pub fn radio(p: &egui::Painter, r: Rect, checked: bool) {
    let c = r.center();
    p.circle_filled(c, 6.0, rgb(0xFF, 0xFF, 0xFF));
    p.circle_stroke(c, 6.0, Stroke::new(1.0, rgb(0x1C, 0x5E, 0x9C)));
    if checked {
        p.circle_filled(c, 2.5, rgb(0x21, 0xA1, 0x21));
    }
}

/// Group box with a caption.
pub fn group_box(p: &egui::Painter, r: Rect, title: &str) {
    let g = p.layout_no_wrap(title.to_string(), font(), rgb(0x00, 0x46, 0xD5));
    let top = r.top() + (g.size().y / 2.0).round();
    let edge = rgb(0xD0, 0xD0, 0xBF);
    px(p, r.left(), top, 8.0, 1.0, edge);
    px(p, r.left() + 10.0 + g.size().x + 2.0, top, r.width() - 12.0 - g.size().x, 1.0, edge);
    px(p, r.left(), top, 1.0, r.height() - (top - r.top()), edge);
    px(p, r.right() - 1.0, top, 1.0, r.height() - (top - r.top()), edge);
    px(p, r.left(), r.bottom() - 1.0, r.width(), 1.0, edge);
    p.galley(Pos2::new(r.left() + 10.0, r.top()), g, rgb(0x00, 0x46, 0xD5));
}

pub fn text(p: &egui::Painter, pos: Pos2, s: &str, c: Color32) -> Rect {
    p.text(pos, Align2::LEFT_TOP, s, font(), c)
}

pub fn text_bold(p: &egui::Painter, pos: Pos2, s: &str, c: Color32) -> Rect {
    p.text(pos, Align2::LEFT_TOP, s, font_bold(), c)
}

pub fn text_width(p: &egui::Painter, s: &str) -> f32 {
    p.layout_no_wrap(s.to_string(), font(), TEXT).size().x
}

/// Shorten a string with an ellipsis so it fits `max_w` pixels.
pub fn ellipsize(p: &egui::Painter, s: &str, max_w: f32, f: FontId) -> String {
    if p.layout_no_wrap(s.to_string(), f.clone(), TEXT).size().x <= max_w {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let mut n = chars.len();
    while n > 0 {
        n -= 1;
        let t: String = chars[..n].iter().collect::<String>() + "...";
        if p.layout_no_wrap(t.clone(), f.clone(), TEXT).size().x <= max_w {
            return t;
        }
    }
    "...".into()
}

/// Shorten a path the way Windows does: "c:\...\last".
pub fn ellipsize_path(p: &egui::Painter, s: &str, max_w: f32, f: FontId) -> String {
    if p.layout_no_wrap(s.to_string(), f.clone(), TEXT).size().x <= max_w {
        return s.to_string();
    }
    let sep = if s.contains('\\') { '\\' } else { '/' };
    let parts: Vec<&str> = s.split(sep).collect();
    if parts.len() < 3 {
        return ellipsize(p, s, max_w, f);
    }
    for keep in (1..parts.len() - 1).rev() {
        let t = format!("{}{sep}...{sep}{}", parts[0], parts[parts.len() - keep..].join(&sep.to_string()));
        if p.layout_no_wrap(t.clone(), f.clone(), TEXT).size().x <= max_w {
            return t;
        }
    }
    ellipsize(p, s, max_w, f)
}

pub fn round_rect(r: Rect) -> Rect {
    Rect::from_min_size(r.min.round(), r.size().round())
}
