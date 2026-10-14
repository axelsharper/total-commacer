//! One file panel: drive row, path row, column headers, the list, the status line.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use egui::{Align2, Color32, Id, Pos2, Rect, Sense, Ui, Vec2};

use crate::fs::{self, Entry, SortBy};
use crate::icons::{self, Icons};
use crate::xp;

pub const ROW_H: f32 = 17.0;
pub const HEADER_H: f32 = 18.0;
pub const DRIVE_ROW_H: f32 = 24.0;
pub const PATH_ROW_H: f32 = 18.0;
pub const STATUS_H: f32 = 18.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Brief,
    Full,
}

#[derive(Clone, Debug)]
pub enum PanelEvent {
    Activate,
    OpenFile(PathBuf),
    DriveMenu,
    ContextMenu(usize),
}

#[derive(Clone, Debug)]
pub struct Tab {
    pub path: PathBuf,
    pub cursor: Option<String>,
}

pub struct Panel {
    pub path: PathBuf,
    pub entries: Vec<Entry>,
    pub cursor: usize,
    pub selected: HashSet<String>,
    pub saved_selection: HashSet<String>,
    pub sort: SortBy,
    pub reverse: bool,
    pub view: View,
    pub scroll: usize,
    pub quick: Option<String>,
    pub history: Vec<PathBuf>,
    pub hist_pos: usize,
    pub error: Option<String>,
    pub show_hidden: bool,
    pub filter: String,
    pub branch: bool,
    pub tabs: Vec<Tab>,
    pub tab: usize,
    pub free: Option<(u64, u64)>,
    pub rows_visible: usize,
    pub cols_visible: usize,
    pub list_title: Option<String>,
    pub drive_label: String,
    sel_anchor: Option<usize>,
}

impl Panel {
    pub fn new(path: PathBuf, sort: SortBy, reverse: bool, brief: bool, show_hidden: bool) -> Self {
        let mut p = Self {
            path: path.clone(),
            entries: Vec::new(),
            cursor: 0,
            selected: HashSet::new(),
            saved_selection: HashSet::new(),
            sort,
            reverse,
            view: if brief { View::Brief } else { View::Full },
            scroll: 0,
            quick: None,
            history: vec![path],
            hist_pos: 0,
            error: None,
            show_hidden,
            filter: "*.*".into(),
            branch: false,
            tabs: Vec::new(),
            tab: 0,
            free: None,
            rows_visible: 20,
            cols_visible: 3,
            list_title: None,
            drive_label: String::new(),
            sel_anchor: None,
        };
        p.reload();
        p
    }

    // ------------------------------------------------------------------ data
    pub fn reload(&mut self) {
        let keep = self.cursor_entry().map(|e| e.name.clone());
        let listing = if self.branch { self.branch_listing() } else { fs::list_dir(&self.path, self.show_hidden) };
        match listing {
            Ok(mut v) => {
                if self.filter != "*.*" && !self.filter.is_empty() {
                    let f = self.filter.clone();
                    v.retain(|e| e.is_dir || fs::matches_mask(&e.name, &f));
                }
                fs::sort_entries(&mut v, self.sort, self.reverse);
                self.entries = v;
                self.error = None;
            }
            Err(e) => {
                self.error = Some(format!("{}\n{}", fs::display(&self.path), fs::err_text(&e)));
                // fall back to the parent if the directory vanished
                if !self.path.is_dir() {
                    if let Some(p) = self.path.parent() {
                        self.path = p.to_path_buf();
                        return self.reload();
                    }
                }
                self.entries = fs::list_dir(&self.path, self.show_hidden).unwrap_or_default();
            }
        }
        let names: HashSet<&str> = self.entries.iter().map(|e| e.name.as_str()).collect();
        self.selected.retain(|n| names.contains(n.as_str()));
        if let Some(k) = keep {
            if let Some(i) = self.entries.iter().position(|e| e.name == k) {
                self.cursor = i;
            }
        }
        self.cursor = self.cursor.min(self.entries.len().saturating_sub(1));
        self.free = fs::space(&self.path);
        self.ensure_visible();
    }

    fn branch_listing(&self) -> std::io::Result<Vec<Entry>> {
        let mut v = Vec::new();
        if let Some(parent) = self.path.parent() {
            v.push(Entry::up(parent.to_path_buf()));
        }
        for e in walkdir::WalkDir::new(&self.path).min_depth(1).into_iter().flatten() {
            if e.file_type().is_dir() {
                continue;
            }
            let rel = e.path().strip_prefix(&self.path).unwrap_or(e.path()).to_string_lossy().to_string();
            if !self.show_hidden && rel.split(['/', '\\']).any(|s| s.starts_with('.')) {
                continue;
            }
            let meta = e.metadata().ok();
            v.push(Entry {
                name: rel,
                path: e.path().to_path_buf(),
                is_dir: false,
                is_up: false,
                size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
                modified: meta.as_ref().and_then(|m| m.modified().ok()).map(chrono::DateTime::<chrono::Local>::from),
                hidden: false,
                readonly: meta.map(|m| m.permissions().readonly()).unwrap_or(false),
                system: false,
                archive: true,
                symlink: e.path_is_symlink(),
                dir_size: None,
            });
        }
        Ok(v)
    }

    pub fn goto(&mut self, path: &Path, cursor_on: Option<&str>) {
        if !path.is_dir() {
            self.error = Some(format!("{}\nThe directory does not exist.", fs::display(path)));
            return;
        }
        let path = std::fs::canonicalize(path).unwrap_or(path.to_path_buf());
        #[cfg(windows)]
        let path = PathBuf::from(path.to_string_lossy().trim_start_matches(r"\\?\"));
        if path != self.path {
            self.history.truncate(self.hist_pos + 1);
            self.history.push(path.clone());
            self.hist_pos = self.history.len() - 1;
        }
        self.path = path;
        self.branch = false;
        self.list_title = None;
        self.selected.clear();
        self.quick = None;
        self.cursor = 0;
        self.scroll = 0;
        self.reload();
        if let Some(n) = cursor_on {
            if let Some(i) = self.entries.iter().position(|e| e.name == n) {
                self.cursor = i;
            }
        }
        self.ensure_visible();
    }

    pub fn parent(&mut self) {
        if let Some(p) = self.path.parent().map(|p| p.to_path_buf()) {
            let here = self.path.file_name().map(|s| s.to_string_lossy().to_string());
            self.goto(&p, here.as_deref());
        }
    }

    pub fn root(&mut self) {
        let mut r = self.path.clone();
        while let Some(p) = r.parent() {
            r = p.to_path_buf();
        }
        self.goto(&r, None);
    }

    pub fn go_back(&mut self) {
        if self.hist_pos > 0 {
            self.hist_pos -= 1;
            let p = self.history[self.hist_pos].clone();
            self.path = p;
            self.cursor = 0;
            self.selected.clear();
            self.reload();
        }
    }

    pub fn go_forward(&mut self) {
        if self.hist_pos + 1 < self.history.len() {
            self.hist_pos += 1;
            let p = self.history[self.hist_pos].clone();
            self.path = p;
            self.cursor = 0;
            self.selected.clear();
            self.reload();
        }
    }

    pub fn cursor_entry(&self) -> Option<&Entry> {
        self.entries.get(self.cursor)
    }

    /// Files to operate on: the selection, else the file under the cursor (never "..").
    pub fn targets(&self) -> Vec<Entry> {
        let sel: Vec<Entry> = self.entries.iter().filter(|e| self.selected.contains(&e.name)).cloned().collect();
        if !sel.is_empty() {
            return sel;
        }
        match self.cursor_entry() {
            Some(e) if !e.is_up => vec![e.clone()],
            _ => vec![],
        }
    }

    pub fn has_selection(&self) -> bool {
        !self.selected.is_empty()
    }

    // ------------------------------------------------------------------ cursor
    pub fn ensure_visible(&mut self) {
        match self.view {
            View::Full => {
                let rows = self.rows_visible.max(1);
                if self.cursor < self.scroll {
                    self.scroll = self.cursor;
                } else if self.cursor >= self.scroll + rows {
                    self.scroll = self.cursor + 1 - rows;
                }
                let max_scroll = self.entries.len().saturating_sub(rows);
                self.scroll = self.scroll.min(max_scroll);
            }
            View::Brief => {
                let rows = self.rows_visible.max(1);
                let cols = self.cols_visible.max(1);
                let col = self.cursor / rows;
                let first_col = self.scroll / rows;
                if col < first_col {
                    self.scroll = col * rows;
                } else if col >= first_col + cols {
                    self.scroll = (col + 1 - cols) * rows;
                }
                let total_cols = self.entries.len().div_ceil(rows);
                let max_first = total_cols.saturating_sub(cols);
                self.scroll = self.scroll.min(max_first * rows);
            }
        }
    }

    pub fn move_cursor(&mut self, delta: i64, select: bool) {
        if self.entries.is_empty() {
            return;
        }
        let old = self.cursor;
        let n = self.entries.len() as i64;
        self.cursor = (self.cursor as i64 + delta).clamp(0, n - 1) as usize;
        if select {
            // Shift+movement: toggle the rows passed over (TC selects/deselects by the anchor state)
            let (a, b) = if old <= self.cursor { (old, self.cursor) } else { (self.cursor, old) };
            let on = !self.is_selected(old);
            for i in a..=b {
                if i == self.cursor && delta.abs() == 1 {
                    continue;
                }
                self.set_selected(i, on);
            }
        }
        self.ensure_visible();
    }

    pub fn page_rows(&self) -> i64 {
        self.rows_visible.max(1) as i64
    }

    pub fn home(&mut self, select: bool) {
        let d = -(self.cursor as i64);
        self.move_cursor(d, select);
    }
    pub fn end(&mut self, select: bool) {
        let d = self.entries.len() as i64 - 1 - self.cursor as i64;
        self.move_cursor(d, select);
    }

    // ------------------------------------------------------------------ selection
    pub fn is_selected(&self, i: usize) -> bool {
        self.entries.get(i).map(|e| self.selected.contains(&e.name)).unwrap_or(false)
    }
    pub fn set_selected(&mut self, i: usize, on: bool) {
        if let Some(e) = self.entries.get(i) {
            if e.is_up {
                return;
            }
            if on {
                self.selected.insert(e.name.clone());
            } else {
                self.selected.remove(&e.name);
            }
        }
    }
    pub fn toggle(&mut self, i: usize) {
        let on = !self.is_selected(i);
        self.set_selected(i, on);
    }
    pub fn select_mask(&mut self, mask: &str, on: bool, dirs_too: bool) {
        for i in 0..self.entries.len() {
            let e = &self.entries[i];
            if e.is_up || (e.is_dir && !dirs_too) {
                continue;
            }
            if fs::matches_mask(&e.name, mask) {
                self.set_selected(i, on);
            }
        }
    }
    pub fn invert(&mut self) {
        for i in 0..self.entries.len() {
            if !self.entries[i].is_up && !self.entries[i].is_dir {
                self.toggle(i);
            }
        }
    }
    pub fn select_same_ext(&mut self, on: bool) {
        if let Some(e) = self.cursor_entry() {
            let ext = e.ext_lower();
            for i in 0..self.entries.len() {
                if !self.entries[i].is_dir && self.entries[i].ext_lower() == ext {
                    self.set_selected(i, on);
                }
            }
        }
    }
    pub fn save_selection(&mut self) {
        self.saved_selection = self.selected.clone();
    }
    pub fn restore_selection(&mut self) {
        std::mem::swap(&mut self.selected, &mut self.saved_selection);
    }

    /// Space on a directory: count its size and show it instead of <DIR>.
    pub fn count_dir_under_cursor(&mut self) {
        let i = self.cursor;
        if let Some(e) = self.entries.get_mut(i) {
            if e.is_dir && !e.is_up {
                e.dir_size = Some(fs::dir_size(&e.path));
            }
        }
    }
    pub fn count_all_dirs(&mut self) {
        for e in self.entries.iter_mut() {
            if e.is_dir && !e.is_up {
                e.dir_size = Some(fs::dir_size(&e.path));
            }
        }
    }

    // ------------------------------------------------------------------ quick search
    pub fn quick_type(&mut self, c: char) {
        let mut q = self.quick.clone().unwrap_or_default();
        q.push(c);
        if self.quick_find(&q, self.cursor) {
            self.quick = Some(q);
        } else if !self.quick_find(&q, 0) {
            // not found anywhere: keep the old text
        } else {
            self.quick = Some(q);
        }
    }
    pub fn quick_backspace(&mut self) {
        if let Some(q) = &mut self.quick {
            q.pop();
            if q.is_empty() {
                self.quick = None;
            }
        }
    }
    pub fn quick_next(&mut self) {
        if let Some(q) = self.quick.clone() {
            let start = (self.cursor + 1) % self.entries.len().max(1);
            if !self.quick_find(&q, start) {
                self.quick_find(&q, 0);
            }
        }
    }
    fn quick_find(&mut self, q: &str, from: usize) -> bool {
        let q = q.to_lowercase();
        let n = self.entries.len();
        for k in 0..n {
            let i = (from + k) % n;
            if self.entries[i].name.to_lowercase().starts_with(&q) {
                self.cursor = i;
                self.ensure_visible();
                return true;
            }
        }
        false
    }

    // ------------------------------------------------------------------ status
    pub fn status_text(&self) -> String {
        let mut sel_b = 0u64;
        let mut tot_b = 0u64;
        let (mut sf, mut tf, mut sd, mut td) = (0, 0, 0, 0);
        for e in &self.entries {
            if e.is_up {
                continue;
            }
            let s = self.selected.contains(&e.name);
            if e.is_dir {
                td += 1;
                if s {
                    sd += 1;
                }
                if let Some(ds) = e.dir_size {
                    tot_b += ds;
                    if s {
                        sel_b += ds;
                    }
                }
            } else {
                tf += 1;
                tot_b += e.size;
                if s {
                    sf += 1;
                    sel_b += e.size;
                }
            }
        }
        format!("{} / {} in {} / {} file(s), {} / {} dir(s)", fs::fmt_k(sel_b), fs::fmt_k(tot_b), sf, tf, sd, td)
    }

    // ------------------------------------------------------------------ tabs
    pub fn new_tab(&mut self) {
        if self.tabs.is_empty() {
            self.tabs.push(Tab { path: self.path.clone(), cursor: self.cursor_entry().map(|e| e.name.clone()) });
        }
        self.tabs.push(Tab { path: self.path.clone(), cursor: self.cursor_entry().map(|e| e.name.clone()) });
        self.tab = self.tabs.len() - 1;
    }
    pub fn close_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.tabs.remove(self.tab);
            self.tab = self.tab.min(self.tabs.len() - 1);
            let t = self.tabs[self.tab].clone();
            self.goto(&t.path, t.cursor.as_deref());
            if self.tabs.len() == 1 {
                self.tabs.clear();
            }
        }
    }
    pub fn switch_tab(&mut self, delta: i64) {
        if self.tabs.len() > 1 {
            self.tabs[self.tab] = Tab { path: self.path.clone(), cursor: self.cursor_entry().map(|e| e.name.clone()) };
            let n = self.tabs.len() as i64;
            self.tab = ((self.tab as i64 + delta).rem_euclid(n)) as usize;
            let t = self.tabs[self.tab].clone();
            self.goto(&t.path, t.cursor.as_deref());
        }
    }

    // ------------------------------------------------------------------ drawing
    pub fn ui(&mut self, ui: &mut Ui, rect: Rect, active: bool, icons: &Icons, right_click_selects: bool, drives: &[fs::Drive]) -> Vec<PanelEvent> {
        let mut ev = Vec::new();
        let p = ui.painter_at(rect);
        let id = ui.id().with(rect.left() as i32);

        // --- drive row: [-c-] v   123 456 k of 234 567 k free      [\] [..]
        let drive_row = Rect::from_min_size(rect.min, Vec2::new(rect.width(), DRIVE_ROW_H));
        xp::fill(&p, drive_row, xp::FACE);
        let combo = Rect::from_min_size(drive_row.min + Vec2::new(0.0, 2.0), Vec2::new(46.0, 20.0));
        xp::sunken(&p, combo);
        let drv = drives.iter().filter(|d| self.path.starts_with(&d.root)).max_by_key(|d| d.root.as_os_str().len());
        let (dtex, dletter) = match drv {
            Some(d) => (drive_icon(icons, d.kind), d.letter.clone()),
            None => (&icons.drive_hdd, "?".to_string()),
        };
        self.drive_label = dletter.clone();
        icons::draw(&p, dtex, combo.min + Vec2::new(3.0, 2.0));
        xp::text(&p, combo.min + Vec2::new(21.0, 3.0), &format!("[{}]", dletter), xp::TEXT);
        let arrow = Rect::from_min_size(Pos2::new(combo.right() - 16.0, combo.top() + 1.0), Vec2::new(15.0, 18.0));
        let ar = ui.interact(combo, id.with("drive"), Sense::CLICK);
        xp::scrollbar_button(&p, arrow, ar.hovered(), ar.is_pointer_button_down_on());
        xp::arrow_glyph(&p, arrow.center(), xp::Dir::Down, xp::rgb(0x4D, 0x61, 0x85));
        if ar.clicked() {
            ev.push(PanelEvent::DriveMenu);
        }
        if let Some((free, total)) = self.free {
            xp::text(&p, Pos2::new(combo.right() + 8.0, combo.top() + 3.0), &format!("{} of {} free", fs::fmt_k(free), fs::fmt_k(total)), xp::TEXT);
        }
        // "\" and ".." buttons at the right end
        let b2 = Rect::from_min_size(Pos2::new(drive_row.right() - 22.0, drive_row.top() + 2.0), Vec2::new(20.0, 20.0));
        let b1 = Rect::from_min_size(Pos2::new(drive_row.right() - 44.0, drive_row.top() + 2.0), Vec2::new(20.0, 20.0));
        for (r, label, which) in [(b1, "\\", 0), (b2, "..", 1)] {
            let resp = ui.interact(r, id.with(("navbtn", which)), Sense::CLICK);
            xp::button_face(&p, r, resp.hovered(), resp.is_pointer_button_down_on(), false, true);
            p.text(r.center(), Align2::CENTER_CENTER, label, xp::font(), xp::TEXT);
            if resp.clicked() {
                if which == 0 {
                    self.root();
                } else {
                    self.parent();
                }
                ev.push(PanelEvent::Activate);
            }
        }

        // --- path row
        let path_row = Rect::from_min_size(Pos2::new(rect.left(), drive_row.bottom()), Vec2::new(rect.width(), PATH_ROW_H));
        let (bg, fg) = if active { (xp::HIGHLIGHT, xp::HIGHLIGHT_TEXT) } else { (xp::FACE, xp::TEXT) };
        xp::fill(&p, path_row, bg);
        let title = match &self.list_title {
            Some(t) => t.clone(),
            None => {
                let d = fs::display(&self.path);
                let sep = if d.ends_with(fs::sep()) { "" } else { &fs::sep().to_string() };
                format!("{}{}{}{}", d, sep, self.filter, if self.branch { "  (branch view)" } else { "" })
            }
        };
        let shown = xp::ellipsize_path(&p, &title, path_row.width() - 8.0, xp::font());
        xp::text(&p, path_row.min + Vec2::new(3.0, 2.0), &shown, fg);
        let pr = ui.interact(path_row, id.with("path"), Sense::CLICK);
        if pr.clicked() {
            ev.push(PanelEvent::Activate);
        }

        // --- tabs (only when more than one)
        let mut top = path_row.bottom();
        if self.tabs.len() > 1 {
            let tabs_row = Rect::from_min_size(Pos2::new(rect.left(), top), Vec2::new(rect.width(), 20.0));
            xp::fill(&p, tabs_row, xp::FACE);
            let mut x = tabs_row.left() + 2.0;
            let mut clicked_tab = None;
            let tabs = self.tabs.clone();
            for (i, t) in tabs.iter().enumerate() {
                let name = if i == self.tab { self.path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| fs::display(&self.path)) } else { t.path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| fs::display(&t.path)) };
                let w = xp::text_width(&p, &name) + 16.0;
                let r = Rect::from_min_size(Pos2::new(x, tabs_row.top() + 2.0), Vec2::new(w, 18.0));
                let resp = ui.interact(r, id.with(("tab", i)), Sense::CLICK);
                if i == self.tab {
                    xp::fill(&p, r, xp::WINDOW);
                    xp::frame(&p, r, xp::SHADOW);
                    px_line(&p, r.left() + 1.0, r.bottom() - 1.0, r.width() - 2.0, xp::WINDOW);
                } else {
                    xp::fill(&p, r.shrink2(Vec2::new(0.0, 2.0)), xp::LIGHT);
                    xp::frame(&p, r.shrink2(Vec2::new(0.0, 2.0)), xp::SHADOW);
                }
                p.text(r.center(), Align2::CENTER_CENTER, &name, xp::font(), xp::TEXT);
                if resp.clicked() && i != self.tab {
                    clicked_tab = Some(i);
                }
                x += w + 2.0;
            }
            if let Some(i) = clicked_tab {
                let d = i as i64 - self.tab as i64;
                self.switch_tab(d);
                ev.push(PanelEvent::Activate);
            }
            px_line(&p, tabs_row.left(), tabs_row.bottom() - 1.0, tabs_row.width(), xp::SHADOW);
            top = tabs_row.bottom();
        }

        // --- list area with header + scrollbar + status
        let status = Rect::from_min_size(Pos2::new(rect.left(), rect.bottom() - STATUS_H), Vec2::new(rect.width(), STATUS_H));
        let list_outer = Rect::from_min_max(Pos2::new(rect.left(), top), Pos2::new(rect.right(), status.top()));
        xp::sunken(&p, list_outer);
        let inner = list_outer.shrink(1.0);
        match self.view {
            View::Full => self.ui_full(ui, inner, active, icons, right_click_selects, id, &mut ev),
            View::Brief => self.ui_brief(ui, inner, active, icons, right_click_selects, id, &mut ev),
        }

        // --- status line
        xp::fill(&p, status, xp::FACE);
        let st = self.status_text();
        xp::text(&p, status.min + Vec2::new(3.0, 2.0), &st, xp::TEXT);
        if let Some(q) = &self.quick {
            let w = xp::text_width(&p, q) + 12.0;
            let qr = Rect::from_min_size(Pos2::new(status.left() + 2.0, status.top()), Vec2::new(w.max(60.0), STATUS_H - 1.0));
            xp::sunken(&p, qr);
            xp::text(&p, qr.min + Vec2::new(3.0, 2.0), q, xp::TEXT);
            px_line(&p, qr.left() + 3.0 + xp::text_width(&p, q) + 1.0, qr.top() + 3.0, 1.0, xp::TEXT);
            p.rect_filled(Rect::from_min_size(Pos2::new(qr.left() + 3.0 + xp::text_width(&p, q) + 1.0, qr.top() + 3.0), Vec2::new(1.0, 11.0)), 0.0, xp::TEXT);
        }
        ev
    }

    fn ui_full(&mut self, ui: &mut Ui, inner: Rect, active: bool, icons: &Icons, right_click_selects: bool, id: Id, ev: &mut Vec<PanelEvent>) {
        let p = ui.painter_at(inner);
        let header = Rect::from_min_size(inner.min, Vec2::new(inner.width() - xp::SB_W, HEADER_H));
        let list = Rect::from_min_max(Pos2::new(inner.left(), header.bottom()), Pos2::new(inner.right() - xp::SB_W, inner.bottom()));
        let sb = Rect::from_min_max(Pos2::new(inner.right() - xp::SB_W, inner.top()), inner.right_bottom());
        self.rows_visible = (list.height() / ROW_H).floor().max(1.0) as usize;
        self.ensure_visible();

        // columns
        let w_attr = 34.0;
        let w_date = 100.0;
        let w_size = 70.0;
        let w_ext = 40.0;
        let w_name = (header.width() - w_attr - w_date - w_size - w_ext).max(40.0);
        let cols: [(&str, f32, Option<SortBy>); 5] = [("Name", w_name, Some(SortBy::Name)), ("Ext", w_ext, Some(SortBy::Ext)), ("Size", w_size, Some(SortBy::Size)), ("Date", w_date, Some(SortBy::Date)), ("Attr", w_attr, None)];
        let mut x = header.left();
        let mut col_x = [0.0f32; 6];
        for (i, (label, w, sort)) in cols.iter().enumerate() {
            col_x[i] = x;
            let r = Rect::from_min_size(Pos2::new(x, header.top()), Vec2::new(*w, HEADER_H));
            let resp = ui.interact(r, id.with(("hdr", i)), Sense::CLICK);
            xp::header_cell(&p, r, resp.hovered(), resp.is_pointer_button_down_on());
            let align = if i == 2 { Align2::RIGHT_CENTER } else { Align2::LEFT_CENTER };
            let tpos = if i == 2 { Pos2::new(r.right() - 6.0, r.center().y) } else { Pos2::new(r.left() + 4.0, r.center().y) };
            p.text(tpos, align, label, xp::font(), xp::TEXT);
            if Some(self.sort) == *sort {
                let ax = if i == 2 { r.left() + 10.0 } else { r.right() - 12.0 };
                // TC: name/ext ascending = arrow down; size/date default (descending) = arrow up
                let up = matches!(self.sort, SortBy::Size | SortBy::Date) != self.reverse;
                xp::sort_arrow(&p, Pos2::new(ax, r.top() + 7.0), up, xp::DKSHADOW);
            }
            if resp.clicked() {
                if let Some(s) = sort {
                    if self.sort == *s {
                        self.reverse = !self.reverse;
                    } else {
                        self.sort = *s;
                        self.reverse = false;
                    }
                    self.reload();
                }
                ev.push(PanelEvent::Activate);
            }
            x += w;
        }
        col_x[5] = x;
        xp::header_bottom(&p, header);

        // rows
        let lp = ui.painter_at(list);
        let rows = self.rows_visible;
        for row in 0..rows {
            let i = self.scroll + row;
            if i >= self.entries.len() {
                break;
            }
            let y = list.top() + row as f32 * ROW_H;
            let rr = Rect::from_min_size(Pos2::new(list.left(), y), Vec2::new(list.width(), ROW_H));
            let e = &self.entries[i];
            let selected = self.selected.contains(&e.name);
            let color = if selected { xp::MARK } else { xp::TEXT };
            let tex = if e.is_up { &icons.folder_up } else if e.is_dir { &icons.folder } else { icons.for_ext(&e.ext_lower()) };
            icons::draw(&lp, tex, Pos2::new(col_x[0] + 2.0, y));
            let name = if e.is_dir { format!("[{}]", e.name) } else { e.stem().to_string() };
            let name_clip = Rect::from_min_size(Pos2::new(col_x[0] + 20.0, y), Vec2::new(col_x[1] - col_x[0] - 22.0, ROW_H));
            lp.with_clip_rect(name_clip).text(Pos2::new(col_x[0] + 20.0, y + 2.0), Align2::LEFT_TOP, &name, xp::font(), color);
            let ext_clip = Rect::from_min_size(Pos2::new(col_x[1] + 2.0, y), Vec2::new(col_x[2] - col_x[1] - 4.0, ROW_H));
            lp.with_clip_rect(ext_clip).text(Pos2::new(col_x[1] + 2.0, y + 2.0), Align2::LEFT_TOP, e.ext(), xp::font(), color);
            let size = if e.is_up { String::new() } else if e.is_dir { e.dir_size.map(fs::fmt_num).unwrap_or_else(|| "<DIR>".into()) } else { fs::fmt_num(e.size) };
            lp.text(Pos2::new(col_x[3] - 4.0, y + 2.0), Align2::RIGHT_TOP, &size, xp::font(), color);
            lp.text(Pos2::new(col_x[3] + 2.0, y + 2.0), Align2::LEFT_TOP, &fs::fmt_date(&e.modified), xp::font(), color);
            lp.text(Pos2::new(col_x[4] + 2.0, y + 2.0), Align2::LEFT_TOP, &fs::attr_string(e), xp::font(), color);
            if active && i == self.cursor {
                xp::focus_rect(&lp, rr, xp::TEXT);
            }
            let resp = ui.interact(rr, id.with(("row", i)), Sense::CLICK);
            self.row_mouse(ui, &resp, i, right_click_selects, ev);
        }
        self.list_mouse(ui, list, id, ev);
        let max = self.entries.len();
        if let Some(s) = scrollbar(ui, &p, sb, id.with("vsb"), self.scroll, rows, max, true) {
            self.scroll = s;
        }
    }

    fn ui_brief(&mut self, ui: &mut Ui, inner: Rect, active: bool, icons: &Icons, right_click_selects: bool, id: Id, ev: &mut Vec<PanelEvent>) {
        let p = ui.painter_at(inner);
        let list = Rect::from_min_max(inner.min, Pos2::new(inner.right(), inner.bottom() - xp::SB_W));
        let sb = Rect::from_min_max(Pos2::new(inner.left(), inner.bottom() - xp::SB_W), inner.right_bottom());
        self.rows_visible = (list.height() / ROW_H).floor().max(1.0) as usize;
        let col_w = (list.width() / 3.0).floor().max(100.0);
        self.cols_visible = (list.width() / col_w).floor().max(1.0) as usize;
        self.ensure_visible();
        let rows = self.rows_visible;
        let cols = self.cols_visible;
        let first_col = self.scroll / rows;
        let lp = ui.painter_at(list);
        for c in 0..cols {
            let x = list.left() + c as f32 * col_w;
            if c > 0 {
                px_line(&lp, x - 1.0, list.top(), 1.0, xp::rgb(0xE0, 0xE0, 0xE0));
                lp.rect_filled(Rect::from_min_size(Pos2::new(x - 1.0, list.top()), Vec2::new(1.0, list.height())), 0.0, xp::rgb(0xE8, 0xE8, 0xE8));
            }
            for row in 0..rows {
                let i = (first_col + c) * rows + row;
                if i >= self.entries.len() {
                    break;
                }
                let y = list.top() + row as f32 * ROW_H;
                let rr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(col_w - 1.0, ROW_H));
                let e = &self.entries[i];
                let selected = self.selected.contains(&e.name);
                let color = if selected { xp::MARK } else { xp::TEXT };
                let tex = if e.is_up { &icons.folder_up } else if e.is_dir { &icons.folder } else { icons.for_ext(&e.ext_lower()) };
                icons::draw(&lp, tex, Pos2::new(x + 2.0, y));
                let name = if e.is_dir { format!("[{}]", e.name) } else { e.name.clone() };
                let clip = Rect::from_min_size(Pos2::new(x + 20.0, y), Vec2::new(col_w - 22.0, ROW_H));
                lp.with_clip_rect(clip).text(Pos2::new(x + 20.0, y + 2.0), Align2::LEFT_TOP, &name, xp::font(), color);
                if active && i == self.cursor {
                    xp::focus_rect(&lp, rr, xp::TEXT);
                }
                let resp = ui.interact(rr, id.with(("row", i)), Sense::CLICK);
                self.row_mouse(ui, &resp, i, right_click_selects, ev);
            }
        }
        self.list_mouse(ui, list, id, ev);
        let total_cols = self.entries.len().div_ceil(rows);
        if let Some(s) = scrollbar(ui, &p, sb, id.with("hsb"), first_col, cols, total_cols, false) {
            self.scroll = s * rows;
        }
    }

    fn row_mouse(&mut self, ui: &Ui, resp: &egui::Response, i: usize, right_click_selects: bool, ev: &mut Vec<PanelEvent>) {
        let mods = ui.input(|inp| inp.modifiers);
        if resp.double_clicked() {
            self.cursor = i;
            self.open_cursor(ev);
            ev.push(PanelEvent::Activate);
        } else if resp.clicked() {
            if mods.ctrl || (!right_click_selects && mods.shift && self.sel_anchor.is_none()) {
                self.toggle(i);
            } else if mods.shift {
                let a = self.sel_anchor.unwrap_or(self.cursor);
                let (lo, hi) = if a <= i { (a, i) } else { (i, a) };
                for k in lo..=hi {
                    self.set_selected(k, true);
                }
            } else {
                self.sel_anchor = Some(i);
            }
            self.cursor = i;
            self.quick = None;
            ev.push(PanelEvent::Activate);
        } else if resp.secondary_clicked() {
            self.cursor = i;
            if right_click_selects {
                self.toggle(i);
            } else {
                ev.push(PanelEvent::ContextMenu(i));
            }
            ev.push(PanelEvent::Activate);
        }
    }

    fn list_mouse(&mut self, ui: &Ui, list: Rect, id: Id, ev: &mut Vec<PanelEvent>) {
        let resp = ui.interact(list, id.with("listbg"), Sense::CLICK);
        if resp.clicked() || resp.secondary_clicked() {
            ev.push(PanelEvent::Activate);
        }
        if resp.hovered() {
            let dy = wheel_dy(ui);
            if dy.abs() > 0.0 {
                let step = if dy > 0.0 { -3i64 } else { 3 };
                match self.view {
                    View::Full => {
                        let max = self.entries.len().saturating_sub(self.rows_visible) as i64;
                        self.scroll = (self.scroll as i64 + step).clamp(0, max.max(0)) as usize;
                    }
                    View::Brief => {
                        let rows = self.rows_visible.max(1);
                        let total_cols = self.entries.len().div_ceil(rows);
                        let max = total_cols.saturating_sub(self.cols_visible) as i64;
                        let col = (self.scroll / rows) as i64 + step.signum();
                        self.scroll = col.clamp(0, max.max(0)) as usize * rows;
                    }
                }
            }
        }
    }

    /// Enter on the entry under the cursor.
    pub fn open_cursor(&mut self, ev: &mut Vec<PanelEvent>) {
        let Some(e) = self.cursor_entry().cloned() else { return };
        if e.is_up {
            self.parent();
        } else if e.is_dir {
            self.goto(&e.path, None);
        } else {
            ev.push(PanelEvent::OpenFile(e.path));
        }
    }
}

fn px_line(p: &egui::Painter, x: f32, y: f32, w: f32, c: Color32) {
    p.rect_filled(Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, 1.0)), 0.0, c);
}

pub fn drive_icon(icons: &Icons, kind: fs::DriveKind) -> &egui::TextureHandle {
    match kind {
        fs::DriveKind::Floppy => &icons.drive_floppy,
        fs::DriveKind::Fixed => &icons.drive_hdd,
        fs::DriveKind::Removable => &icons.drive_removable,
        fs::DriveKind::Cdrom => &icons.drive_cd,
        fs::DriveKind::Network => &icons.drive_net,
        fs::DriveKind::Home => &icons.drive_home,
    }
}

/// XP-styled scrollbar. `pos` is the first visible item, `page` the number visible, `total` the item count.
/// Returns a new `pos` when the user scrolled with it.
pub fn scrollbar(ui: &mut Ui, p: &egui::Painter, r: Rect, id: Id, pos: usize, page: usize, total: usize, vertical: bool) -> Option<usize> {
    xp::scrollbar_track(p, r);
    let len = if vertical { r.height() } else { r.width() };
    let btn = xp::SB_W.min(len / 2.0);
    let (b1, b2, track) = if vertical {
        (
            Rect::from_min_size(r.min, Vec2::new(r.width(), btn)),
            Rect::from_min_size(Pos2::new(r.left(), r.bottom() - btn), Vec2::new(r.width(), btn)),
            Rect::from_min_max(Pos2::new(r.left(), r.top() + btn), Pos2::new(r.right(), r.bottom() - btn)),
        )
    } else {
        (
            Rect::from_min_size(r.min, Vec2::new(btn, r.height())),
            Rect::from_min_size(Pos2::new(r.right() - btn, r.top()), Vec2::new(btn, r.height())),
            Rect::from_min_max(Pos2::new(r.left() + btn, r.top()), Pos2::new(r.right() - btn, r.bottom())),
        )
    };
    let max_pos = total.saturating_sub(page);
    let mut out = None;
    let r1 = ui.interact(b1, id.with("b1"), Sense::CLICK);
    let r2 = ui.interact(b2, id.with("b2"), Sense::CLICK);
    xp::scrollbar_button(p, b1, r1.hovered(), r1.is_pointer_button_down_on());
    xp::scrollbar_button(p, b2, r2.hovered(), r2.is_pointer_button_down_on());
    let glyph = xp::rgb(0x4D, 0x61, 0x85);
    xp::arrow_glyph(p, b1.center(), if vertical { xp::Dir::Up } else { xp::Dir::Left }, glyph);
    xp::arrow_glyph(p, b2.center(), if vertical { xp::Dir::Down } else { xp::Dir::Right }, glyph);
    if r1.clicked() && pos > 0 {
        out = Some(pos - 1);
    }
    if r2.clicked() && pos < max_pos {
        out = Some(pos + 1);
    }
    if total > page && page > 0 {
        let tlen = if vertical { track.height() } else { track.width() };
        let thumb_len = (tlen * page as f32 / total as f32).max(12.0).min(tlen);
        let travel = tlen - thumb_len;
        let frac = if max_pos > 0 { pos as f32 / max_pos as f32 } else { 0.0 };
        let off = (travel * frac).round();
        let thumb = if vertical {
            Rect::from_min_size(Pos2::new(track.left(), track.top() + off), Vec2::new(track.width(), thumb_len))
        } else {
            Rect::from_min_size(Pos2::new(track.left() + off, track.top()), Vec2::new(thumb_len, track.height()))
        };
        let tr = ui.interact(thumb, id.with("thumb"), (Sense::CLICK | Sense::DRAG));
        xp::scrollbar_thumb(p, thumb, tr.hovered(), tr.dragged());
        if tr.drag_started() {
            ui.data_mut(|d| d.insert_temp(id.with("drag0"), (pos as f32, off)));
        }
        if tr.dragged() {
            if let Some(ptr) = ui.input(|i| i.pointer.interact_pos()) {
                let (start_pos, start_off) = ui.data(|d| d.get_temp::<(f32, f32)>(id.with("drag0"))).unwrap_or((pos as f32, off));
                let origin = ui.data(|d| d.get_temp::<Pos2>(id.with("dragp"))).unwrap_or(ptr);
                if ui.data(|d| d.get_temp::<Pos2>(id.with("dragp"))).is_none() {
                    ui.data_mut(|d| d.insert_temp(id.with("dragp"), ptr));
                }
                let delta = if vertical { ptr.y - origin.y } else { ptr.x - origin.x };
                let _ = start_off;
                if travel > 0.0 {
                    let np = (start_pos + delta / travel * max_pos as f32).round().clamp(0.0, max_pos as f32) as usize;
                    if np != pos {
                        out = Some(np);
                    }
                }
            }
        }
        if tr.drag_stopped() {
            ui.data_mut(|d| d.remove::<Pos2>(id.with("dragp")));
        }
        // page up/down by clicking the track
        let trk = ui.interact(track, id.with("track"), Sense::CLICK);
        if trk.clicked() && !tr.hovered() {
            if let Some(ptr) = ui.input(|i| i.pointer.interact_pos()) {
                let before = if vertical { ptr.y < thumb.top() } else { ptr.x < thumb.left() };
                out = Some(if before { pos.saturating_sub(page) } else { (pos + page).min(max_pos) });
            }
        }
    }
    out
}

/// Mouse-wheel movement this frame (positive = wheel up), summed from the raw events.
pub fn wheel_dy(ui: &Ui) -> f32 {
    ui.input(|i| {
        i.events
            .iter()
            .map(|e| match e {
                egui::Event::MouseWheel { delta, .. } => delta.y,
                _ => 0.0,
            })
            .sum()
    })
}
