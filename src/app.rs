//! The main window: frame, menu bar, toolbar, drive bars, two panels, command line, function keys.

use std::path::{Path, PathBuf};
use std::process::Command;

use egui::{Align2, Color32, Context, Key, Pos2, Rect, Sense, Ui, Vec2, ViewportCommand};

use crate::commands::{self, Cmd, Mods, MENUS};
use crate::config::{Config, QuickSearch};
use crate::dialogs::{self, Action, Dialog, FindState};
use crate::fs::{self, Drive, SortBy};
use crate::icons::{self, Icons};
use crate::lister::Lister;
use crate::ops::{self, Msg, OpHandle, OpKind};
use crate::panel::{self, Panel, PanelEvent};
use crate::xp;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Left,
    Right,
}

#[derive(Clone, Debug)]
pub enum PopupAction {
    Cmd(Cmd),
    Goto(Side, PathBuf),
    SetCmdLine(String),
    HotAdd,
    HotRemove,
    None,
}

#[derive(Clone, Debug)]
pub enum PopupItem {
    Item { label: String, shortcut: String, enabled: bool, action: PopupAction },
    Sep,
}

pub struct Popup {
    pub items: Vec<PopupItem>,
    pub pos: Pos2,
    pub width: f32,
    pub menu_index: Option<usize>,
    pub hover: Option<usize>,
    pub opened_at: f64,
}

const MENU_H: f32 = 20.0;
const TOOLBAR_H: f32 = 28.0;
const DRIVEBAR_H: f32 = 26.0;
const CMD_H: f32 = 24.0;
const FKEY_H: f32 = 25.0;

pub struct App {
    cfg: Config,
    icons: Icons,
    drives: Vec<Drive>,
    left: Panel,
    right: Panel,
    active: Side,
    cmd: String,
    cmd_caret: usize,
    cmd_hist_pos: Option<usize>,
    dialog: Option<Dialog>,
    op: Option<OpHandle>,
    popup: Option<Popup>,
    lister: Option<Lister>,
    quick_view: bool,
    quick_lister: Option<Lister>,
    quick_path: Option<PathBuf>,
    swallow_text: bool,
    exit_requested: bool,
    frame_no: u64,
    menu_rects: Vec<Rect>,
    dialog_seen_at: Option<f64>,
    now: f64,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let cfg = Config::load();
        let icons = Icons::load(&cc.egui_ctx);
        let left = Panel::new(cfg.left.path.clone(), cfg.left.sort, cfg.left.reverse, cfg.left.brief, cfg.show_hidden);
        let right = Panel::new(cfg.right.path.clone(), cfg.right.sort, cfg.right.reverse, cfg.right.brief, cfg.show_hidden);
        let nag = cfg.nag;
        xp::set_theme(cfg.theme);
        xp::apply_egui_visuals(&cc.egui_ctx);
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
        let mut app = Self {
            cfg,
            icons,
            drives: fs::drives(),
            left,
            right,
            active: Side::Left,
            cmd: String::new(),
            cmd_caret: 0,
            cmd_hist_pos: None,
            dialog: None,
            op: None,
            popup: None,
            lister: None,
            quick_view: false,
            quick_lister: None,
            quick_path: None,
            swallow_text: false,
            exit_requested: false,
            frame_no: 0,
            menu_rects: Vec::new(),
            dialog_seen_at: None,
            now: 0.0,
        };
        if nag {
            app.dialog = Some(Dialog::Nag { number: (seed % 3) as u8 + 1, tries: 0 });
        }
        app
    }

    fn active(&mut self) -> &mut Panel {
        match self.active {
            Side::Left => &mut self.left,
            Side::Right => &mut self.right,
        }
    }
    fn other(&mut self) -> &mut Panel {
        match self.active {
            Side::Left => &mut self.right,
            Side::Right => &mut self.left,
        }
    }
    fn panel(&mut self, s: Side) -> &mut Panel {
        match s {
            Side::Left => &mut self.left,
            Side::Right => &mut self.right,
        }
    }

    fn reload_both(&mut self) {
        self.left.reload();
        self.right.reload();
    }

    fn message(&mut self, title: &str, text: &str, error: bool) {
        self.dialog = Some(Dialog::Message { title: title.into(), text: text.into(), error });
    }

    fn not_implemented(&mut self, what: &str) {
        self.message("total-commacer", &format!("{what} is not implemented yet."), false);
    }

    // ---------------------------------------------------------------- commands
    fn dispatch(&mut self, ctx: &Context, cmd: Cmd) {
        use Cmd::*;
        match cmd {
            Help | Keyboard => self.dialog = Some(Dialog::Text { title: "Keyboard layout".into(), text: commands::KEYBOARD_HELP.into() }),
            Reread => {
                self.active().reload();
                let a = self.active().path.clone();
                if self.other().path == a {
                    self.other().reload();
                }
                self.drives = fs::drives();
            }
            View | ViewCursor => {
                if let Some(e) = self.active().cursor_entry().cloned() {
                    if !e.is_dir {
                        self.open_lister(&e.path);
                    }
                }
            }
            Edit => {
                if let Some(e) = self.active().cursor_entry().cloned() {
                    if !e.is_dir {
                        self.edit_file(&e.path);
                    }
                }
            }
            EditNew => self.dialog = Some(Dialog::EditNew { name: "new.txt".into() }),
            Copy | Move => self.copy_dialog(cmd == Move, false),
            CopySameDir => self.copy_dialog(false, true),
            Rename => {
                if let Some(e) = self.active().cursor_entry().cloned() {
                    if !e.is_up {
                        self.dialog = Some(Dialog::Rename { old: e.path.clone(), name: e.name.clone() });
                    }
                }
            }
            MkDir => self.dialog = Some(Dialog::MkDir { name: String::new() }),
            Delete => {
                let t = self.active().targets();
                if t.is_empty() {
                    return;
                }
                let prompt = if t.len() == 1 { format!("Do you really want to delete\n{}?", fs::display(&t[0].path)) } else { format!("Do you really want to delete {} file(s)/directorie(s)?", t.len()) };
                self.dialog = Some(Dialog::Delete { prompt, sources: t.iter().map(|e| e.path.clone()).collect() });
            }
            Exit => {
                debug("exit: Cmd::Exit");
                self.exit_requested = true;
            }
            Pack => {
                let t = self.active().targets();
                if t.is_empty() {
                    return;
                }
                let name = if t.len() == 1 { t[0].stem().to_string() } else { self.active().path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or("archive".into()) };
                let dir = self.other().path.clone();
                let d = fs::display(&dir);
                let sep = if d.ends_with(fs::sep()) { String::new() } else { fs::sep().to_string() };
                let archive = format!("zip:{d}{sep}{name}.zip");
                self.dialog = Some(Dialog::Pack { archive, sources: t.iter().map(|e| e.path.clone()).collect() });
            }
            Unpack => {
                let t: Vec<PathBuf> = self.active().targets().iter().filter(|e| !e.is_dir && e.ext_lower() == "zip").map(|e| e.path.clone()).collect();
                if t.is_empty() {
                    self.message("Unpack", "Please put the cursor on a ZIP archive.", false);
                    return;
                }
                let dir = self.other().path.clone();
                self.dialog = Some(Dialog::Unpack { target: self.path_with_star(&dir), archives: t });
            }
            Find => self.dialog = Some(Dialog::Find(FindState::new(&self.active().path.clone()))),
            Properties => self.properties(),
            CalcSpace => self.calc_space(),
            CalcAllDirs => self.active().count_all_dirs(),
            Attributes => self.not_implemented("Change attributes"),
            CompareContent => self.not_implemented("Compare by content"),
            CompareDirs => self.compare_dirs(),
            MultiRename => self.not_implemented("The multi-rename tool"),
            Open => {
                if !self.cmd.is_empty() {
                    self.run_cmdline();
                } else {
                    let mut ev = Vec::new();
                    self.active().quick = None;
                    self.active().open_cursor(&mut ev);
                    self.handle_panel_events(ctx, self.active, ev);
                }
            }
            OpenDir => {
                let mut ev = Vec::new();
                if self.active().cursor_entry().map(|e| e.is_dir).unwrap_or(false) {
                    self.active().open_cursor(&mut ev);
                }
            }
            Parent => {
                if !self.cmd.is_empty() {
                    self.cmd_backspace();
                } else {
                    self.active().parent();
                }
            }
            Root => self.active().root(),
            GoBack => self.active().go_back(),
            GoForward => self.active().go_forward(),
            DirHistory => self.dir_history_popup(),
            OpenInOther => {
                let (p, dir) = match self.active().cursor_entry().cloned() {
                    Some(e) if e.is_dir && !e.is_up => (e.path, true),
                    _ => (self.active().path.clone(), false),
                };
                let _ = dir;
                self.other().goto(&p, None);
            }
            DriveLeft => self.drive_popup(Side::Left),
            DriveRight => self.drive_popup(Side::Right),
            Tree | TreeView => self.not_implemented("The directory tree"),
            Hotlist => self.hotlist_popup(),
            NewTab => self.active().new_tab(),
            CloseTab => self.active().close_tab(),
            NextTab => self.active().switch_tab(1),
            PrevTab => self.active().switch_tab(-1),
            SwitchPanel => {
                self.active().quick = None;
                self.active = if self.active == Side::Left { Side::Right } else { Side::Left };
            }
            SwapPanels => {
                std::mem::swap(&mut self.left, &mut self.right);
            }
            TargetEqSource => {
                let p = self.active().path.clone();
                self.other().goto(&p, None);
            }
            CursorUp => {
                if self.active().quick.is_some() {
                    self.quick_prev();
                } else {
                    self.active().move_cursor(-1, false)
                }
            }
            CursorDown => {
                if self.active().quick.is_some() {
                    self.active().quick_next();
                } else {
                    self.active().move_cursor(1, false)
                }
            }
            PageUp => {
                let n = self.active().page_rows();
                self.active().move_cursor(-n, false)
            }
            PageDown => {
                let n = self.active().page_rows();
                self.active().move_cursor(n, false)
            }
            Home => {
                if !self.cmd.is_empty() {
                    self.cmd_caret = 0;
                } else {
                    self.active().home(false)
                }
            }
            End => {
                if !self.cmd.is_empty() {
                    self.cmd_caret = self.cmd.chars().count();
                } else {
                    self.active().end(false)
                }
            }
            SelUp => self.active().move_cursor(-1, true),
            SelDown => self.active().move_cursor(1, true),
            SelPageUp => {
                let n = self.active().page_rows();
                self.active().move_cursor(-n, true)
            }
            SelPageDown => {
                let n = self.active().page_rows();
                self.active().move_cursor(n, true)
            }
            SelHome => self.active().home(true),
            SelEnd => self.active().end(true),
            Left | Right => {
                let right = cmd == Right;
                if !self.cmd.is_empty() {
                    let n = self.cmd.chars().count();
                    self.cmd_caret = if right { (self.cmd_caret + 1).min(n) } else { self.cmd_caret.saturating_sub(1) };
                } else if self.active().view == panel::View::Brief {
                    let rows = self.active().rows_visible as i64;
                    self.active().move_cursor(if right { rows } else { -rows }, false);
                } else if self.cfg.lynx {
                    if right {
                        self.dispatch(ctx, OpenDir);
                    } else {
                        self.active().parent();
                    }
                }
            }
            Escape => {
                self.cmd.clear();
                self.cmd_caret = 0;
                self.cmd_hist_pos = None;
                self.active().quick = None;
            }
            ToggleSelDown => {
                let i = self.active().cursor;
                self.active().toggle(i);
                self.active().move_cursor(1, false);
            }
            ToggleSelCount => {
                let i = self.active().cursor;
                self.active().count_dir_under_cursor();
                self.active().toggle(i);
            }
            SelectAll => self.active().select_mask("*.*", true, true),
            UnselectAll => self.active().selected.clear(),
            SelectGroup => self.dialog = Some(Dialog::SelectGroup { mask: "*.*".into(), on: true }),
            UnselectGroup => self.dialog = Some(Dialog::SelectGroup { mask: "*.*".into(), on: false }),
            InvertSelection => self.active().invert(),
            RestoreSelection => self.active().restore_selection(),
            SelectSameExt => self.active().select_same_ext(true),
            UnselectSameExt => self.active().select_same_ext(false),
            CopyNamesToClip => {
                let s = self.active().targets().iter().map(|e| e.name.clone()).collect::<Vec<_>>().join("\n");
                ctx.copy_text(s);
            }
            CopyFullNamesToClip => {
                let s = self.active().targets().iter().map(|e| fs::display(&e.path)).collect::<Vec<_>>().join("\n");
                ctx.copy_text(s);
            }
            Brief => {
                self.active().view = panel::View::Brief;
                self.active().ensure_visible();
            }
            Full => {
                self.active().view = panel::View::Full;
                self.active().ensure_visible();
            }
            QuickView => {
                self.quick_view = !self.quick_view;
                self.quick_lister = None;
                self.quick_path = None;
            }
            SortName => self.sort(SortBy::Name),
            SortExt => self.sort(SortBy::Ext),
            SortDate => self.sort(SortBy::Date),
            SortSize => self.sort(SortBy::Size),
            SortUnsorted => self.sort(SortBy::Unsorted),
            ReverseSort => {
                let p = self.active();
                p.reverse = !p.reverse;
                p.reload();
            }
            AllFiles => {
                self.active().filter = "*.*".into();
                self.active().reload();
            }
            ProgramsOnly => {
                self.active().filter = "*.exe;*.com;*.bat;*.cmd;*.app;*.sh;*.appimage;*.run".into();
                self.active().reload();
            }
            CustomFilter => self.dialog = Some(Dialog::CustomFilter { mask: self.active().filter.clone() }),
            BranchView => {
                let p = self.active();
                p.branch = !p.branch;
                p.cursor = 0;
                p.reload();
            }
            ToggleHidden => {
                self.cfg.show_hidden = !self.cfg.show_hidden;
                self.left.show_hidden = self.cfg.show_hidden;
                self.right.show_hidden = self.cfg.show_hidden;
                self.reload_both();
            }
            CopyNameToCmd => {
                if let Some(e) = self.active().cursor_entry().cloned() {
                    self.cmd_append(&quote(&e.name));
                }
            }
            CopyPathNameToCmd => {
                if let Some(e) = self.active().cursor_entry().cloned() {
                    self.cmd_append(&quote(&fs::display(&e.path)));
                }
            }
            CopyPathToCmd => {
                let p = fs::display(&self.active().path);
                self.cmd_append(&quote(&p));
            }
            CmdHistory => self.cmd_history_popup(),
            CmdPrev => self.cmd_history_step(1),
            CmdNext => self.cmd_history_step(-1),
            OpenTerminal => {
                let dir = self.active().path.clone();
                if let Err(e) = open_terminal(&dir) {
                    self.message("Error", &e, true);
                }
            }
            MenuBar => self.open_menu(0),
            Options => {
                self.dialog = Some(Dialog::Options(dialogs::Options {
                    nag: self.cfg.nag,
                    lynx: self.cfg.lynx,
                    right_click_selects: self.cfg.right_click_selects,
                    show_hidden: self.cfg.show_hidden,
                    quick_search: self.cfg.quick_search,
                    editor: self.cfg.editor.clone(),
                    theme: self.cfg.theme,
                }))
            }
            About => self.dialog = Some(Dialog::About),
            Homepage => {
                let _ = open::that_detached("https://github.com/axelsharper/total-commacer");
            }
            SavePosition => {
                self.store_config(ctx);
                self.cfg.save();
            }
            Nothing => {}
        }
    }

    fn sort(&mut self, by: SortBy) {
        let p = self.active();
        if p.sort == by {
            p.reverse = !p.reverse;
        } else {
            p.sort = by;
            p.reverse = false;
        }
        p.reload();
    }

    fn path_with_star(&self, dir: &Path) -> String {
        let d = fs::display(dir);
        if d.ends_with(fs::sep()) {
            format!("{d}*.*")
        } else {
            format!("{d}{}*.*", fs::sep())
        }
    }

    fn copy_dialog(&mut self, mv: bool, same_dir: bool) {
        let t = self.active().targets();
        if t.is_empty() {
            return;
        }
        let verb = if mv { "Move/Rename" } else { "Copy" };
        let prompt = if t.len() == 1 { format!("{verb} '{}' to:", t[0].name) } else { format!("{verb} {} file(s)/directorie(s) to:", t.len()) };
        let cwd = self.active().path.clone();
        let target = if same_dir {
            if t.len() == 1 {
                t[0].name.clone()
            } else {
                self.path_with_star(&cwd)
            }
        } else {
            let o = self.other().path.clone();
            self.path_with_star(&o)
        };
        self.dialog = Some(Dialog::Copy { prompt, target, mv, sources: t.iter().map(|e| e.path.clone()).collect(), cwd });
    }

    fn properties(&mut self) {
        let Some(e) = self.active().cursor_entry().cloned() else { return };
        if e.is_up {
            return;
        }
        let mut lines = vec![
            ("Name:".to_string(), e.name.clone()),
            ("Type:".to_string(), if e.is_dir { "File Folder".into() } else if e.ext().is_empty() { "File".into() } else { format!("{} File", e.ext().to_uppercase()) }),
            ("Location:".to_string(), fs::display(&self.active().path)),
        ];
        if e.is_dir {
            let s = fs::dir_size(&e.path);
            lines.push(("Size:".into(), format!("{} bytes ({})", fs::fmt_num(s), fs::fmt_k(s))));
        } else {
            lines.push(("Size:".into(), format!("{} bytes ({})", fs::fmt_num(e.size), fs::fmt_k(e.size))));
        }
        lines.push(("Modified:".into(), fs::fmt_date(&e.modified)));
        lines.push(("Attributes:".into(), fs::attr_string(&e)));
        if e.symlink {
            if let Ok(t) = std::fs::read_link(&e.path) {
                lines.push(("Link to:".into(), t.to_string_lossy().to_string()));
            }
        }
        self.dialog = Some(Dialog::Properties { name: e.name.clone(), lines });
    }

    fn calc_space(&mut self) {
        let t = self.active().targets();
        let mut bytes = 0u64;
        let mut files = 0u64;
        let mut dirs = 0u64;
        for e in &t {
            if e.is_dir {
                dirs += 1;
                for w in walkdir::WalkDir::new(&e.path).min_depth(1).into_iter().flatten() {
                    if w.file_type().is_dir() {
                        dirs += 1;
                    } else {
                        files += 1;
                        bytes += w.metadata().map(|m| m.len()).unwrap_or(0);
                    }
                }
            } else {
                files += 1;
                bytes += e.size;
            }
        }
        self.dialog = Some(Dialog::CalcSpace {
            lines: vec![
                ("Selected:".into(), format!("{} file(s), {} dir(s)", files, dirs)),
                ("Total size:".into(), format!("{} bytes", fs::fmt_num(bytes))),
                ("".into(), fs::fmt_k(bytes)),
                ("".into(), format!("{:.2} MB", bytes as f64 / 1048576.0)),
            ],
        });
    }

    /// Shift+F2: mark files that exist only on one side or are newer than the other side's copy.
    fn compare_dirs(&mut self) {
        let mark = |a: &mut Panel, b: &Panel| {
            let names: std::collections::HashMap<&str, &fs::Entry> = b.entries.iter().filter(|e| !e.is_dir).map(|e| (e.name.as_str(), e)).collect();
            a.selected.clear();
            for i in 0..a.entries.len() {
                let e = &a.entries[i];
                if e.is_dir {
                    continue;
                }
                let on = match names.get(e.name.as_str()) {
                    None => true,
                    Some(o) => e.modified > o.modified,
                };
                if on {
                    a.set_selected(i, true);
                }
            }
        };
        mark(&mut self.left, &self.right);
        mark(&mut self.right, &self.left);
    }

    fn open_lister(&mut self, path: &Path) {
        match Lister::open(path) {
            Ok(l) => self.lister = Some(l),
            Err(e) => self.message("Lister", &format!("{}\n{}", fs::display(path), fs::err_text(&e)), true),
        }
    }

    fn edit_file(&mut self, path: &Path) {
        let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let r = if !self.cfg.editor.trim().is_empty() {
            let mut parts = self.cfg.editor.split_whitespace();
            let exe = parts.next().unwrap_or("");
            Command::new(exe).args(parts).arg(path).current_dir(&dir).spawn().map(|_| ()).map_err(|e| e.to_string())
        } else if cfg!(windows) {
            Command::new("notepad.exe").arg(path).spawn().map(|_| ()).map_err(|e| e.to_string())
        } else if cfg!(target_os = "macos") {
            Command::new("open").arg("-t").arg(path).spawn().map(|_| ()).map_err(|e| e.to_string())
        } else {
            // no editor configured: try the usual desktop text editors, then $EDITOR in a
            // terminal, then whatever the desktop associates with the file
            let mut r = Err(String::new());
            for ed in ["gnome-text-editor", "gedit", "kate", "mousepad", "xed", "pluma", "leafpad", "geany"] {
                if let Ok(_) = Command::new(ed).arg(path).current_dir(&dir).spawn() {
                    r = Ok(());
                    break;
                }
            }
            if r.is_err() {
                if let Ok(ed) = std::env::var("EDITOR") {
                    if !ed.trim().is_empty() {
                        r = run_in_terminal(&dir, &format!("{ed} '{}'", path.to_string_lossy().replace('\'', "'\\''")));
                    }
                }
            }
            if r.is_err() {
                r = open::that_detached(path).map_err(|e| e.to_string());
            }
            r
        };
        if let Err(e) = r {
            self.message("Error", &format!("Cannot start the editor:\n{e}"), true);
        }
    }

    fn open_file(&mut self, path: &Path) {
        if let Err(e) = open::that_detached(path) {
            self.message("Error", &format!("Cannot open\n{}\n{}", fs::display(path), e), true);
        }
    }

    // ---------------------------------------------------------------- command line
    fn cmd_append(&mut self, s: &str) {
        if !self.cmd.is_empty() && !self.cmd.ends_with(' ') {
            self.cmd.push(' ');
        }
        self.cmd.push_str(s);
        self.cmd.push(' ');
        self.cmd_caret = self.cmd.chars().count();
    }

    fn cmd_insert(&mut self, s: &str) {
        let idx = byte_index(&self.cmd, self.cmd_caret);
        self.cmd.insert_str(idx, s);
        self.cmd_caret += s.chars().count();
    }

    fn cmd_backspace(&mut self) {
        if self.cmd_caret > 0 {
            let idx = byte_index(&self.cmd, self.cmd_caret - 1);
            self.cmd.remove(idx);
            self.cmd_caret -= 1;
        }
    }

    fn cmd_delete(&mut self) {
        if self.cmd_caret < self.cmd.chars().count() {
            let idx = byte_index(&self.cmd, self.cmd_caret);
            self.cmd.remove(idx);
        }
    }

    fn cmd_history_step(&mut self, dir: i64) {
        if self.cfg.cmd_history.is_empty() {
            return;
        }
        let n = self.cfg.cmd_history.len() as i64;
        let pos = match self.cmd_hist_pos {
            None => {
                if dir > 0 {
                    0
                } else {
                    return;
                }
            }
            Some(p) => (p as i64 + dir).clamp(-1, n - 1),
        };
        if pos < 0 {
            self.cmd_hist_pos = None;
            self.cmd.clear();
            self.cmd_caret = 0;
        } else {
            self.cmd_hist_pos = Some(pos as usize);
            self.cmd = self.cfg.cmd_history[pos as usize].clone();
            self.cmd_caret = self.cmd.chars().count();
        }
    }

    fn run_cmdline(&mut self) {
        let line = self.cmd.trim().to_string();
        self.cmd.clear();
        self.cmd_caret = 0;
        self.cmd_hist_pos = None;
        if line.is_empty() {
            return;
        }
        self.cfg.cmd_history.retain(|h| h != &line);
        self.cfg.cmd_history.insert(0, line.clone());
        self.cfg.cmd_history.truncate(50);
        let lower = line.to_lowercase();
        if lower == "cd" || lower.starts_with("cd ") || lower.starts_with("cd\\") || lower.starts_with("cd/") {
            let arg = line[2..].trim().trim_matches('"').to_string();
            let target = if arg.is_empty() {
                dirs::home_dir().unwrap_or_default()
            } else if arg == "~" || arg.starts_with("~/") {
                dirs::home_dir().unwrap_or_default().join(arg.trim_start_matches("~/").trim_start_matches('~'))
            } else if arg == ".." {
                self.active().parent();
                return;
            } else {
                let p = PathBuf::from(&arg);
                if p.is_absolute() {
                    p
                } else {
                    self.active().path.join(p)
                }
            };
            self.active().goto(&target, None);
            return;
        }
        let cwd = self.active().path.clone();
        let as_path = cwd.join(&line);
        if as_path.is_dir() || Path::new(&line).is_dir() {
            let p = if Path::new(&line).is_dir() { PathBuf::from(&line) } else { as_path };
            self.active().goto(&p, None);
            return;
        }
        if let Err(e) = run_in_terminal(&cwd, &line) {
            self.message("Error", &format!("Cannot execute:\n{line}\n{e}"), true);
        }
        self.active().reload();
    }

    // ---------------------------------------------------------------- popups
    fn open_menu(&mut self, i: usize) {
        let i = i.min(MENUS.len() - 1);
        let items = MENUS[i]
            .items
            .iter()
            .map(|it| {
                if it.label == "-" {
                    PopupItem::Sep
                } else {
                    PopupItem::Item { label: it.label.to_string(), shortcut: it.shortcut.to_string(), enabled: it.cmd.is_some(), action: it.cmd.map(PopupAction::Cmd).unwrap_or(PopupAction::None) }
                }
            })
            .collect();
        let pos = self.menu_rects.get(i).map(|r| r.left_bottom()).unwrap_or(Pos2::new(xp::BORDER_W, xp::title_h() + MENU_H));
        self.popup = Some(Popup { items, pos, width: 300.0, menu_index: Some(i), hover: None, opened_at: self.now });
    }

    fn drive_popup(&mut self, side: Side) {
        let items = self
            .drives
            .iter()
            .map(|d| PopupItem::Item { label: format!("[{}]  {}", d.letter, d.label), shortcut: String::new(), enabled: true, action: PopupAction::Goto(side, d.root.clone()) })
            .collect();
        let x = if side == Side::Left { xp::BORDER_W + 4.0 } else { xp::BORDER_W + 4.0 + (self.left_width()) };
        self.popup = Some(Popup { items, pos: Pos2::new(x, xp::title_h() + MENU_H + TOOLBAR_H + DRIVEBAR_H + panel::DRIVE_ROW_H), width: 220.0, menu_index: None, hover: None, opened_at: self.now });
        self.active = side;
    }

    fn hotlist_popup(&mut self) {
        let side = self.active;
        let mut items: Vec<PopupItem> = self.cfg.hotlist.iter().map(|(n, p)| PopupItem::Item { label: n.clone(), shortcut: String::new(), enabled: p.is_dir(), action: PopupAction::Goto(side, p.clone()) }).collect();
        items.push(PopupItem::Sep);
        items.push(PopupItem::Item { label: "&Add current dir".into(), shortcut: String::new(), enabled: true, action: PopupAction::HotAdd });
        items.push(PopupItem::Item { label: "&Remove current dir".into(), shortcut: String::new(), enabled: true, action: PopupAction::HotRemove });
        let x = self.panel_x(side) + 8.0;
        self.popup = Some(Popup { items, pos: Pos2::new(x, xp::title_h() + MENU_H + TOOLBAR_H + DRIVEBAR_H + panel::DRIVE_ROW_H + panel::PATH_ROW_H), width: 260.0, menu_index: None, hover: None, opened_at: self.now });
    }

    fn dir_history_popup(&mut self) {
        let side = self.active;
        let hist: Vec<PathBuf> = self.active().history.iter().rev().take(20).cloned().collect();
        let items = hist.iter().map(|p| PopupItem::Item { label: fs::display(p), shortcut: String::new(), enabled: true, action: PopupAction::Goto(side, p.clone()) }).collect();
        let x = self.panel_x(side) + 8.0;
        self.popup = Some(Popup { items, pos: Pos2::new(x, xp::title_h() + MENU_H + TOOLBAR_H + DRIVEBAR_H + panel::DRIVE_ROW_H + panel::PATH_ROW_H), width: 320.0, menu_index: None, hover: None, opened_at: self.now });
    }

    fn cmd_history_popup(&mut self) {
        if self.cfg.cmd_history.is_empty() {
            return;
        }
        let items = self.cfg.cmd_history.iter().map(|c| PopupItem::Item { label: c.clone(), shortcut: String::new(), enabled: true, action: PopupAction::SetCmdLine(c.clone()) }).collect();
        self.popup = Some(Popup { items, pos: Pos2::new(xp::BORDER_W + 8.0, 0.0), width: 400.0, menu_index: None, hover: None, opened_at: self.now });
        // position above the command line once we know the screen height (set in draw_popup)
    }

    fn left_width(&self) -> f32 {
        0.0 // replaced at draw time
    }

    fn panel_x(&self, side: Side) -> f32 {
        if side == Side::Left {
            xp::BORDER_W
        } else {
            xp::BORDER_W + self.cfg.win_w / 2.0
        }
    }

    fn popup_action(&mut self, ctx: &Context, a: PopupAction) {
        match a {
            PopupAction::Cmd(c) => self.dispatch(ctx, c),
            PopupAction::Goto(side, p) => {
                self.active = side;
                self.panel(side).goto(&p, None);
            }
            PopupAction::SetCmdLine(s) => {
                self.cmd = s;
                self.cmd_caret = self.cmd.chars().count();
            }
            PopupAction::HotAdd => {
                let p = self.active().path.clone();
                let name = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| fs::display(&p));
                if !self.cfg.hotlist.iter().any(|(_, q)| q == &p) {
                    self.cfg.hotlist.push((name, p));
                }
            }
            PopupAction::HotRemove => {
                let p = self.active().path.clone();
                self.cfg.hotlist.retain(|(_, q)| q != &p);
            }
            PopupAction::None => {}
        }
    }

    // ---------------------------------------------------------------- keyboard
    fn handle_keys(&mut self, ctx: &Context) {
        let events = ctx.input(|i| i.events.clone());
        self.swallow_text = false;
        for e in events {
            match e {
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    let m = Mods { ctrl: modifiers.ctrl || modifiers.mac_cmd, alt: modifiers.alt, shift: modifiers.shift };
                    if std::env::var_os("TC_DEBUG").is_some() {
                        eprintln!("key {:?} {:?} -> {:?}", key, m, commands::lookup(m, key));
                    }
                    self.key(ctx, key, m);
                }
                egui::Event::Text(t) => {
                    if self.swallow_text {
                        continue;
                    }
                    for c in t.chars() {
                        if c.is_control() {
                            continue;
                        }
                        self.typed(ctx, c);
                    }
                }
                _ => {}
            }
        }
    }

    fn typed(&mut self, ctx: &Context, c: char) {
        if self.popup.is_some() {
            return;
        }
        if self.active().quick.is_some() {
            self.active().quick_type(c);
            return;
        }
        if self.cmd.is_empty() {
            match c {
                '*' => return self.dispatch(ctx, Cmd::InvertSelection),
                '+' => return self.dispatch(ctx, Cmd::SelectGroup),
                '-' => return self.dispatch(ctx, Cmd::UnselectGroup),
                _ => {}
            }
            if self.cfg.quick_search == QuickSearch::Letters && (c.is_alphanumeric() || c == '.' || c == '_') {
                self.active().quick = Some(String::new());
                self.active().quick_type(c);
                return;
            }
        }
        let s = c.to_string();
        self.cmd_insert(&s);
        self.cmd_hist_pos = None;
    }

    fn key(&mut self, ctx: &Context, key: Key, m: Mods) {
        // popup / menu navigation
        if self.popup.is_some() {
            self.popup_key(ctx, key, m);
            return;
        }
        // quick search mode
        let quick_mode_key = match self.cfg.quick_search {
            QuickSearch::CtrlAlt => m.ctrl && m.alt,
            QuickSearch::Alt => m.alt && !m.ctrl,
            QuickSearch::Letters => false,
        };
        if quick_mode_key {
            if let Some(c) = key_char(key, m.shift) {
                if self.active().quick.is_none() {
                    self.active().quick = Some(String::new());
                }
                self.active().quick_type(c);
                return;
            }
        }
        if self.active().quick.is_some() {
            match key {
                Key::Backspace => {
                    self.active().quick_backspace();
                    return;
                }
                Key::Escape => {
                    self.active().quick = None;
                    return;
                }
                Key::Enter | Key::ArrowUp | Key::ArrowDown | Key::Tab => {}
                _ if m.ctrl || m.alt => {}
                // printable keys: the Text event that follows extends the search
                Key::Space | Key::Period | Key::Minus | Key::Plus | Key::Comma | Key::Semicolon | Key::Quote | Key::Equals | Key::Backtick | Key::OpenBracket | Key::CloseBracket => return,
                _ if key_char(key, false).is_some() => return,
                _ => {
                    // navigation / function keys end quick search and still act
                    self.active().quick = None;
                }
            }
        }
        // command line editing keys when the line has text
        if !self.cmd.is_empty() && m == commands::NONE {
            match key {
                Key::Delete => {
                    self.cmd_delete();
                    return;
                }
                Key::Space | Key::Plus | Key::Minus => return, // printable: let the Text event through
                _ => {}
            }
        }
        if !self.cmd.is_empty() && key == Key::Space {
            return;
        }
        // Alt+letter opens a menu by mnemonic (unless quick search uses Alt)
        if m.alt && !m.ctrl && self.cfg.quick_search != QuickSearch::Alt {
            if let Some(c) = key_char(key, false) {
                for (i, menu) in MENUS.iter().enumerate() {
                    let (_, under) = xp::strip_mnemonic(menu.title);
                    let (t, _) = xp::strip_mnemonic(menu.title);
                    if let Some(u) = under {
                        if t.chars().nth(u).map(|x| x.to_ascii_lowercase()) == Some(c) {
                            self.open_menu(i);
                            return;
                        }
                    }
                }
            }
        }
        if let Some(cmd) = commands::lookup(m, key) {
            if matches!(key, Key::Space | Key::Plus | Key::Minus) {
                self.swallow_text = true;
            }
            self.dispatch(ctx, cmd);
        }
    }

    fn popup_key(&mut self, ctx: &Context, key: Key, m: Mods) {
        if matches!(key, Key::Space | Key::Plus | Key::Minus) {
            self.swallow_text = true;
        }
        let Some(p) = &mut self.popup else { return };
        let n = p.items.len();
        let is_item = |items: &Vec<PopupItem>, i: usize| matches!(items.get(i), Some(PopupItem::Item { enabled: true, .. }));
        match key {
            Key::Escape => self.popup = None,
            Key::ArrowDown => {
                let mut i = p.hover.map(|h| h + 1).unwrap_or(0);
                while i < n && !is_item(&p.items, i) {
                    i += 1;
                }
                if i < n {
                    p.hover = Some(i);
                }
            }
            Key::ArrowUp => {
                let mut i = p.hover.unwrap_or(n) as i64 - 1;
                while i >= 0 && !is_item(&p.items, i as usize) {
                    i -= 1;
                }
                if i >= 0 {
                    p.hover = Some(i as usize);
                }
            }
            Key::ArrowLeft | Key::ArrowRight => {
                if let Some(mi) = p.menu_index {
                    let k = MENUS.len();
                    let ni = if key == Key::ArrowLeft { (mi + k - 1) % k } else { (mi + 1) % k };
                    self.open_menu(ni);
                    if let Some(p) = &mut self.popup {
                        p.hover = Some(0);
                    }
                }
            }
            Key::Enter => {
                if let Some(h) = p.hover {
                    if let Some(PopupItem::Item { action, enabled: true, .. }) = p.items.get(h) {
                        let a = action.clone();
                        self.popup = None;
                        self.popup_action(ctx, a);
                    }
                }
            }
            _ => {
                if let Some(c) = key_char(key, false) {
                    if m.alt {
                        // switch menus by mnemonic
                        for (i, menu) in MENUS.iter().enumerate() {
                            let (t, under) = xp::strip_mnemonic(menu.title);
                            if under.and_then(|u| t.chars().nth(u)).map(|x| x.to_ascii_lowercase()) == Some(c) {
                                self.open_menu(i);
                                return;
                            }
                        }
                    }
                    for (i, it) in p.items.iter().enumerate() {
                        if let PopupItem::Item { label, enabled: true, action, .. } = it {
                            let (t, under) = xp::strip_mnemonic(label);
                            if under.and_then(|u| t.chars().nth(u)).map(|x| x.to_ascii_lowercase()) == Some(c) {
                                let a = action.clone();
                                let _ = i;
                                self.popup = None;
                                self.swallow_text = true;
                                self.popup_action(ctx, a);
                                return;
                            }
                        }
                    }
                    // any other letter while a menu is open: don't let it fall into the command line
                    self.swallow_text = true;
                }
            }
        }
    }

    fn quick_prev(&mut self) {
        let p = self.active();
        if let Some(q) = p.quick.clone() {
            let q = q.to_lowercase();
            let n = p.entries.len();
            for k in 1..=n {
                let i = (p.cursor + n - k) % n;
                if p.entries[i].name.to_lowercase().starts_with(&q) {
                    p.cursor = i;
                    p.ensure_visible();
                    return;
                }
            }
        }
    }

    // ---------------------------------------------------------------- dialogs / ops
    fn act(&mut self, ctx: &Context, a: Action) {
        match a {
            Action::Close => self.dialog = None,
            Action::StartOp(req) => {
                self.dialog = Some(Dialog::Progress);
                self.op = Some(ops::start(req));
            }
            Action::MkDir(name) => {
                self.dialog = None;
                let p = self.active().path.join(&name);
                match std::fs::create_dir_all(&p) {
                    Ok(()) => {
                        self.reload_both();
                        let first = name.split(['/', '\\']).next().unwrap_or(&name).to_string();
                        let pan = self.active();
                        if let Some(i) = pan.entries.iter().position(|e| e.name == first) {
                            pan.cursor = i;
                            pan.ensure_visible();
                        }
                    }
                    Err(e) => self.message("Error", &format!("Cannot create directory\n{}\n{}", fs::display(&p), fs::err_text(&e)), true),
                }
            }
            Action::Rename(old, name) => {
                self.dialog = None;
                let new = old.parent().map(|p| p.join(&name)).unwrap_or(PathBuf::from(&name));
                if new.exists() && new != old {
                    self.message("Error", &format!("Cannot rename: {} already exists.", name), true);
                    return;
                }
                match std::fs::rename(&old, &new) {
                    Ok(()) => {
                        self.reload_both();
                        let pan = self.active();
                        if let Some(i) = pan.entries.iter().position(|e| e.name == name) {
                            pan.cursor = i;
                            pan.ensure_visible();
                        }
                    }
                    Err(e) => self.message("Error", &format!("Cannot rename\n{}\n{}", fs::display(&old), fs::err_text(&e)), true),
                }
            }
            Action::Reply(r) => {
                if let Some(op) = &mut self.op {
                    op.reply(r);
                }
                self.dialog = Some(Dialog::Progress);
            }
            Action::CancelOp => {
                if let Some(op) = &self.op {
                    op.cancel();
                }
            }
            Action::GotoFile(p) => {
                self.dialog = None;
                let dir = p.parent().map(|d| d.to_path_buf()).unwrap_or_default();
                let name = p.file_name().map(|s| s.to_string_lossy().to_string());
                self.active().goto(&dir, name.as_deref());
            }
            Action::FeedToListbox(paths, title) => {
                self.dialog = None;
                let pan = self.active();
                let mut v = Vec::new();
                for p in paths {
                    let Ok(meta) = std::fs::metadata(&p) else { continue };
                    v.push(fs::Entry {
                        name: fs::display(&p),
                        path: p.clone(),
                        is_dir: meta.is_dir(),
                        is_up: false,
                        size: if meta.is_dir() { 0 } else { meta.len() },
                        modified: meta.modified().ok().map(chrono::DateTime::<chrono::Local>::from),
                        hidden: false,
                        readonly: meta.permissions().readonly(),
                        system: false,
                        archive: !meta.is_dir(),
                        symlink: false,
                        dir_size: None,
                    });
                }
                pan.entries = v;
                pan.cursor = 0;
                pan.scroll = 0;
                pan.selected.clear();
                pan.list_title = Some(title);
            }
            Action::SelectGroup { mask, on } => {
                self.dialog = None;
                self.active().select_mask(&mask, on, false);
            }
            Action::SetFilter(mask) => {
                self.dialog = None;
                self.active().filter = mask;
                self.active().reload();
            }
            Action::ApplyOptions(o) => {
                self.dialog = None;
                self.cfg.nag = o.nag;
                self.cfg.lynx = o.lynx;
                self.cfg.right_click_selects = o.right_click_selects;
                self.cfg.quick_search = o.quick_search;
                self.cfg.editor = o.editor;
                if self.cfg.theme != o.theme {
                    self.cfg.theme = o.theme;
                    xp::set_theme(o.theme);
                    xp::apply_egui_visuals(ctx);
                }
                if self.cfg.show_hidden != o.show_hidden {
                    self.dispatch(ctx, Cmd::ToggleHidden);
                }
                self.cfg.save();
            }
            Action::EditNew(name) => {
                self.dialog = None;
                let p = self.active().path.join(&name);
                if !p.exists() {
                    if let Err(e) = std::fs::write(&p, b"") {
                        self.message("Error", &format!("Cannot create\n{}\n{}", fs::display(&p), fs::err_text(&e)), true);
                        return;
                    }
                    self.active().reload();
                }
                self.edit_file(&p);
            }
            Action::OpenUrl(u) => {
                let _ = open::that_detached(u);
            }
        }
    }

    fn poll_op(&mut self) {
        let Some(op) = &mut self.op else { return };
        if op.poll() {
            let fin = op.finished.take();
            let kind = op.kind;
            self.op = None;
            if matches!(self.dialog, Some(Dialog::Progress) | Some(Dialog::Overwrite { .. })) {
                self.dialog = None;
            }
            if let Some(Msg::Error(e)) = &fin {
                let e = e.clone();
                self.message("Error", &e, true);
            }
            if matches!(kind, OpKind::Copy | OpKind::Move | OpKind::Delete | OpKind::Pack | OpKind::Unpack) {
                if matches!(fin, Some(Msg::Done)) && kind != OpKind::Pack {
                    self.active().selected.clear();
                }
                self.reload_both();
            }
        } else if let Some(q) = op.pending.take() {
            self.dialog = Some(Dialog::Overwrite { q });
        }
    }

    fn store_config(&mut self, ctx: &Context) {
        let size = ctx.input(|i| i.viewport().inner_rect.map(|r| r.size()));
        if let Some(s) = size {
            self.cfg.win_w = s.x;
            self.cfg.win_h = s.y;
        }
        self.cfg.maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        for (c, p) in [(&mut self.cfg.left, &self.left), (&mut self.cfg.right, &self.right)] {
            c.path = p.path.clone();
            c.sort = p.sort;
            c.reverse = p.reverse;
            c.brief = p.view == panel::View::Brief;
        }
    }

    fn handle_panel_events(&mut self, ctx: &Context, side: Side, ev: Vec<PanelEvent>) {
        for e in ev {
            match e {
                PanelEvent::Activate => {
                    if self.active != side {
                        self.active().quick = None;
                    }
                    self.active = side;
                }
                PanelEvent::OpenFile(p) => {
                    self.active = side;
                    self.open_file(&p);
                }
                PanelEvent::DriveMenu => {
                    self.active = side;
                    self.drive_popup(side);
                }
                PanelEvent::ContextMenu(_) => {
                    self.active = side;
                    let items = vec![
                        PopupItem::Item { label: "&View  F3".into(), shortcut: String::new(), enabled: true, action: PopupAction::Cmd(Cmd::View) },
                        PopupItem::Item { label: "&Edit  F4".into(), shortcut: String::new(), enabled: true, action: PopupAction::Cmd(Cmd::Edit) },
                        PopupItem::Sep,
                        PopupItem::Item { label: "&Copy  F5".into(), shortcut: String::new(), enabled: true, action: PopupAction::Cmd(Cmd::Copy) },
                        PopupItem::Item { label: "&Move  F6".into(), shortcut: String::new(), enabled: true, action: PopupAction::Cmd(Cmd::Move) },
                        PopupItem::Item { label: "&Rename  Shift+F6".into(), shortcut: String::new(), enabled: true, action: PopupAction::Cmd(Cmd::Rename) },
                        PopupItem::Item { label: "&Delete  F8".into(), shortcut: String::new(), enabled: true, action: PopupAction::Cmd(Cmd::Delete) },
                        PopupItem::Sep,
                        PopupItem::Item { label: "P&roperties  Alt+Enter".into(), shortcut: String::new(), enabled: true, action: PopupAction::Cmd(Cmd::Properties) },
                    ];
                    let pos = ctx.input(|i| i.pointer.interact_pos()).unwrap_or(Pos2::new(100.0, 100.0));
                    self.popup = Some(Popup { items, pos, width: 200.0, menu_index: None, hover: None, opened_at: self.now });
                }
            }
        }
        let _ = ctx;
    }

    // ---------------------------------------------------------------- drawing
    fn draw_menubar(&mut self, ui: &mut Ui, r: Rect) {
        let p = ui.painter();
        xp::fill(p, r, xp::face());
        let mut x = r.left() + 2.0;
        self.menu_rects.clear();
        let open = self.popup.as_ref().and_then(|p| p.menu_index);
        let mut switch_to = None;
        for (i, m) in MENUS.iter().enumerate() {
            let (t, _) = xp::strip_mnemonic(m.title);
            let w = xp::text_width(p, &t) + 14.0;
            let mr = Rect::from_min_size(Pos2::new(x, r.top() + 1.0), Vec2::new(w, MENU_H - 2.0));
            self.menu_rects.push(mr);
            let resp = ui.interact(mr, ui.id().with(("menu", i)), Sense::CLICK);
            let p = ui.painter();
            let hi = open == Some(i);
            if hi {
                xp::fill(p, mr, xp::highlight());
                xp::mnemonic_text(p, mr.center(), Align2::CENTER_CENTER, m.title, xp::font(), xp::HIGHLIGHT_TEXT);
            } else {
                if resp.hovered() {
                    xp::fill(p, mr, xp::rgb(0xF6, 0xF7, 0xFD));
                    xp::frame(p, mr, xp::highlight());
                }
                xp::mnemonic_text(p, mr.center(), Align2::CENTER_CENTER, m.title, xp::font(), xp::TEXT);
            }
            if resp.clicked() {
                switch_to = Some(if hi { None } else { Some(i) });
            } else if open.is_some() && resp.hovered() && !hi {
                switch_to = Some(Some(i));
            }
            x += w;
        }
        xp::hseparator(ui.painter(), r.left(), r.right(), r.bottom() - 2.0);
        if let Some(s) = switch_to {
            match s {
                Some(i) => self.open_menu(i),
                None => self.popup = None,
            }
        }
    }

    fn draw_toolbar(&mut self, ui: &mut Ui, r: Rect) -> Option<Cmd> {
        let p = ui.painter();
        xp::fill(p, r, xp::face());
        let brief = self.active().view == panel::View::Brief;
        let items: Vec<(Option<&egui::TextureHandle>, &str, Option<Cmd>, bool)> = vec![
            (Some(&self.icons.tb_refresh), "Reread source", Some(Cmd::Reread), false),
            (Some(&self.icons.tb_brief), "Brief", Some(Cmd::Brief), brief),
            (Some(&self.icons.tb_full), "Full", Some(Cmd::Full), !brief),
            (Some(&self.icons.tb_tree), "Tree", Some(Cmd::TreeView), false),
            (Some(&self.icons.tb_quickview), "Quick view (Ctrl+Q)", Some(Cmd::QuickView), self.quick_view),
            (None, "", None, false),
            (Some(&self.icons.tb_search), "Find files (Alt+F7)", Some(Cmd::Find), false),
            (Some(&self.icons.tb_sync), "Synchronize dirs", None, false),
            (None, "", None, false),
            (Some(&self.icons.tb_pack), "Pack (Alt+F5)", Some(Cmd::Pack), false),
            (Some(&self.icons.tb_unpack), "Unpack (Alt+F9)", Some(Cmd::Unpack), false),
            (Some(&self.icons.tb_compare), "Compare by content", Some(Cmd::CompareContent), false),
            (None, "", None, false),
            (Some(&self.icons.tb_notepad), "Edit new file (Shift+F4)", Some(Cmd::EditNew), false),
            (Some(&self.icons.tb_cmd), "Open command prompt window", Some(Cmd::OpenTerminal), false),
        ];
        let mut x = r.left() + 4.0;
        let mut out = None;
        let mut tooltip = None;
        for (i, (tex, tip, cmd, checked)) in items.iter().enumerate() {
            let Some(tex) = tex else {
                xp::vseparator(ui.painter(), x + 3.0, r.top() + 4.0, r.bottom() - 4.0);
                x += 8.0;
                continue;
            };
            let br = Rect::from_min_size(Pos2::new(x, r.top() + 3.0), Vec2::new(23.0, 22.0));
            let resp = ui.interact(br, ui.id().with(("tb", i)), Sense::CLICK);
            let enabled = cmd.is_some();
            xp::tool_button(ui.painter(), br, resp.hovered() && enabled, resp.is_pointer_button_down_on() && enabled, *checked);
            icons::draw(ui.painter(), tex, br.min + Vec2::new(3.0, 3.0));
            if !enabled {
                ui.painter().rect_filled(br.shrink(1.0), 0.0, Color32::from_rgba_unmultiplied(0xEC, 0xE9, 0xD8, 160));
            }
            if resp.hovered() {
                tooltip = Some((br, tip.to_string()));
            }
            if resp.clicked() {
                out = *cmd;
            }
            x += 24.0;
        }
        xp::hseparator(ui.painter(), r.left(), r.right(), r.bottom() - 2.0);
        if let Some((br, tip)) = tooltip {
            self.tooltip(ui, br, &tip);
        }
        out
    }

    fn tooltip(&self, ui: &mut Ui, anchor: Rect, text: &str) {
        let id = ui.id().with("tip");
        let now = ui.input(|i| i.time);
        let start = ui.data_mut(|d| *d.get_temp_mut_or_insert_with(id, || now));
        if now - start < 0.5 {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
            return;
        }
        let p = ui.painter();
        let w = xp::text_width(p, text) + 10.0;
        let r = Rect::from_min_size(Pos2::new(anchor.left(), anchor.bottom() + 6.0), Vec2::new(w, 18.0));
        xp::fill(p, r, xp::TOOLTIP);
        xp::frame(p, r, xp::TEXT);
        xp::text(p, r.min + Vec2::new(5.0, 3.0), text, xp::TEXT);
    }

    fn draw_drivebar(&mut self, ui: &mut Ui, r: Rect, side: Side) {
        let p = ui.painter();
        xp::fill(p, r, xp::face());
        let cur = self.panel(side).path.clone();
        let drives = self.drives.clone();
        let current = drives.iter().filter(|d| cur.starts_with(&d.root)).max_by_key(|d| d.root.as_os_str().len()).map(|d| d.root.clone());
        let mut x = r.left() + 3.0;
        for (i, d) in drives.iter().enumerate() {
            let w = 16.0 + xp::text_width(ui.painter(), &d.letter) + 10.0;
            let br = Rect::from_min_size(Pos2::new(x, r.top() + 2.0), Vec2::new(w, 22.0));
            if br.right() > r.right() - 2.0 {
                break;
            }
            let resp = ui.interact(br, ui.id().with(("drv", side == Side::Left, i)), Sense::CLICK);
            let checked = current.as_ref() == Some(&d.root);
            xp::tool_button(ui.painter(), br, resp.hovered(), resp.is_pointer_button_down_on(), checked);
            icons::draw(ui.painter(), panel::drive_icon(&self.icons, d.kind), br.min + Vec2::new(3.0, 3.0));
            xp::text(ui.painter(), br.min + Vec2::new(21.0, 5.0), &d.letter, xp::TEXT);
            if resp.clicked() {
                self.active = side;
                self.panel(side).goto(&d.root, None);
            }
            x += w + 1.0;
        }
    }

    fn draw_cmdline(&mut self, ui: &mut Ui, r: Rect) {
        let p = ui.painter();
        xp::fill(p, r, xp::face());
        let prompt = format!("{}>", fs::display(&self.active().path));
        let max_w = (r.width() * 0.4).max(60.0);
        let shown = xp::ellipsize_path(p, &prompt, max_w, xp::font());
        let pw = xp::text_width(p, &shown);
        xp::text(p, r.min + Vec2::new(4.0, 5.0), &shown, xp::TEXT);
        let er = Rect::from_min_max(Pos2::new(r.left() + 8.0 + pw, r.top() + 2.0), Pos2::new(r.right() - 4.0, r.bottom() - 2.0));
        xp::sunken(p, er);
        let tp = ui.painter_at(er.shrink(1.0));
        let text_x = er.left() + 3.0;
        tp.text(Pos2::new(text_x, er.top() + 3.0), Align2::LEFT_TOP, &self.cmd, xp::font(), xp::TEXT);
        // caret
        let blink = ui.input(|i| i.time) % 1.0 < 0.5;
        if blink && self.dialog.is_none() {
            let before: String = self.cmd.chars().take(self.cmd_caret).collect();
            let cx = text_x + xp::text_width(&tp, &before);
            tp.rect_filled(Rect::from_min_size(Pos2::new(cx.round(), er.top() + 3.0), Vec2::new(1.0, 13.0)), 0.0, xp::TEXT);
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
        let resp = ui.interact(er, ui.id().with("cmdline"), Sense::CLICK);
        if resp.clicked() {
            self.cmd_caret = self.cmd.chars().count();
        }
    }

    fn draw_fkeys(&mut self, ui: &mut Ui, r: Rect) -> Option<Cmd> {
        let p = ui.painter();
        xp::fill(p, r, xp::face());
        let shift = ui.input(|i| i.modifiers.shift);
        let keys: [(&str, Cmd); 7] = if shift {
            [("F3 View", Cmd::ViewCursor), ("F4 Edit", Cmd::EditNew), ("F5 Copy", Cmd::CopySameDir), ("F6 Rename", Cmd::Rename), ("F7 NewFolder", Cmd::MkDir), ("F8 Delete", Cmd::Delete), ("Alt+F4 Exit", Cmd::Exit)]
        } else {
            [("F3 View", Cmd::View), ("F4 Edit", Cmd::Edit), ("F5 Copy", Cmd::Copy), ("F6 Move", Cmd::Move), ("F7 NewFolder", Cmd::MkDir), ("F8 Delete", Cmd::Delete), ("Alt+F4 Exit", Cmd::Exit)]
        };
        let n = keys.len() as f32;
        let w = ((r.width() - 4.0) / n).floor();
        let mut out = None;
        for (i, (label, cmd)) in keys.iter().enumerate() {
            let br = Rect::from_min_size(Pos2::new(r.left() + 2.0 + i as f32 * w, r.top() + 1.0), Vec2::new(w - 2.0, FKEY_H - 3.0));
            let resp = ui.interact(br, ui.id().with(("fk", i)), Sense::CLICK);
            xp::button_face(ui.painter(), br, resp.hovered(), resp.is_pointer_button_down_on(), false, true);
            ui.painter().text(br.center(), Align2::CENTER_CENTER, label, xp::font(), xp::TEXT);
            if resp.clicked() {
                out = Some(*cmd);
            }
        }
        out
    }

    fn draw_popup(&mut self, ctx: &Context, screen: Rect) {
        let Some(popup) = &mut self.popup else { return };
        let item_h = 18.0;
        let h: f32 = popup.items.iter().map(|it| if matches!(it, PopupItem::Sep) { 7.0 } else { item_h }).sum::<f32>() + 6.0;
        let mut pos = popup.pos;
        if pos.y == 0.0 {
            pos.y = screen.bottom() - xp::BORDER_W - FKEY_H - CMD_H - h;
        }
        if pos.y + h > screen.bottom() {
            pos.y = (screen.bottom() - h).max(0.0);
        }
        if pos.x + popup.width > screen.right() {
            pos.x = (screen.right() - popup.width).max(0.0);
        }
        let r = Rect::from_min_size(pos, Vec2::new(popup.width, h));
        let items = popup.items.clone();
        let opened = popup.opened_at;
        let hover_kbd = popup.hover;
        let mut clicked: Option<PopupAction> = None;
        let mut new_hover = hover_kbd;
        let area = egui::Area::new(egui::Id::new("popup")).order(egui::Order::Foreground).fixed_pos(pos).show(ctx, |ui| {
            let (_, _) = ui.allocate_space(r.size());
            let p = ui.painter();
            xp::menu_frame(p, r);
            let mut y = r.top() + 3.0;
            for (i, it) in items.iter().enumerate() {
                match it {
                    PopupItem::Sep => {
                        xp::menu_separator(p, r, y + 3.0);
                        y += 7.0;
                    }
                    PopupItem::Item { label, shortcut, enabled, action } => {
                        let ir = Rect::from_min_size(Pos2::new(r.left() + 2.0, y), Vec2::new(r.width() - 4.0, item_h));
                        let resp = ui.interact(ir, ui.id().with(("pi", i)), Sense::CLICK);
                        if resp.hovered() {
                            new_hover = Some(i);
                        }
                        let hi = new_hover == Some(i) && *enabled;
                        let p = ui.painter();
                        if hi {
                            xp::fill(p, ir, xp::highlight());
                        }
                        let fg = if !*enabled { xp::graytext() } else if hi { xp::HIGHLIGHT_TEXT } else { xp::TEXT };
                        xp::mnemonic_text(p, Pos2::new(ir.left() + 28.0, ir.center().y), Align2::LEFT_CENTER, label, xp::font(), fg);
                        p.text(Pos2::new(ir.right() - 12.0, ir.center().y), Align2::RIGHT_CENTER, shortcut, xp::font(), fg);
                        if resp.clicked() && *enabled {
                            clicked = Some(action.clone());
                        }
                        y += item_h;
                    }
                }
            }
        });
        let _ = area;
        if let Some(p) = &mut self.popup {
            p.hover = new_hover;
        }
        if let Some(a) = clicked {
            self.popup = None;
            self.popup_action(ctx, a);
            return;
        }
        // click outside closes (ignore the click that opened it)
        let (pressed, pos_opt, now) = ctx.input(|i| (i.pointer.any_pressed(), i.pointer.interact_pos(), i.time));
        if pressed && now > opened {
            if let Some(pp) = pos_opt {
                let in_menubar = self.menu_rects.iter().any(|m| m.contains(pp));
                if !r.contains(pp) && !in_menubar {
                    self.popup = None;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------- window frame
/// Paint an XP window frame + caption inside `screen`; returns the client rect and whether close was clicked.
pub fn frame_window(ui: &mut Ui, screen: Rect, title: &str, icon: &egui::TextureHandle, maximized: bool) -> (Rect, bool) {
    let p = ui.painter();
    let active = ui.input(|i| i.viewport().focused.unwrap_or(true));
    xp::window_frame(p, screen, active);
    let l = xp::window_layout(screen);
    let client = l.client;
    let cap = l.cap;
    let (min, max, close) = (l.min, l.max, l.close);
    icons::draw(p, icon, l.icon);
    xp::caption_title(p, &l, title, active);
    let mut closed = false;
    for (r, kind, name) in [(min, xp::CaptionButton::Minimize, "min"), (max, if maximized { xp::CaptionButton::Restore } else { xp::CaptionButton::Maximize }, "max"), (close, xp::CaptionButton::Close, "close")] {
        let resp = ui.interact(r, ui.id().with(("cap", name)), Sense::CLICK);
        xp::caption_button(ui.painter(), r, kind, resp.hovered(), resp.is_pointer_button_down_on());
        if resp.clicked() {
            match kind {
                xp::CaptionButton::Minimize => ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true)),
                xp::CaptionButton::Maximize => ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(true)),
                xp::CaptionButton::Restore => ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(false)),
                xp::CaptionButton::Close => closed = true,
            }
        }
    }
    // drag / double-click on the caption
    let drag_r = Rect::from_min_max(cap.min, Pos2::new(min.left() - 4.0, cap.bottom()));
    let dr = ui.interact(drag_r, ui.id().with("capdrag"), Sense::CLICK | Sense::DRAG);
    if dr.drag_started() {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if dr.double_clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }
    // resize edges
    if !maximized {
        use egui::viewport::ResizeDirection as D;
        let b = xp::BORDER_W;
        let edges = [
            (Rect::from_min_max(Pos2::new(screen.left() + b, screen.top()), Pos2::new(screen.right() - b, screen.top() + b)), D::North, egui::CursorIcon::ResizeNorth),
            (Rect::from_min_max(Pos2::new(screen.left() + b, screen.bottom() - b), Pos2::new(screen.right() - b, screen.bottom())), D::South, egui::CursorIcon::ResizeSouth),
            (Rect::from_min_max(Pos2::new(screen.left(), screen.top() + b), Pos2::new(screen.left() + b, screen.bottom() - b)), D::West, egui::CursorIcon::ResizeWest),
            (Rect::from_min_max(Pos2::new(screen.right() - b, screen.top() + b), Pos2::new(screen.right(), screen.bottom() - b)), D::East, egui::CursorIcon::ResizeEast),
            (Rect::from_min_size(screen.min, Vec2::splat(b)), D::NorthWest, egui::CursorIcon::ResizeNwSe),
            (Rect::from_min_size(Pos2::new(screen.right() - b, screen.top()), Vec2::splat(b)), D::NorthEast, egui::CursorIcon::ResizeNeSw),
            (Rect::from_min_size(Pos2::new(screen.left(), screen.bottom() - b), Vec2::splat(b)), D::SouthWest, egui::CursorIcon::ResizeNeSw),
            (Rect::from_min_size(Pos2::new(screen.right() - b, screen.bottom() - b), Vec2::splat(b)), D::SouthEast, egui::CursorIcon::ResizeNwSe),
        ];
        for (i, (r, dir, cursor)) in edges.into_iter().enumerate() {
            let resp = ui.interact(r, ui.id().with(("edge", i)), Sense::CLICK | Sense::DRAG);
            if resp.hovered() || resp.dragged() {
                ui.ctx().set_cursor_icon(cursor);
            }
            if resp.drag_started() {
                ui.ctx().send_viewport_cmd(ViewportCommand::BeginResize(dir));
            }
        }
    }
    (client, closed)
}

impl eframe::App for App {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        self.frame_no += 1;
        self.now = ctx.input(|i| i.time);
        // Alt+letter is a mnemonic everywhere (menus, dialog buttons); the letter must never
        // land in a focused edit box as text. AltGr (Ctrl+Alt) still types its characters.
        // (The modifier state may already be released by the time the frame runs, so look at
        // the Key events themselves rather than at the current modifiers.)
        ctx.input_mut(|i| {
            let alt_keys: Vec<String> = i
                .events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Key { key, pressed: true, modifiers, .. } if modifiers.alt && !modifiers.ctrl => Some(key.name().to_lowercase()),
                    _ => None,
                })
                .collect();
            if !alt_keys.is_empty() {
                i.events.retain(|e| match e {
                    egui::Event::Text(t) => !alt_keys.contains(&t.to_lowercase()),
                    _ => true,
                });
            }
        });
        self.poll_op();
        if self.lister.is_none() && self.dialog.is_none() {
            self.handle_keys(ctx);
        }
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let mut pending_cmd: Option<Cmd> = None;
        let mut pending_action: Option<Action> = None;
        let mut events: Vec<(Side, Vec<PanelEvent>)> = Vec::new();
        let mut screen_rect = root.max_rect();
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(root, |ui| {
            let screen = ui.max_rect();
            screen_rect = screen;
            self.cfg.win_w = screen.width();
            let (client, closed) = frame_window(ui, screen, &format!("total-commacer {}", env!("CARGO_PKG_VERSION")), &self.icons.app, maximized);
            if closed {
                debug("exit: caption close button");
                self.exit_requested = true;
            }
            let mut y = client.top();
            let row = |y: &mut f32, h: f32| {
                let r = Rect::from_min_size(Pos2::new(client.left(), *y), Vec2::new(client.width(), h));
                *y += h;
                r
            };
            let menubar = row(&mut y, MENU_H);
            let toolbar = row(&mut y, TOOLBAR_H);
            let drivebar = row(&mut y, DRIVEBAR_H);
            let fkeys = Rect::from_min_size(Pos2::new(client.left(), client.bottom() - FKEY_H), Vec2::new(client.width(), FKEY_H));
            let cmdline = Rect::from_min_size(Pos2::new(client.left(), fkeys.top() - CMD_H), Vec2::new(client.width(), CMD_H));
            let panels = Rect::from_min_max(Pos2::new(client.left(), y), Pos2::new(client.right(), cmdline.top() - 2.0));
            let half = (panels.width() / 2.0).floor();
            let lrect = Rect::from_min_max(panels.min, Pos2::new(panels.left() + half - 2.0, panels.bottom()));
            let rrect = Rect::from_min_max(Pos2::new(panels.left() + half + 2.0, panels.top()), panels.max);

            self.draw_menubar(ui, menubar);
            if let Some(c) = self.draw_toolbar(ui, toolbar) {
                pending_cmd = Some(c);
            }
            let dl = Rect::from_min_max(drivebar.min, Pos2::new(drivebar.left() + half, drivebar.bottom()));
            let dr = Rect::from_min_max(Pos2::new(drivebar.left() + half, drivebar.top()), drivebar.max);
            self.draw_drivebar(ui, dl, Side::Left);
            self.draw_drivebar(ui, dr, Side::Right);
            // splitter
            ui.painter().rect_filled(Rect::from_min_max(Pos2::new(lrect.right(), panels.top()), Pos2::new(rrect.left(), panels.bottom())), 0.0, xp::face());

            let active = self.active;
            let rcs = self.cfg.right_click_selects;
            let drives = self.drives.clone();
            // quick view replaces the inactive panel
            if self.quick_view {
                let cur = self.active().cursor_entry().filter(|e| !e.is_dir).map(|e| e.path.clone());
                if cur != self.quick_path {
                    self.quick_path = cur.clone();
                    self.quick_lister = cur.as_deref().and_then(|p| Lister::open(p).ok());
                }
            }
            let (a_rect, o_rect) = if active == Side::Left { (lrect, rrect) } else { (rrect, lrect) };
            let other = if active == Side::Left { Side::Right } else { Side::Left };
            let icons = &self.icons;
            let (a, o) = match active {
                Side::Left => (&mut self.left, &mut self.right),
                Side::Right => (&mut self.right, &mut self.left),
            };
            let ev = a.ui(ui, a_rect, true, icons, rcs, &drives);
            events.push((active, ev));
            if self.quick_view {
                let p = ui.painter().clone();
                xp::sunken(&p, o_rect);
                if let Some(l) = &mut self.quick_lister {
                    l.ui(ui, o_rect.shrink(1.0), false);
                } else {
                    xp::text(&p, o_rect.min + Vec2::new(8.0, 8.0), "Quick view: put the cursor on a file.", xp::TEXT);
                }
            } else {
                let ev = o.ui(ui, o_rect, false, icons, rcs, &drives);
                events.push((other, ev));
            }
            self.draw_cmdline(ui, cmdline);
            if let Some(c) = self.draw_fkeys(ui, fkeys) {
                pending_cmd = Some(c);
            }
            // modal dialog on top
            if self.dialog.is_some() {
                let blocker = egui::Area::new(egui::Id::new("modal-blocker")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
                    let (r, _) = ui.allocate_exact_size(screen.size(), Sense::CLICK | Sense::DRAG);
                    let _ = r;
                    let op = self.op.as_ref();
                    // the same input is replayed on egui's second pass: "fresh" is per input frame, by time
                    let now = ctx.input(|i| i.time);
                    let fresh = match self.dialog_seen_at {
                        None => {
                            self.dialog_seen_at = Some(now);
                            true
                        }
                        Some(t) => t == now,
                    };
                    if let Some(d) = &mut self.dialog {
                        pending_action = d.show(ui, screen, op, fresh);
                    }
                });
                let _ = blocker;
            }
        });
        if self.dialog.is_none() {
            self.dialog_seen_at = None;
        }
        for (side, ev) in events {
            self.handle_panel_events(ctx, side, ev);
        }
        if let Some(c) = pending_cmd {
            self.dispatch(ctx, c);
        }
        if let Some(a) = pending_action {
            self.act(ctx, a);
        }
        self.draw_popup(ctx, screen_rect);
        // panel errors -> message box
        for side in [Side::Left, Side::Right] {
            if let Some(e) = self.panel(side).error.take() {
                if self.dialog.is_none() {
                    self.message("Error", &e, true);
                }
            }
        }
        // Lister in its own window
        if let Some(lister) = &mut self.lister {
            let title = lister.title();
            let icon = &self.icons.file_text;
            let id = egui::ViewportId::from_hash_of("lister");
            ctx.show_viewport_immediate(id, egui::ViewportBuilder::default().with_title(&title).with_decorations(false).with_inner_size([760.0, 540.0]).with_min_inner_size([300.0, 200.0]), |vroot, _class| {
                let vctx = vroot.ctx().clone();
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(vroot, |ui| {
                    let screen = ui.max_rect();
                    let maximized = vctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    let (client, closed) = frame_window(ui, screen, &title, icon, maximized);
                    if closed {
                        lister.close = true;
                    }
                    let focused = vctx.input(|i| i.viewport().focused.unwrap_or(true));
                    lister.ui(ui, client, focused);
                });
                if vctx.input(|i| i.viewport().close_requested()) {
                    lister.close = true;
                }
            });
            if lister.close {
                self.lister = None;
                ctx.request_repaint();
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            debug("exit: close_requested");
            self.exit_requested = true;
        }
        if self.exit_requested {
            self.store_config(ctx);
            self.cfg.save();
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.cfg.save();
    }
}

fn debug(msg: &str) {
    if std::env::var_os("TC_DEBUG").is_some() {
        eprintln!("{msg}");
    }
}

fn byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map(|(i, _)| i).unwrap_or(s.len())
}

fn quote(s: &str) -> String {
    if s.contains(' ') {
        format!("\"{s}\"")
    } else {
        s.to_string()
    }
}

/// The character a key produces, for quick search and mnemonics (ASCII letters/digits only).
fn key_char(key: Key, shift: bool) -> Option<char> {
    let name = key.name();
    let mut chars = name.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return match key {
            Key::Period => Some('.'),
            Key::Minus => Some('-'),
            Key::Num0 => Some('0'),
            Key::Num1 => Some('1'),
            Key::Num2 => Some('2'),
            Key::Num3 => Some('3'),
            Key::Num4 => Some('4'),
            Key::Num5 => Some('5'),
            Key::Num6 => Some('6'),
            Key::Num7 => Some('7'),
            Key::Num8 => Some('8'),
            Key::Num9 => Some('9'),
            _ => None,
        };
    }
    if c.is_ascii_alphabetic() {
        Some(if shift { c.to_ascii_uppercase() } else { c.to_ascii_lowercase() })
    } else if c.is_ascii_digit() {
        Some(c)
    } else {
        None
    }
}

fn run_in_terminal(dir: &Path, line: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        return Command::new("cmd").args(["/C", "start", "", "/D"]).arg(dir).args(["cmd", "/K", line]).spawn().map(|_| ()).map_err(|e| e.to_string());
    }
    #[cfg(target_os = "macos")]
    {
        let script = format!("tell application \"Terminal\" to do script \"cd {} && {}\"", dir.to_string_lossy().replace('"', "\\\""), line.replace('"', "\\\""));
        return Command::new("osascript").args(["-e", &script]).spawn().map(|_| ()).map_err(|e| e.to_string());
    }
    #[allow(unreachable_code)]
    {
        let cmdline = format!("{line}; echo; echo '[press Enter to close]'; read _");
        for term in ["x-terminal-emulator", "gnome-terminal", "konsole", "xfce4-terminal", "xterm"] {
            let r = if term == "gnome-terminal" {
                Command::new(term).arg("--").args(["sh", "-c", &cmdline]).current_dir(dir).spawn()
            } else {
                Command::new(term).arg("-e").args(["sh", "-c", &cmdline]).current_dir(dir).spawn()
            };
            if r.is_ok() {
                return Ok(());
            }
        }
        Command::new("sh").args(["-c", line]).current_dir(dir).spawn().map(|_| ()).map_err(|e| e.to_string())
    }
}

fn open_terminal(dir: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        return Command::new("cmd").args(["/C", "start", "", "/D"]).arg(dir).arg("cmd").spawn().map(|_| ()).map_err(|e| e.to_string());
    }
    #[cfg(target_os = "macos")]
    {
        return Command::new("open").arg("-a").arg("Terminal").arg(dir).spawn().map(|_| ()).map_err(|e| e.to_string());
    }
    #[allow(unreachable_code)]
    {
        for term in ["x-terminal-emulator", "gnome-terminal", "konsole", "xfce4-terminal", "xterm"] {
            if Command::new(term).current_dir(dir).spawn().is_ok() {
                return Ok(());
            }
        }
        Err("No terminal emulator found.".into())
    }
}
