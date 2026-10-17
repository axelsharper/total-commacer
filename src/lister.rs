//! Lister (F3): text, binary and hex views of a file, with search. Opens in its own window.

use std::path::{Path, PathBuf};

use egui::{Align2, Color32, Pos2, Rect, Sense, Ui, Vec2};

use crate::xp;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Text,
    Binary,
    Hex,
}

pub struct Lister {
    pub path: PathBuf,
    pub data: Vec<u8>,
    pub mode: Mode,
    pub wrap: bool,
    pub scroll: usize,
    pub lines: Vec<String>,
    pub search: String,
    pub searching: bool,
    pub search_hit: Option<usize>,
    pub rows_visible: usize,
    pub close: bool,
    pub menu_open: Option<usize>,
    cache_key: (Mode, bool, usize),
}

const MAX_BYTES: usize = 64 * 1024 * 1024;

impl Lister {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let mut data = std::fs::read(path)?;
        if data.len() > MAX_BYTES {
            data.truncate(MAX_BYTES);
        }
        let mode = if looks_binary(&data) { Mode::Hex } else { Mode::Text };
        let mut l = Self {
            path: path.to_path_buf(),
            data,
            mode,
            wrap: true,
            scroll: 0,
            lines: Vec::new(),
            search: String::new(),
            searching: false,
            search_hit: None,
            rows_visible: 30,
            close: false,
            menu_open: None,
            cache_key: (Mode::Text, false, 0),
        };
        l.relayout(100);
        Ok(l)
    }

    pub fn title(&self) -> String {
        format!("Lister - [{}]", self.path.to_string_lossy())
    }

    fn relayout(&mut self, cols: usize) {
        let key = (self.mode, self.wrap, cols);
        if key == self.cache_key && !self.lines.is_empty() {
            return;
        }
        self.cache_key = key;
        self.lines = match self.mode {
            Mode::Text => {
                let text = String::from_utf8_lossy(&self.data).replace("\r\n", "\n").replace('\r', "\n");
                let mut out = Vec::new();
                for line in text.split('\n') {
                    let line = line.replace('\t', "        ");
                    if self.wrap && cols > 8 {
                        let chars: Vec<char> = line.chars().collect();
                        if chars.is_empty() {
                            out.push(String::new());
                        }
                        for chunk in chars.chunks(cols) {
                            out.push(chunk.iter().collect());
                        }
                    } else {
                        out.push(line);
                    }
                }
                out
            }
            Mode::Binary => {
                let cols = cols.max(16);
                self.data
                    .chunks(cols)
                    .map(|c| c.iter().map(|&b| if (32..127).contains(&b) || b >= 160 { b as char } else { '.' }).collect())
                    .collect()
            }
            Mode::Hex => self
                .data
                .chunks(16)
                .enumerate()
                .map(|(i, c)| {
                    let hex: Vec<String> = c.iter().map(|b| format!("{b:02x}")).collect();
                    let asc: String = c.iter().map(|&b| if (32..127).contains(&b) { b as char } else { '.' }).collect();
                    format!("{:08x}: {:<48} {}", i * 16, hex.join(" "), asc)
                })
                .collect(),
        };
    }

    fn find_next(&mut self, from: usize) {
        if self.search.is_empty() {
            return;
        }
        let q = self.search.to_lowercase();
        for (k, line) in self.lines.iter().enumerate().skip(from) {
            if line.to_lowercase().contains(&q) {
                self.search_hit = Some(k);
                self.scroll = k.saturating_sub(2);
                return;
            }
        }
        self.search_hit = None;
    }

    pub fn ui(&mut self, ui: &mut Ui, rect: Rect, active: bool) {
        let p = ui.painter_at(rect);
        xp::fill(&p, rect, xp::face());
        let menu_h = 20.0;
        let menu = Rect::from_min_size(rect.min, Vec2::new(rect.width(), menu_h));
        // menu bar: File Edit Options Help  (only the keys that matter are wired)
        let items = ["&File", "&Edit", "&Options", "&Help"];
        let mut x = menu.left() + 4.0;
        for (i, it) in items.iter().enumerate() {
            let (t, _) = xp::strip_mnemonic(it);
            let w = xp::text_width(&p, &t) + 14.0;
            let r = Rect::from_min_size(Pos2::new(x, menu.top() + 1.0), Vec2::new(w, menu_h - 2.0));
            let resp = ui.interact(r, ui.id().with(("lmenu", i)), Sense::CLICK);
            if resp.hovered() || self.menu_open == Some(i) {
                xp::fill(&p, r, xp::highlight());
                xp::mnemonic_text(&p, r.center(), Align2::CENTER_CENTER, it, xp::font(), xp::HIGHLIGHT_TEXT);
            } else {
                xp::mnemonic_text(&p, r.center(), Align2::CENTER_CENTER, it, xp::font(), xp::TEXT);
            }
            if resp.clicked() {
                self.menu_open = if self.menu_open == Some(i) { None } else { Some(i) };
            }
            x += w;
        }
        let status_h = 20.0;
        let view = Rect::from_min_max(Pos2::new(rect.left(), menu.bottom()), Pos2::new(rect.right(), rect.bottom() - status_h));
        xp::sunken(&p, view);
        let inner = view.shrink(2.0);
        let text_area = Rect::from_min_max(inner.min, Pos2::new(inner.right() - xp::SB_W, inner.bottom()));
        let sb = Rect::from_min_max(Pos2::new(inner.right() - xp::SB_W, inner.top()), inner.right_bottom());
        let line_h = 14.0;
        let char_w = p.layout_no_wrap("M".into(), xp::font_mono(), xp::TEXT).size().x.max(1.0);
        let cols = ((text_area.width() - 6.0) / char_w).floor().max(8.0) as usize;
        self.relayout(cols);
        self.rows_visible = (text_area.height() / line_h).floor().max(1.0) as usize;
        let max_scroll = self.lines.len().saturating_sub(self.rows_visible);
        self.scroll = self.scroll.min(max_scroll);

        // keys
        if active && !self.searching {
            let (keys, text) = ui.input(|i| {
                let mut keys = Vec::new();
                let mut text = String::new();
                for e in &i.events {
                    match e {
                        egui::Event::Key { key, pressed: true, modifiers, .. } => keys.push((*key, *modifiers)),
                        egui::Event::Text(t) => text.push_str(t),
                        _ => {}
                    }
                }
                (keys, text)
            });
            for (k, m) in keys {
                use egui::Key::*;
                match k {
                    Escape | Q => self.close = true,
                    ArrowDown => self.scroll = (self.scroll + 1).min(max_scroll),
                    ArrowUp => self.scroll = self.scroll.saturating_sub(1),
                    PageDown | Space => self.scroll = (self.scroll + self.rows_visible).min(max_scroll),
                    PageUp => self.scroll = self.scroll.saturating_sub(self.rows_visible),
                    Home => self.scroll = 0,
                    End => self.scroll = max_scroll,
                    F7 => {
                        self.searching = true;
                        self.menu_open = None;
                    }
                    F3 => {
                        let from = self.search_hit.map(|h| h + 1).unwrap_or(self.scroll);
                        self.find_next(from);
                    }
                    F2 => {
                        if let Ok(d) = std::fs::read(&self.path) {
                            self.data = d;
                            self.lines.clear();
                            self.relayout(cols);
                        }
                    }
                    Num1 => self.set_mode(Mode::Text, cols),
                    Num2 => self.set_mode(Mode::Binary, cols),
                    Num3 => self.set_mode(Mode::Hex, cols),
                    W => {
                        self.wrap = !self.wrap;
                        self.lines.clear();
                        self.relayout(cols);
                    }
                    F if m.ctrl => self.searching = true,
                    _ => {}
                }
            }
            for c in text.chars() {
                match c {
                    '1' => self.set_mode(Mode::Text, cols),
                    '2' => self.set_mode(Mode::Binary, cols),
                    '3' => self.set_mode(Mode::Hex, cols),
                    _ => {}
                }
            }
        }

        // text
        let tp = ui.painter_at(text_area);
        let q = self.search.to_lowercase();
        for row in 0..self.rows_visible {
            let i = self.scroll + row;
            let Some(line) = self.lines.get(i) else { break };
            let y = text_area.top() + row as f32 * line_h;
            if self.search_hit == Some(i) && !q.is_empty() {
                if let Some(pos) = line.to_lowercase().find(&q) {
                    let pre = line[..pos].chars().count() as f32;
                    let len = q.chars().count() as f32;
                    tp.rect_filled(Rect::from_min_size(Pos2::new(text_area.left() + 2.0 + pre * char_w, y), Vec2::new(len * char_w, line_h)), 0.0, xp::highlight());
                }
            }
            tp.text(Pos2::new(text_area.left() + 2.0, y), Align2::LEFT_TOP, line, xp::font_mono(), xp::TEXT);
        }
        if let Some(s) = crate::panel::scrollbar(ui, &p, sb, ui.id().with("lsb"), self.scroll, self.rows_visible, self.lines.len(), true) {
            self.scroll = s;
        }
        let resp = ui.interact(text_area, ui.id().with("lview"), Sense::CLICK);
        if resp.hovered() {
            let dy = crate::panel::wheel_dy(ui);
            if dy > 0.0 {
                self.scroll = self.scroll.saturating_sub(3);
            } else if dy < 0.0 {
                self.scroll = (self.scroll + 3).min(max_scroll);
            }
        }
        if resp.clicked() {
            self.menu_open = None;
        }

        // status
        let status = Rect::from_min_size(Pos2::new(rect.left(), rect.bottom() - status_h), Vec2::new(rect.width(), status_h));
        xp::fill(&p, status, xp::face());
        xp::hseparator(&p, status.left(), status.right(), status.top());
        let pct = if self.lines.is_empty() { 100 } else { ((self.scroll + self.rows_visible).min(self.lines.len()) * 100 / self.lines.len()) as u32 };
        let mode = match self.mode {
            Mode::Text => "Text",
            Mode::Binary => "Binary",
            Mode::Hex => "Hex",
        };
        xp::text(&p, status.min + Vec2::new(4.0, 4.0), &format!("{}  {}%  {}  {} bytes   1 Text  2 Binary  3 Hex  W Wrap  F7 Search  F3 Next  Q Quit", mode, pct, if self.wrap { "Wrap" } else { "" }, crate::fs::fmt_num(self.data.len() as u64)), xp::TEXT);

        // search box
        if self.searching {
            let r = Rect::from_center_size(view.center(), Vec2::new(360.0, 110.0));
            let _ = xp::dialog_frame(&p, r, "Find");
            xp::text(&p, r.min + Vec2::new(10.0, 34.0), "Find what:", xp::TEXT);
            let er = Rect::from_min_size(r.min + Vec2::new(70.0, 31.0), Vec2::new(270.0, 20.0));
            xp::sunken(&p, er);
            let te = egui::TextEdit::singleline(&mut self.search).id_salt("lister-find").frame(egui::Frame::NONE).font(xp::font()).desired_width(260.0);
            let mut cui = ui.new_child(egui::UiBuilder::new().max_rect(er.shrink2(Vec2::new(3.0, 2.0))));
            let resp = cui.add(te);
            resp.request_focus();
            let ok = Rect::from_min_size(r.min + Vec2::new(r.width() - 170.0, 70.0), Vec2::new(75.0, 23.0));
            let cancel = Rect::from_min_size(r.min + Vec2::new(r.width() - 85.0, 70.0), Vec2::new(75.0, 23.0));
            let okr = ui.interact(ok, ui.id().with("fok"), Sense::CLICK);
            let cr = ui.interact(cancel, ui.id().with("fcancel"), Sense::CLICK);
            xp::button_face(&p, ok, okr.hovered(), okr.is_pointer_button_down_on(), true, true);
            xp::button_face(&p, cancel, cr.hovered(), cr.is_pointer_button_down_on(), false, true);
            p.text(ok.center(), Align2::CENTER_CENTER, "OK", xp::font(), xp::TEXT);
            p.text(cancel.center(), Align2::CENTER_CENTER, "Cancel", xp::font(), xp::TEXT);
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
            if okr.clicked() || enter {
                self.searching = false;
                self.find_next(self.scroll);
            }
            if cr.clicked() || esc {
                self.searching = false;
            }
        }
        if self.menu_open.is_some() {
            self.menus(ui, &p, menu);
        }
    }

    fn set_mode(&mut self, m: Mode, cols: usize) {
        self.mode = m;
        self.lines.clear();
        self.relayout(cols);
        self.scroll = 0;
    }

    fn menus(&mut self, ui: &mut Ui, p: &egui::Painter, menu: Rect) {
        let Some(open) = self.menu_open else { return };
        let defs: [&[(&str, &str)]; 4] = [
            &[("&Open...", ""), ("&Reload", "F2"), ("-", ""), ("E&xit", "Esc")],
            &[("&Find...", "F7"), ("Find &Next", "F3"), ("-", ""), ("Select &All", "Ctrl+A")],
            &[("&Text only", "1"), ("&Binary (fixed width)", "2"), ("&Hex", "3"), ("-", ""), ("&Wrap text", "W")],
            &[("&About Lister", "")],
        ];
        let x0 = menu.left() + 4.0 + (0..open).map(|i| xp::text_width(p, &xp::strip_mnemonic(["&File", "&Edit", "&Options", "&Help"][i]).0) + 14.0).sum::<f32>();
        let items = defs[open];
        let w = 190.0;
        let h = items.iter().map(|(l, _)| if *l == "-" { 7.0 } else { 18.0 }).sum::<f32>() + 6.0;
        let r = Rect::from_min_size(Pos2::new(x0, menu.bottom()), Vec2::new(w, h));
        let mp = ui.painter_at(r.expand(2.0));
        xp::menu_frame(&mp, r);
        let mut y = r.top() + 3.0;
        for (label, key) in items {
            if *label == "-" {
                xp::menu_separator(&mp, r, y + 3.0);
                y += 7.0;
                continue;
            }
            let ir = Rect::from_min_size(Pos2::new(r.left() + 2.0, y), Vec2::new(w - 4.0, 18.0));
            let resp = ui.interact(ir, ui.id().with(("lmi", open, y as i32)), Sense::CLICK);
            let (fg, bg) = if resp.hovered() { (xp::HIGHLIGHT_TEXT, xp::highlight()) } else { (xp::TEXT, Color32::TRANSPARENT) };
            xp::fill(&mp, ir, bg);
            xp::mnemonic_text(&mp, Pos2::new(ir.left() + 26.0, ir.center().y), Align2::LEFT_CENTER, label, xp::font(), fg);
            mp.text(Pos2::new(ir.right() - 8.0, ir.center().y), Align2::RIGHT_CENTER, key, xp::font(), fg);
            if resp.clicked() {
                self.menu_open = None;
                match (open, *label) {
                    (0, "&Reload") => {
                        if let Ok(d) = std::fs::read(&self.path) {
                            self.data = d;
                            self.lines.clear();
                        }
                    }
                    (0, "E&xit") => self.close = true,
                    (1, "&Find...") => self.searching = true,
                    (1, "Find &Next") => {
                        let from = self.search_hit.map(|h| h + 1).unwrap_or(self.scroll);
                        self.find_next(from);
                    }
                    (2, "&Text only") => self.set_mode(Mode::Text, 100),
                    (2, "&Binary (fixed width)") => self.set_mode(Mode::Binary, 100),
                    (2, "&Hex") => self.set_mode(Mode::Hex, 100),
                    (2, "&Wrap text") => {
                        self.wrap = !self.wrap;
                        self.lines.clear();
                    }
                    _ => {}
                }
            }
            y += 18.0;
        }
        if ui.input(|i| i.pointer.any_click()) && !ui.rect_contains_pointer(r) && !ui.rect_contains_pointer(menu) {
            self.menu_open = None;
        }
    }
}

fn looks_binary(data: &[u8]) -> bool {
    let sample = &data[..data.len().min(4096)];
    if sample.is_empty() {
        return false;
    }
    let ctrl = sample.iter().filter(|&&b| b == 0 || (b < 32 && b != 9 && b != 10 && b != 13 && b != 12)).count();
    ctrl * 20 > sample.len()
}
