//! Directory listing, drives, sorting, number/date formatting — the model behind a panel.

use chrono::{DateTime, Local};
use std::cmp::Ordering;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_up: bool,
    pub size: u64,
    pub modified: Option<DateTime<Local>>,
    pub hidden: bool,
    pub readonly: bool,
    pub system: bool,
    pub archive: bool,
    pub symlink: bool,
    /// Size counted with Space / Alt+Shift+Enter (directories only).
    pub dir_size: Option<u64>,
}

impl Entry {
    pub fn up(parent: PathBuf) -> Self {
        Self {
            name: "..".into(),
            path: parent,
            is_dir: true,
            is_up: true,
            size: 0,
            modified: None,
            hidden: false,
            readonly: false,
            system: false,
            archive: false,
            symlink: false,
            dir_size: None,
        }
    }

    /// Name without extension (TC splits at the last dot; directories and dot-files are whole).
    pub fn stem(&self) -> &str {
        if self.is_dir {
            return &self.name;
        }
        match self.name.rfind('.') {
            Some(i) if i > 0 => &self.name[..i],
            _ => &self.name,
        }
    }

    pub fn ext(&self) -> &str {
        if self.is_dir {
            return "";
        }
        match self.name.rfind('.') {
            Some(i) if i > 0 => &self.name[i + 1..],
            _ => "",
        }
    }

    pub fn ext_lower(&self) -> String {
        self.ext().to_ascii_lowercase()
    }

    pub fn shown_size(&self) -> u64 {
        if self.is_dir {
            self.dir_size.unwrap_or(0)
        } else {
            self.size
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortBy {
    Name,
    Ext,
    Size,
    Date,
    Unsorted,
}

pub fn list_dir(path: &Path, show_hidden: bool) -> io::Result<Vec<Entry>> {
    let mut out = Vec::new();
    if let Some(parent) = path.parent() {
        out.push(Entry::up(parent.to_path_buf()));
    }
    for de in fs::read_dir(path)? {
        let de = match de {
            Ok(d) => d,
            Err(_) => continue,
        };
        let name = de.file_name().to_string_lossy().to_string();
        let p = de.path();
        let lmeta = match fs::symlink_metadata(&p) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let symlink = lmeta.file_type().is_symlink();
        let meta = if symlink { fs::metadata(&p).unwrap_or(lmeta.clone()) } else { lmeta.clone() };
        let (hidden, system, archive, readonly) = attrs(&name, &meta);
        if hidden && !show_hidden {
            continue;
        }
        let modified = meta.modified().ok().map(DateTime::<Local>::from);
        out.push(Entry {
            name,
            path: p,
            is_dir: meta.is_dir(),
            is_up: false,
            size: if meta.is_dir() { 0 } else { meta.len() },
            modified,
            hidden,
            readonly,
            system,
            archive,
            symlink,
            dir_size: None,
        });
    }
    Ok(out)
}

#[cfg(windows)]
fn attrs(_name: &str, meta: &fs::Metadata) -> (bool, bool, bool, bool) {
    use std::os::windows::fs::MetadataExt;
    let a = meta.file_attributes();
    (a & 0x2 != 0, a & 0x4 != 0, a & 0x20 != 0, a & 0x1 != 0)
}

#[cfg(not(windows))]
fn attrs(name: &str, meta: &fs::Metadata) -> (bool, bool, bool, bool) {
    let hidden = name.starts_with('.');
    let readonly = meta.permissions().readonly();
    (hidden, false, !meta.is_dir(), readonly)
}

pub fn attr_string(e: &Entry) -> String {
    if e.is_up {
        return String::new();
    }
    format!(
        "{}{}{}{}",
        if e.readonly { 'r' } else { '-' },
        if e.archive { 'a' } else { '-' },
        if e.hidden { 'h' } else { '-' },
        if e.system { 's' } else { '-' }
    )
}

fn ci(a: &str, b: &str) -> Ordering {
    let r = a.to_lowercase().cmp(&b.to_lowercase());
    if r == Ordering::Equal {
        a.cmp(b)
    } else {
        r
    }
}

/// Sort like Total Commander: ".." first, directories before files (by name), then the key.
/// Size and date sort descending by default (largest / newest first), as in TC.
pub fn sort_entries(v: &mut [Entry], by: SortBy, reverse: bool) {
    v.sort_by(|a, b| {
        if a.is_up != b.is_up {
            return if a.is_up { Ordering::Less } else { Ordering::Greater };
        }
        if a.is_dir != b.is_dir {
            return if a.is_dir { Ordering::Less } else { Ordering::Greater };
        }
        let o = if a.is_dir {
            match by {
                SortBy::Date => b.modified.cmp(&a.modified).then_with(|| ci(&a.name, &b.name)),
                SortBy::Unsorted => Ordering::Equal,
                _ => ci(&a.name, &b.name),
            }
        } else {
            match by {
                SortBy::Name => ci(&a.name, &b.name),
                SortBy::Ext => ci(a.ext(), b.ext()).then_with(|| ci(a.stem(), b.stem())),
                SortBy::Size => b.size.cmp(&a.size).then_with(|| ci(&a.name, &b.name)),
                SortBy::Date => b.modified.cmp(&a.modified).then_with(|| ci(&a.name, &b.name)),
                SortBy::Unsorted => Ordering::Equal,
            }
        };
        if reverse {
            o.reverse()
        } else {
            o
        }
    });
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DriveKind {
    Floppy,
    Fixed,
    Removable,
    Cdrom,
    Network,
    Home,
}

#[derive(Clone, Debug)]
pub struct Drive {
    /// Short label on the drive button ("c", "/", "~", "USB").
    pub letter: String,
    pub root: PathBuf,
    pub kind: DriveKind,
    pub label: String,
}

#[cfg(windows)]
pub fn drives() -> Vec<Drive> {
    use windows_sys::Win32::Storage::FileSystem::GetDriveTypeW;
    let mut v = Vec::new();
    for c in b'A'..=b'Z' {
        let root = format!("{}:\\", c as char);
        let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
        let t = unsafe { GetDriveTypeW(wide.as_ptr()) };
        let kind = match t {
            2 => {
                if c == b'A' || c == b'B' {
                    DriveKind::Floppy
                } else {
                    DriveKind::Removable
                }
            }
            3 => DriveKind::Fixed,
            4 => DriveKind::Network,
            5 => DriveKind::Cdrom,
            6 => DriveKind::Fixed,
            _ => continue,
        };
        v.push(Drive { letter: (c as char).to_ascii_lowercase().to_string(), root: PathBuf::from(&root), kind, label: root });
    }
    v
}

#[cfg(target_os = "macos")]
pub fn drives() -> Vec<Drive> {
    let mut v = vec![Drive { letter: "/".into(), root: PathBuf::from("/"), kind: DriveKind::Fixed, label: "Macintosh HD".into() }];
    if let Some(h) = dirs::home_dir() {
        v.push(Drive { letter: "~".into(), root: h, kind: DriveKind::Home, label: "Home".into() });
    }
    if let Ok(rd) = fs::read_dir("/Volumes") {
        let mut vols: Vec<_> = rd.flatten().collect();
        vols.sort_by_key(|d| d.file_name());
        for d in vols {
            let p = d.path();
            if fs::symlink_metadata(&p).map(|m| m.file_type().is_symlink()).unwrap_or(false) {
                continue; // "Macintosh HD" -> /
            }
            let name = d.file_name().to_string_lossy().to_string();
            v.push(Drive { letter: short(&name), root: p, kind: DriveKind::Removable, label: name });
        }
    }
    v
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn drives() -> Vec<Drive> {
    let mut v = vec![Drive { letter: "/".into(), root: PathBuf::from("/"), kind: DriveKind::Fixed, label: "/".into() }];
    if let Some(h) = dirs::home_dir() {
        v.push(Drive { letter: "~".into(), root: h, kind: DriveKind::Home, label: "Home".into() });
    }
    if let Ok(s) = fs::read_to_string("/proc/mounts") {
        for line in s.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }
            let mp = parts[1].replace("\\040", " ");
            if mp.starts_with("/media/") || mp.starts_with("/mnt/") || mp.starts_with("/run/media/") {
                let name = Path::new(&mp).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or(mp.clone());
                let kind = if parts[2].contains("cifs") || parts[2].contains("nfs") || parts[2].contains("smb") {
                    DriveKind::Network
                } else if parts[2].contains("iso9660") || parts[2].contains("udf") {
                    DriveKind::Cdrom
                } else {
                    DriveKind::Removable
                };
                v.push(Drive { letter: short(&name), root: PathBuf::from(&mp), kind, label: name });
            }
        }
    }
    v
}

#[allow(dead_code)]
fn short(name: &str) -> String {
    let s: String = name.chars().take(6).collect();
    s
}

/// (free, total) bytes for the volume holding `path`.
pub fn space(path: &Path) -> Option<(u64, u64)> {
    let mut p = path.to_path_buf();
    loop {
        if let (Ok(f), Ok(t)) = (fs4::available_space(&p), fs4::total_space(&p)) {
            return Some((f, t));
        }
        if !p.pop() {
            return None;
        }
    }
}

/// "12 345" — thousands separated by a space (the Swedish XP locale; TC follows the system).
pub fn fmt_num(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    let len = s.len();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// "1 234 k" — TC's kilobyte figure (rounded up).
pub fn fmt_k(bytes: u64) -> String {
    format!("{} k", fmt_num(bytes.div_ceil(1024)))
}

pub fn fmt_date(d: &Option<DateTime<Local>>) -> String {
    match d {
        Some(d) => d.format("%d.%m.%Y %H:%M").to_string(),
        None => String::new(),
    }
}

/// Total size of a directory tree (Space / Alt+Shift+Enter).
pub fn dir_size(path: &Path) -> u64 {
    walkdir::WalkDir::new(path)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

/// Wildcard match with `*` and `?`, several masks separated by `;` (case-insensitive).
pub fn matches_mask(name: &str, mask: &str) -> bool {
    let name = name.to_lowercase();
    mask.split(';').map(|m| m.trim()).filter(|m| !m.is_empty()).any(|m| {
        let m = m.to_lowercase();
        // "*.*" means everything, also names without a dot (DOS semantics)
        if m == "*.*" || m == "*" {
            return true;
        }
        glob(m.as_bytes(), name.as_bytes())
    })
}

fn glob(m: &[u8], s: &[u8]) -> bool {
    match (m.first(), s.first()) {
        (None, None) => true,
        (Some(b'*'), _) => glob(&m[1..], s) || (!s.is_empty() && glob(m, &s[1..])),
        (Some(b'?'), Some(_)) => glob(&m[1..], &s[1..]),
        (Some(a), Some(b)) if a == b => glob(&m[1..], &s[1..]),
        // "*.txt" should also match "notes.txt"; a trailing "." in the mask matches a missing extension
        (Some(b'.'), None) => m.iter().all(|&c| c == b'.' || c == b'*'),
        _ => false,
    }
}

/// Path string the way TC shows it: "c:\dir\sub" / "/home/axel".
pub fn display(p: &Path) -> String {
    let s = p.to_string_lossy().to_string();
    #[cfg(windows)]
    {
        let s = s.trim_start_matches(r"\\?\").to_string();
        let mut c = s.chars();
        return match c.next() {
            Some(d) if d.is_ascii_alphabetic() => d.to_ascii_lowercase().to_string() + c.as_str(),
            _ => s,
        };
    }
    #[allow(unreachable_code)]
    s
}

pub fn sep() -> char {
    if cfg!(windows) {
        '\\'
    } else {
        '/'
    }
}

/// Make a unique "Copy of x" style name — used by Shift+F5 default.
pub fn copy_name(name: &str) -> String {
    name.to_string()
}

/// Human-readable error.
pub fn err_text(e: &io::Error) -> String {
    let s = e.to_string();
    match s.find(" (os error") {
        Some(i) => s[..i].to_string(),
        None => s,
    }
}
