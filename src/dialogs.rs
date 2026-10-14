//! Modal dialogs, drawn as XP windows on top of the main window.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use egui::{Align2, Color32, Pos2, Rect, Sense, Ui, Vec2};

use crate::config::QuickSearch;
use crate::fs;
use crate::ops::{OpHandle, OpKind, OpRequest, Query, Reply};
use crate::xp;

#[derive(Clone, Debug)]
pub enum Action {
    Close,
    StartOp(OpRequest),
    MkDir(String),
    Rename(PathBuf, String),
    Reply(Reply),
    CancelOp,
    GotoFile(PathBuf),
    FeedToListbox(Vec<PathBuf>, String),
    SelectGroup { mask: String, on: bool },
    SetFilter(String),
    ApplyOptions(Options),
    EditNew(String),
    OpenUrl(String),
}

#[derive(Clone, Debug)]
pub struct Options {
    pub nag: bool,
    pub lynx: bool,
    pub right_click_selects: bool,
    pub show_hidden: bool,
    pub quick_search: QuickSearch,
    pub editor: String,
}

pub struct FindState {
    pub mask: String,
    pub dir: String,
    pub text: String,
    pub results: Vec<PathBuf>,
    pub cursor: usize,
    pub scroll: usize,
    pub rx: Option<Receiver<Option<PathBuf>>>,
    pub cancel: Arc<AtomicBool>,
    pub running: bool,
    pub searched: usize,
}

pub enum Dialog {
    /// F5 / F6 / Shift+F5
    Copy { prompt: String, target: String, mv: bool, sources: Vec<PathBuf>, cwd: PathBuf },
    Delete { prompt: String, sources: Vec<PathBuf> },
    MkDir { name: String },
    Rename { old: PathBuf, name: String },
    Message { title: String, text: String, error: bool },
    Overwrite { q: Query },
    Progress,
    Properties { name: String, lines: Vec<(String, String)> },
    SelectGroup { mask: String, on: bool },
    Pack { archive: String, sources: Vec<PathBuf> },
    Unpack { target: String, archives: Vec<PathBuf> },
    Find(FindState),
    About,
    Nag { number: u8, tries: u32 },
    Text { title: String, text: String },
    Options(Options),
    CustomFilter { mask: String },
    EditNew { name: String },
    CalcSpace { lines: Vec<(String, String)> },
}

/// Read Enter/Esc before any TextEdit can consume them. On the frame a dialog opens the key
/// that opened it is still "pressed", so a fresh dialog ignores both.
fn keys(ui: &Ui, fresh: bool) -> (bool, bool) {
    if fresh {
        return (false, false);
    }
    ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)))
}

/// Draw a dialog window centred on the screen. Returns the client rect.
pub fn window(ui: &mut Ui, screen: Rect, title: &str, size: Vec2) -> Rect {
    let r = xp::round_rect(Rect::from_center_size(screen.center(), size));
    let p = ui.painter().clone();
    // dim nothing (XP didn't); just paint the window with a thin dark-blue frame
    p.rect_filled(r.expand(1.0), 0.0, xp::FRAME_DARK);
    xp::fill(&p, r, xp::FACE);
    xp::frame(&p, r, xp::FRAME);
    let cap = Rect::from_min_size(r.min + Vec2::new(1.0, 1.0), Vec2::new(r.width() - 2.0, 24.0));
    xp::caption(&p, cap, true);
    p.text(cap.min + Vec2::new(8.0, 5.0), Align2::LEFT_TOP, title, xp::font_caption(), xp::rgb(0x0A, 0x24, 0x6A));
    p.text(cap.min + Vec2::new(7.0, 4.0), Align2::LEFT_TOP, title, xp::font_caption(), Color32::WHITE);
    // block clicks from reaching the panels below
    let _ = ui.interact(r, ui.id().with("dlg-modal"), (Sense::CLICK | Sense::DRAG));
    Rect::from_min_max(Pos2::new(r.left() + 1.0, cap.bottom()), r.max - Vec2::new(1.0, 1.0))
}

pub fn button(ui: &mut Ui, r: Rect, label: &str, default: bool, enabled: bool) -> bool {
    let resp = ui.interact(r, ui.id().with(("dbtn", label, r.min.x as i32, r.min.y as i32)), Sense::CLICK);
    let p = ui.painter().clone();
    xp::button_face(&p, r, resp.hovered() && enabled, resp.is_pointer_button_down_on() && enabled, default, enabled);
    xp::mnemonic_text(&p, r.center(), Align2::CENTER_CENTER, label, xp::font(), if enabled { xp::TEXT } else { xp::GRAYTEXT });
    let (text, under) = xp::strip_mnemonic(label);
    let mnemonic = under.and_then(|u| text.chars().nth(u)).map(|c| c.to_ascii_lowercase());
    let fresh = ui.data(|d| d.get_temp::<bool>(egui::Id::new("dlg-fresh"))).unwrap_or(false);
    let key_hit = enabled && !fresh && mnemonic.map(|c| mnemonic_pressed(ui, c)).unwrap_or(false);
    (resp.clicked() && enabled) || key_hit
}

/// The underlined letter of a button: Alt+letter always, the bare letter when no edit box has focus.
fn mnemonic_pressed(ui: &Ui, c: char) -> bool {
    let text_focus = ui.memory(|m| m.focused().is_some());
    ui.input(|i| {
        i.events.iter().any(|e| match e {
            egui::Event::Key { key, pressed: true, modifiers, .. } => {
                let name = key.name();
                name.chars().count() == 1 && name.chars().next().map(|k| k.to_ascii_lowercase()) == Some(c) && (modifiers.alt || (!text_focus && !modifiers.ctrl))
            }
            _ => false,
        })
    })
}

/// Single-line edit box. `salt` keeps each box's cursor state apart; `select` sets the
/// selection (char range) — used on a dialog's first frame, like Windows selecting the text.
pub fn edit(ui: &mut Ui, r: Rect, text: &mut String, focus: bool, salt: &str, select: Option<(usize, usize)>) -> egui::Response {
    xp::sunken(ui.painter(), r);
    let id = ui.id().with(("xp-edit", salt));
    // the selection can only be applied once the box has focus (an unfocused TextEdit drops it),
    // so remember the wish on the dialog's first frame and apply it when focus has arrived
    let pending = id.with("sel-pending");
    let fresh = ui.data(|d| d.get_temp::<bool>(egui::Id::new("dlg-fresh"))).unwrap_or(false);
    if fresh && select.is_some() {
        ui.data_mut(|d| d.insert_temp(pending, true));
    }
    let mut cui = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(Vec2::new(3.0, 2.0))));
    let te = egui::TextEdit::singleline(text).id(id).frame(egui::Frame::NONE).font(xp::font()).text_color(xp::TEXT).desired_width(r.width() - 8.0);
    let resp = cui.add(te);
    if resp.has_focus() && ui.data(|d| d.get_temp::<bool>(pending)).unwrap_or(false) {
        if let Some((a, b)) = select {
            let n = text.chars().count();
            let mut st = egui::widgets::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
            st.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(a.min(n)), egui::text::CCursor::new(b.min(n)))));
            st.store(ui.ctx(), id);
            ui.data_mut(|d| d.remove::<bool>(pending));
            ui.ctx().request_repaint();
        }
    }
    if focus && !resp.has_focus() && ui.memory(|m| m.focused().is_none()) {
        resp.request_focus();
    }
    resp
}

pub fn check(ui: &mut Ui, pos: Pos2, label: &str, v: &mut bool) {
    let w = xp::text_width(ui.painter(), label) + 20.0;
    let r = Rect::from_min_size(pos, Vec2::new(w, 14.0));
    let resp = ui.interact(r, ui.id().with(("chk", label)), Sense::CLICK);
    if resp.clicked() {
        *v = !*v;
    }
    let p = ui.painter().clone();
    xp::checkbox(&p, Rect::from_min_size(pos, Vec2::splat(13.0)), *v, resp.hovered());
    xp::mnemonic_text(&p, Pos2::new(pos.x + 18.0, pos.y + 7.0), Align2::LEFT_CENTER, label, xp::font(), xp::TEXT);
}

pub fn radio(ui: &mut Ui, pos: Pos2, label: &str, on: bool) -> bool {
    let w = xp::text_width(ui.painter(), label) + 20.0;
    let r = Rect::from_min_size(pos, Vec2::new(w, 14.0));
    let resp = ui.interact(r, ui.id().with(("rad", label)), Sense::CLICK);
    let p = ui.painter().clone();
    xp::radio(&p, Rect::from_min_size(pos, Vec2::splat(13.0)), on);
    xp::mnemonic_text(&p, Pos2::new(pos.x + 18.0, pos.y + 7.0), Align2::LEFT_CENTER, label, xp::font(), xp::TEXT);
    resp.clicked()
}

fn label(ui: &Ui, pos: Pos2, s: &str) {
    xp::text(ui.painter(), pos, s, xp::TEXT);
}

fn wrap_text(ui: &Ui, pos: Pos2, w: f32, s: &str) -> f32 {
    let galley = ui.painter().layout(s.to_string(), xp::font(), xp::TEXT, w);
    let h = galley.size().y;
    ui.painter().galley(pos, galley, xp::TEXT);
    h
}

fn two_buttons(ui: &mut Ui, client: Rect, a: &str, b: &str) -> (bool, bool) {
    let y = client.bottom() - 32.0;
    let ra = Rect::from_min_size(Pos2::new(client.right() - 170.0, y), Vec2::new(75.0, 23.0));
    let rb = Rect::from_min_size(Pos2::new(client.right() - 85.0, y), Vec2::new(75.0, 23.0));
    (button(ui, ra, a, true, true), button(ui, rb, b, false, true))
}

impl Dialog {
    pub fn is_progress(&self) -> bool {
        matches!(self, Dialog::Progress)
    }

    pub fn show(&mut self, ui: &mut Ui, screen: Rect, op: Option<&OpHandle>, fresh: bool) -> Option<Action> {
        let (enter, esc) = keys(ui, fresh);
        ui.data_mut(|d| d.insert_temp(egui::Id::new("dlg-fresh"), fresh));
        let all = |t: &str| Some((0, t.chars().count()));
        let stem = |t: &str| Some((0, t.rfind('.').filter(|&i| i > 0).map(|i| t[..i].chars().count()).unwrap_or(t.chars().count())));
        match self {
            Dialog::Copy { prompt, target, mv, sources, cwd } => {
                let client = window(ui, screen, if *mv { "Move/Rename" } else { "Copy" }, Vec2::new(520.0, 150.0));
                label(ui, client.min + Vec2::new(12.0, 14.0), prompt);
                let er = Rect::from_min_size(client.min + Vec2::new(12.0, 34.0), Vec2::new(client.width() - 24.0, 21.0));
                let sel = all(target);
                edit(ui, er, target, true, "copy", sel);
                let (ok, cancel) = two_buttons(ui, client, "OK", "Cancel");
                let tree = Rect::from_min_size(Pos2::new(client.left() + 12.0, client.bottom() - 32.0), Vec2::new(75.0, 23.0));
                let _ = button(ui, tree, "&Tree", false, false);
                let opts = Rect::from_min_size(Pos2::new(client.left() + 97.0, client.bottom() - 32.0), Vec2::new(75.0, 23.0));
                let _ = button(ui, opts, "&Options", false, false);
                if ok || enter {
                    let (dir, rename) = resolve_target(target, cwd, sources.len());
                    return Some(Action::StartOp(OpRequest { kind: if *mv { OpKind::Move } else { OpKind::Copy }, sources: sources.clone(), target: dir, rename_to: rename }));
                }
                if cancel || esc {
                    return Some(Action::Close);
                }
            }
            Dialog::Delete { prompt, sources } => {
                let client = window(ui, screen, "Delete", Vec2::new(430.0, 130.0));
                wrap_text(ui, client.min + Vec2::new(14.0, 16.0), client.width() - 28.0, prompt);
                let y = client.bottom() - 32.0;
                let yes = Rect::from_min_size(Pos2::new(client.center().x - 80.0, y), Vec2::new(75.0, 23.0));
                let no = Rect::from_min_size(Pos2::new(client.center().x + 5.0, y), Vec2::new(75.0, 23.0));
                let a = button(ui, yes, "&Yes", true, true);
                let b = button(ui, no, "&No", false, true);
                if a || enter {
                    return Some(Action::StartOp(OpRequest { kind: OpKind::Delete, sources: sources.clone(), target: PathBuf::new(), rename_to: None }));
                }
                if b || esc {
                    return Some(Action::Close);
                }
            }
            Dialog::MkDir { name } => {
                let client = window(ui, screen, "Create new directory", Vec2::new(420.0, 125.0));
                label(ui, client.min + Vec2::new(12.0, 12.0), "Create new directory:");
                let er = Rect::from_min_size(client.min + Vec2::new(12.0, 30.0), Vec2::new(client.width() - 24.0, 21.0));
                edit(ui, er, name, true, "mkdir", None);
                let (ok, cancel) = two_buttons(ui, client, "OK", "Cancel");
                if (ok || enter) && !name.trim().is_empty() {
                    return Some(Action::MkDir(name.trim().to_string()));
                }
                if cancel || esc {
                    return Some(Action::Close);
                }
            }
            Dialog::Rename { old, name } => {
                let client = window(ui, screen, "Rename", Vec2::new(420.0, 125.0));
                label(ui, client.min + Vec2::new(12.0, 12.0), &format!("Rename '{}' to:", old.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()));
                let er = Rect::from_min_size(client.min + Vec2::new(12.0, 30.0), Vec2::new(client.width() - 24.0, 21.0));
                let sel = stem(name);
                edit(ui, er, name, true, "rename", sel);
                let (ok, cancel) = two_buttons(ui, client, "OK", "Cancel");
                if (ok || enter) && !name.trim().is_empty() {
                    return Some(Action::Rename(old.clone(), name.trim().to_string()));
                }
                if cancel || esc {
                    return Some(Action::Close);
                }
            }
            Dialog::EditNew { name } => {
                let client = window(ui, screen, "Edit", Vec2::new(420.0, 125.0));
                label(ui, client.min + Vec2::new(12.0, 12.0), "Edit file:");
                let er = Rect::from_min_size(client.min + Vec2::new(12.0, 30.0), Vec2::new(client.width() - 24.0, 21.0));
                let sel = stem(name);
                edit(ui, er, name, true, "editnew", sel);
                let (ok, cancel) = two_buttons(ui, client, "OK", "Cancel");
                if (ok || enter) && !name.trim().is_empty() {
                    return Some(Action::EditNew(name.trim().to_string()));
                }
                if cancel || esc {
                    return Some(Action::Close);
                }
            }
            Dialog::CustomFilter { mask } => {
                let client = window(ui, screen, "Define custom view", Vec2::new(420.0, 125.0));
                label(ui, client.min + Vec2::new(12.0, 12.0), "Only show files matching (e.g. *.txt;*.doc):");
                let er = Rect::from_min_size(client.min + Vec2::new(12.0, 30.0), Vec2::new(client.width() - 24.0, 21.0));
                let sel = all(mask);
                edit(ui, er, mask, true, "filter", sel);
                let (ok, cancel) = two_buttons(ui, client, "OK", "Cancel");
                if ok || enter {
                    return Some(Action::SetFilter(if mask.trim().is_empty() { "*.*".into() } else { mask.trim().to_string() }));
                }
                if cancel || esc {
                    return Some(Action::Close);
                }
            }
            Dialog::SelectGroup { mask, on } => {
                let client = window(ui, screen, if *on { "Expand selection" } else { "Shrink selection" }, Vec2::new(380.0, 125.0));
                label(ui, client.min + Vec2::new(12.0, 12.0), if *on { "Select files:" } else { "Unselect files:" });
                let er = Rect::from_min_size(client.min + Vec2::new(12.0, 30.0), Vec2::new(client.width() - 24.0, 21.0));
                let sel = all(mask);
                edit(ui, er, mask, true, "selgroup", sel);
                let (ok, cancel) = two_buttons(ui, client, "OK", "Cancel");
                if ok || enter {
                    return Some(Action::SelectGroup { mask: mask.clone(), on: *on });
                }
                if cancel || esc {
                    return Some(Action::Close);
                }
            }
            Dialog::Message { title, text, error } => {
                let client = window(ui, screen, title, Vec2::new(420.0, 140.0));
                if *error {
                    // red circle with a white X, the XP "error" glyph
                    let c = client.min + Vec2::new(30.0, 36.0);
                    ui.painter().circle_filled(c, 15.0, xp::rgb(0xD8, 0x30, 0x20));
                    ui.painter().circle_stroke(c, 15.0, egui::Stroke::new(1.0, xp::rgb(0x8C, 0x10, 0x10)));
                    ui.painter().text(c, Align2::CENTER_CENTER, "✕", egui::FontId::proportional(18.0), Color32::WHITE);
                } else {
                    let c = client.min + Vec2::new(30.0, 36.0);
                    ui.painter().circle_filled(c, 15.0, xp::rgb(0x31, 0x6A, 0xC5));
                    ui.painter().text(c, Align2::CENTER_CENTER, "i", xp::font_caption(), Color32::WHITE);
                }
                wrap_text(ui, client.min + Vec2::new(60.0, 18.0), client.width() - 72.0, text);
                let y = client.bottom() - 32.0;
                let ok = Rect::from_min_size(Pos2::new(client.center().x - 37.0, y), Vec2::new(75.0, 23.0));
                if button(ui, ok, "OK", true, true) || enter || esc {
                    return Some(Action::Close);
                }
            }
            Dialog::Text { title, text } => {
                let n_lines = text.lines().count() as f32;
                let h = (n_lines * 14.0 + 90.0).min(screen.height() - 40.0);
                let client = window(ui, screen, title, Vec2::new(680.0, h));
                let area = Rect::from_min_max(client.min + Vec2::new(12.0, 12.0), Pos2::new(client.right() - 12.0, client.bottom() - 44.0));
                xp::sunken(ui.painter(), area);
                let tp = ui.painter_at(area.shrink(4.0));
                let mono = egui::FontId::new(11.0, egui::FontFamily::Monospace);
                tp.text(area.min + Vec2::new(6.0, 4.0), Align2::LEFT_TOP, text, mono, xp::TEXT);
                let y = client.bottom() - 32.0;
                let ok = Rect::from_min_size(Pos2::new(client.center().x - 37.0, y), Vec2::new(75.0, 23.0));
                if button(ui, ok, "OK", true, true) || enter || esc {
                    return Some(Action::Close);
                }
            }
            Dialog::Overwrite { q } => {
                let Query::Overwrite { src, dst, src_size, dst_size, src_time, dst_time } = q;
                let client = window(ui, screen, "Overwrite?", Vec2::new(560.0, 190.0));
                let p = ui.painter().clone();
                label(ui, client.min + Vec2::new(12.0, 12.0), "Overwrite file:");
                xp::text(&p, client.min + Vec2::new(12.0, 28.0), &xp::ellipsize_path(&p, &dst.to_string_lossy(), client.width() - 24.0, xp::font()), xp::TEXT);
                label(ui, client.min + Vec2::new(12.0, 44.0), &format!("{} bytes, {}", fs::fmt_num(*dst_size), fs::fmt_date(dst_time)));
                label(ui, client.min + Vec2::new(12.0, 66.0), "With file:");
                xp::text(&p, client.min + Vec2::new(12.0, 82.0), &xp::ellipsize_path(&p, &src.to_string_lossy(), client.width() - 24.0, xp::font()), xp::TEXT);
                label(ui, client.min + Vec2::new(12.0, 98.0), &format!("{} bytes, {}", fs::fmt_num(*src_size), fs::fmt_date(src_time)));
                let y = client.bottom() - 32.0;
                let labels = ["&Overwrite", "&All", "&Skip", "Skip a&ll", "&Cancel"];
                let replies = [Reply::Overwrite, Reply::OverwriteAll, Reply::Skip, Reply::SkipAll, Reply::Cancel];
                let total = labels.len() as f32 * 85.0 - 10.0;
                let mut x = client.center().x - total / 2.0;
                for (i, l) in labels.iter().enumerate() {
                    let r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(75.0, 23.0));
                    if button(ui, r, l, i == 0, true) {
                        return Some(Action::Reply(replies[i]));
                    }
                    x += 85.0;
                }
                if enter {
                    return Some(Action::Reply(Reply::Overwrite));
                }
                if esc {
                    return Some(Action::Reply(Reply::Cancel));
                }
            }
            Dialog::Progress => {
                let title = match op.map(|o| o.kind) {
                    Some(OpKind::Copy) => "Copy",
                    Some(OpKind::Move) => "Move",
                    Some(OpKind::Delete) => "Delete",
                    Some(OpKind::Pack) => "Pack",
                    Some(OpKind::Unpack) => "Unpack",
                    None => "Working",
                };
                let client = window(ui, screen, title, Vec2::new(480.0, 200.0));
                let p = ui.painter().clone();
                if let Some(o) = op {
                    let pr = &o.progress;
                    let w = client.width() - 90.0;
                    xp::text(&p, client.min + Vec2::new(12.0, 14.0), "From:", xp::TEXT);
                    xp::text(&p, client.min + Vec2::new(60.0, 14.0), &xp::ellipsize_path(&p, &pr.from, w, xp::font()), xp::TEXT);
                    xp::text(&p, client.min + Vec2::new(12.0, 32.0), "To:", xp::TEXT);
                    xp::text(&p, client.min + Vec2::new(60.0, 32.0), &xp::ellipsize_path(&p, &pr.to, w, xp::font()), xp::TEXT);
                    let b1 = Rect::from_min_size(client.min + Vec2::new(12.0, 58.0), Vec2::new(client.width() - 24.0, 18.0));
                    xp::progress_bar(&p, b1, pr.file_frac);
                    p.text(b1.center(), Align2::CENTER_CENTER, &format!("{}%", (pr.file_frac * 100.0) as u32), xp::font(), xp::TEXT);
                    let b2 = Rect::from_min_size(client.min + Vec2::new(12.0, 86.0), Vec2::new(client.width() - 24.0, 18.0));
                    xp::progress_bar(&p, b2, pr.total_frac);
                    p.text(b2.center(), Align2::CENTER_CENTER, &format!("{}%", (pr.total_frac * 100.0) as u32), xp::font(), xp::TEXT);
                    xp::text(&p, client.min + Vec2::new(12.0, 110.0), &format!("{} / {} file(s)", pr.done_files, pr.total_files), xp::TEXT);
                }
                let y = client.bottom() - 32.0;
                let c = Rect::from_min_size(Pos2::new(client.center().x - 37.0, y), Vec2::new(75.0, 23.0));
                if button(ui, c, "Cancel", false, true) || esc {
                    return Some(Action::CancelOp);
                }
                ui.ctx().request_repaint();
            }
            Dialog::Properties { name, lines } => {
                let client = window(ui, screen, &format!("{name} Properties"), Vec2::new(380.0, 100.0 + lines.len() as f32 * 20.0));
                let mut y = client.top() + 14.0;
                for (k, v) in lines.iter() {
                    label(ui, Pos2::new(client.left() + 14.0, y), k);
                    let shown = xp::ellipsize(ui.painter(), v, client.width() - 120.0, xp::font());
                    label(ui, Pos2::new(client.left() + 104.0, y), &shown);
                    y += 20.0;
                }
                let yb = client.bottom() - 32.0;
                let ok = Rect::from_min_size(Pos2::new(client.right() - 170.0, yb), Vec2::new(75.0, 23.0));
                let cancel = Rect::from_min_size(Pos2::new(client.right() - 85.0, yb), Vec2::new(75.0, 23.0));
                let a = button(ui, ok, "OK", true, true);
                let b = button(ui, cancel, "Cancel", false, true);
                if a || b || enter || esc {
                    return Some(Action::Close);
                }
            }
            Dialog::CalcSpace { lines } => {
                let client = window(ui, screen, "Calculate occupied space", Vec2::new(380.0, 100.0 + lines.len() as f32 * 20.0));
                let mut y = client.top() + 14.0;
                for (k, v) in lines.iter() {
                    label(ui, Pos2::new(client.left() + 14.0, y), k);
                    label(ui, Pos2::new(client.left() + 150.0, y), v);
                    y += 20.0;
                }
                let yb = client.bottom() - 32.0;
                let ok = Rect::from_min_size(Pos2::new(client.center().x - 37.0, yb), Vec2::new(75.0, 23.0));
                if button(ui, ok, "OK", true, true) || enter || esc {
                    return Some(Action::Close);
                }
            }
            Dialog::Pack { archive, sources } => {
                let client = window(ui, screen, "Pack files", Vec2::new(520.0, 150.0));
                label(ui, client.min + Vec2::new(12.0, 14.0), &format!("Pack {} file(s) to the archive:", sources.len()));
                let er = Rect::from_min_size(client.min + Vec2::new(12.0, 34.0), Vec2::new(client.width() - 24.0, 21.0));
                edit(ui, er, archive, true, "pack", None);
                label(ui, client.min + Vec2::new(12.0, 60.0), "Packer: ZIP (deflate)");
                let (ok, cancel) = two_buttons(ui, client, "OK", "Cancel");
                if ok || enter {
                    let t = archive.trim().trim_start_matches("zip:").to_string();
                    let mut t = PathBuf::from(t);
                    if t.extension().is_none() {
                        t.set_extension("zip");
                    }
                    return Some(Action::StartOp(OpRequest { kind: OpKind::Pack, sources: sources.clone(), target: t, rename_to: None }));
                }
                if cancel || esc {
                    return Some(Action::Close);
                }
            }
            Dialog::Unpack { target, archives } => {
                let client = window(ui, screen, "Unpack files", Vec2::new(520.0, 150.0));
                label(ui, client.min + Vec2::new(12.0, 14.0), "Unpack specific files from archive to:");
                let er = Rect::from_min_size(client.min + Vec2::new(12.0, 34.0), Vec2::new(client.width() - 24.0, 21.0));
                let sel = all(target);
                edit(ui, er, target, true, "copy", sel);
                let (ok, cancel) = two_buttons(ui, client, "OK", "Cancel");
                if ok || enter {
                    let t = PathBuf::from(target.trim().trim_end_matches("*.*").trim_end_matches('*'));
                    return Some(Action::StartOp(OpRequest { kind: OpKind::Unpack, sources: archives.clone(), target: t, rename_to: None }));
                }
                if cancel || esc {
                    return Some(Action::Close);
                }
            }
            Dialog::Find(f) => {
                let client = window(ui, screen, "Find Files", Vec2::new(600.0, 500.0));
                let p = ui.painter().clone();
                xp::group_box(&p, Rect::from_min_size(client.min + Vec2::new(10.0, 8.0), Vec2::new(client.width() - 20.0, 112.0)), "General");
                label(ui, client.min + Vec2::new(22.0, 30.0), "Search for:");
                let e1 = Rect::from_min_size(client.min + Vec2::new(100.0, 27.0), Vec2::new(client.width() - 130.0, 21.0));
                let sel = all(&f.mask);
                edit(ui, e1, &mut f.mask, true, "find-mask", sel);
                label(ui, client.min + Vec2::new(22.0, 56.0), "Search in:");
                let e2 = Rect::from_min_size(client.min + Vec2::new(100.0, 53.0), Vec2::new(client.width() - 130.0, 21.0));
                edit(ui, e2, &mut f.dir, false, "find-dir", None);
                label(ui, client.min + Vec2::new(22.0, 82.0), "Find text:");
                let e3 = Rect::from_min_size(client.min + Vec2::new(100.0, 79.0), Vec2::new(client.width() - 130.0, 21.0));
                edit(ui, e3, &mut f.text, false, "find-text", None);
                // results list
                let lr = Rect::from_min_max(client.min + Vec2::new(10.0, 130.0), Pos2::new(client.right() - 10.0, client.bottom() - 70.0));
                xp::sunken(ui.painter(), lr);
                let inner = lr.shrink(1.0);
                let rows = (inner.height() / 16.0).floor().max(1.0) as usize;
                let lp = ui.painter_at(inner);
                for row in 0..rows {
                    let i = f.scroll + row;
                    let Some(path) = f.results.get(i) else { break };
                    let y = inner.top() + row as f32 * 16.0;
                    let rr = Rect::from_min_size(Pos2::new(inner.left(), y), Vec2::new(inner.width() - xp::SB_W, 16.0));
                    if i == f.cursor {
                        lp.rect_filled(rr, 0.0, xp::HIGHLIGHT);
                    }
                    let col = if i == f.cursor { xp::HIGHLIGHT_TEXT } else { xp::TEXT };
                    lp.text(Pos2::new(rr.left() + 4.0, y + 1.0), Align2::LEFT_TOP, &xp::ellipsize_path(&lp, &path.to_string_lossy(), rr.width() - 8.0, xp::font()), xp::font(), col);
                    let resp = ui.interact(rr, ui.id().with(("fres", i)), Sense::CLICK);
                    if resp.clicked() {
                        f.cursor = i;
                    }
                    if resp.double_clicked() {
                        return Some(Action::GotoFile(path.clone()));
                    }
                }
                let sb = Rect::from_min_max(Pos2::new(inner.right() - xp::SB_W, inner.top()), inner.right_bottom());
                if let Some(s) = crate::panel::scrollbar(ui, &ui.painter().clone(), sb, ui.id().with("fsb"), f.scroll, rows, f.results.len(), true) {
                    f.scroll = s;
                }
                if ui.rect_contains_pointer(inner) {
                    let dy = crate::panel::wheel_dy(ui);
                    if dy > 0.0 {
                        f.scroll = f.scroll.saturating_sub(3);
                    } else if dy < 0.0 {
                        f.scroll = (f.scroll + 3).min(f.results.len().saturating_sub(rows));
                    }
                }
                // poll the worker
                if let Some(rx) = &f.rx {
                    let mut n = 0;
                    while let Ok(m) = rx.try_recv() {
                        match m {
                            Some(p) => f.results.push(p),
                            None => {
                                f.running = false;
                            }
                        }
                        n += 1;
                        if n > 500 {
                            break;
                        }
                    }
                    if f.running {
                        ui.ctx().request_repaint();
                    } else {
                        f.rx = None;
                    }
                }
                let status = if f.running { format!("Searching... {} found", f.results.len()) } else if f.results.is_empty() && f.searched > 0 { "No files found".to_string() } else { format!("{} file(s) found", f.results.len()) };
                label(ui, Pos2::new(client.left() + 12.0, client.bottom() - 60.0), &status);
                // keys in the list
                let (dn, up) = ui.input(|i| (i.key_pressed(egui::Key::ArrowDown), i.key_pressed(egui::Key::ArrowUp)));
                if dn && f.cursor + 1 < f.results.len() {
                    f.cursor += 1;
                }
                if up && f.cursor > 0 {
                    f.cursor -= 1;
                }
                if f.cursor < f.scroll {
                    f.scroll = f.cursor;
                } else if f.cursor >= f.scroll + rows {
                    f.scroll = f.cursor + 1 - rows;
                }
                let y = client.bottom() - 32.0;
                let bs = Rect::from_min_size(Pos2::new(client.left() + 12.0, y), Vec2::new(90.0, 23.0));
                let bf = Rect::from_min_size(Pos2::new(client.left() + 112.0, y), Vec2::new(110.0, 23.0));
                let bg = Rect::from_min_size(Pos2::new(client.left() + 232.0, y), Vec2::new(90.0, 23.0));
                let bc = Rect::from_min_size(Pos2::new(client.right() - 87.0, y), Vec2::new(75.0, 23.0));
                let start = button(ui, bs, if f.running { "&Stop" } else { "&Start search" }, !f.running, true);
                let feed = button(ui, bf, "F&eed to listbox", false, !f.results.is_empty() && !f.running);
                let go = button(ui, bg, "&Go to file", false, !f.results.is_empty() && !f.running);
                let cancel = button(ui, bc, "Cancel", false, true);
                if start || (enter && !f.running && f.results.is_empty()) {
                    if f.running {
                        f.cancel.store(true, Ordering::Relaxed);
                        f.running = false;
                    } else {
                        f.start();
                    }
                } else if enter && !f.results.is_empty() && !f.running {
                    if let Some(p) = f.results.get(f.cursor) {
                        return Some(Action::GotoFile(p.clone()));
                    }
                }
                if go {
                    if let Some(p) = f.results.get(f.cursor) {
                        return Some(Action::GotoFile(p.clone()));
                    }
                }
                if feed {
                    return Some(Action::FeedToListbox(f.results.clone(), format!("Search results: {} in {}", f.mask, f.dir)));
                }
                if cancel || esc {
                    f.cancel.store(true, Ordering::Relaxed);
                    return Some(Action::Close);
                }
            }
            Dialog::About => {
                let client = window(ui, screen, "About total-commacer", Vec2::new(420.0, 300.0));
                let p = ui.painter().clone();
                let c = client.min + Vec2::new(20.0, 20.0);
                xp::text_bold(&p, c, "total-commacer", xp::TEXT);
                xp::text(&p, c + Vec2::new(0.0, 16.0), &format!("Version {} ({})", env!("CARGO_PKG_VERSION"), std::env::consts::OS), xp::TEXT);
                let lines = [
                    "A two-panel file manager in the spirit of the Windows XP era,",
                    "written from scratch in Rust for macOS, Linux and Windows.",
                    "",
                    "Not affiliated with Total Commander or C. Ghisler & Co.",
                    "Tahoma-compatible font: Wine project (LGPL).",
                    "",
                    "This program is free software (MIT license).",
                    "No registration necessary. Really.",
                ];
                for (i, l) in lines.iter().enumerate() {
                    xp::text(&p, c + Vec2::new(0.0, 46.0 + i as f32 * 16.0), l, xp::TEXT);
                }
                let y = client.bottom() - 32.0;
                let ok = Rect::from_min_size(Pos2::new(client.center().x - 37.0, y), Vec2::new(75.0, 23.0));
                let home = Rect::from_min_size(Pos2::new(client.left() + 14.0, y), Vec2::new(110.0, 23.0));
                if button(ui, home, "&Homepage", false, true) {
                    return Some(Action::OpenUrl("https://github.com/axelsharper/total-commacer".into()));
                }
                if button(ui, ok, "OK", true, true) || enter || esc {
                    return Some(Action::Close);
                }
            }
            Dialog::Nag { number, tries } => {
                let client = window(ui, screen, "total-commacer", Vec2::new(400.0, 190.0));
                let p = ui.painter().clone();
                xp::text_bold(&p, client.min + Vec2::new(20.0, 18.0), "This program is free software.", xp::TEXT);
                xp::text(&p, client.min + Vec2::new(20.0, 40.0), "You don't have to register. This dialog is here because", xp::TEXT);
                xp::text(&p, client.min + Vec2::new(20.0, 56.0), "it always was. (Configuration > Options turns it off.)", xp::TEXT);
                xp::text_bold(&p, client.min + Vec2::new(20.0, 84.0), &format!("Please press button number {}", number), xp::TEXT);
                let y = client.bottom() - 42.0;
                let mut pressed = None;
                for i in 0..3u8 {
                    let r = Rect::from_min_size(Pos2::new(client.center().x - 125.0 + i as f32 * 85.0, y), Vec2::new(75.0, 23.0));
                    if button(ui, r, &format!("{}", i + 1), false, true) {
                        pressed = Some(i + 1);
                    }
                }
                let typed = ui.input(|i| i.events.iter().find_map(|e| if let egui::Event::Text(t) = e { t.chars().next().and_then(|c| c.to_digit(10)).map(|d| d as u8) } else { None }));
                if let Some(n) = pressed.or(typed) {
                    if n == *number {
                        return Some(Action::Close);
                    }
                    *tries += 1;
                    *number = (*number % 3) + 1;
                }
                if esc && *tries > 2 {
                    return Some(Action::Close);
                }
            }
            Dialog::Options(o) => {
                let client = window(ui, screen, "Configuration", Vec2::new(460.0, 330.0));
                let p = ui.painter().clone();
                xp::group_box(&p, Rect::from_min_size(client.min + Vec2::new(10.0, 8.0), Vec2::new(client.width() - 20.0, 120.0)), "Operation");
                let x = client.left() + 24.0;
                check(ui, Pos2::new(x, client.top() + 30.0), "Show &hidden/system files (Ctrl+H)", &mut o.show_hidden);
                check(ui, Pos2::new(x, client.top() + 50.0), "&Lynx-like motion (Left = parent, Right = enter)", &mut o.lynx);
                check(ui, Pos2::new(x, client.top() + 70.0), "&Right mouse button selects (Norton style)", &mut o.right_click_selects);
                check(ui, Pos2::new(x, client.top() + 90.0), "Show the &nag screen at startup", &mut o.nag);
                xp::group_box(&p, Rect::from_min_size(client.min + Vec2::new(10.0, 136.0), Vec2::new(client.width() - 20.0, 86.0)), "Quick search");
                if radio(ui, Pos2::new(x, client.top() + 158.0), "&Ctrl+Alt+Letters", o.quick_search == QuickSearch::CtrlAlt) {
                    o.quick_search = QuickSearch::CtrlAlt;
                }
                if radio(ui, Pos2::new(x, client.top() + 178.0), "&Alt+Letters", o.quick_search == QuickSearch::Alt) {
                    o.quick_search = QuickSearch::Alt;
                }
                if radio(ui, Pos2::new(x, client.top() + 198.0), "Letters &only (command line disabled)", o.quick_search == QuickSearch::Letters) {
                    o.quick_search = QuickSearch::Letters;
                }
                label(ui, Pos2::new(client.left() + 12.0, client.top() + 236.0), "Editor for F4 (empty = system default):");
                let er = Rect::from_min_size(Pos2::new(client.left() + 12.0, client.top() + 254.0), Vec2::new(client.width() - 24.0, 21.0));
                edit(ui, er, &mut o.editor, false, "editor", None);
                let (ok, cancel) = two_buttons(ui, client, "OK", "Cancel");
                if ok || enter {
                    return Some(Action::ApplyOptions(o.clone()));
                }
                if cancel || esc {
                    return Some(Action::Close);
                }
            }
        }
        None
    }
}

impl FindState {
    pub fn new(dir: &Path) -> Self {
        Self {
            mask: "*.*".into(),
            dir: fs::display(dir),
            text: String::new(),
            results: Vec::new(),
            cursor: 0,
            scroll: 0,
            rx: None,
            cancel: Arc::new(AtomicBool::new(false)),
            running: false,
            searched: 0,
        }
    }

    pub fn start(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.cancel = Arc::new(AtomicBool::new(false));
        self.results.clear();
        self.cursor = 0;
        self.scroll = 0;
        self.searched += 1;
        self.running = true;
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let mask = if self.mask.trim().is_empty() { "*.*".to_string() } else { self.mask.clone() };
        let dir = PathBuf::from(self.dir.trim());
        let text = self.text.clone();
        let cancel = self.cancel.clone();
        std::thread::spawn(move || {
            let needle = text.to_lowercase();
            for e in walkdir::WalkDir::new(&dir).into_iter().flatten() {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let name = e.file_name().to_string_lossy();
                if !fs::matches_mask(&name, &mask) {
                    continue;
                }
                if !needle.is_empty() {
                    if !e.file_type().is_file() {
                        continue;
                    }
                    let ok = std::fs::read(e.path()).map(|d| String::from_utf8_lossy(&d).to_lowercase().contains(&needle)).unwrap_or(false);
                    if !ok {
                        continue;
                    }
                }
                if tx.send(Some(e.path().to_path_buf())).is_err() {
                    break;
                }
            }
            let _ = tx.send(None);
        });
    }
}

/// TC target semantics: "dir\*.*" / "dir\" / "dir" = into dir; "dir\newname" for one file = rename.
pub fn resolve_target(s: &str, cwd: &Path, n_sources: usize) -> (PathBuf, Option<String>) {
    let mut t = s.trim().to_string();
    for suf in ["*.*", "*"] {
        if t.ends_with(suf) {
            t.truncate(t.len() - suf.len());
        }
    }
    let ends_sep = t.ends_with('/') || t.ends_with('\\');
    let t = t.trim_end_matches(['/', '\\']).to_string();
    let path = if t.is_empty() {
        cwd.to_path_buf()
    } else {
        let p = PathBuf::from(&t);
        if p.is_absolute() {
            p
        } else {
            cwd.join(p)
        }
    };
    if ends_sep || path.is_dir() || n_sources != 1 {
        (path, None)
    } else {
        let name = path.file_name().map(|s| s.to_string_lossy().to_string());
        let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or(cwd.to_path_buf());
        (dir, name)
    }
}
