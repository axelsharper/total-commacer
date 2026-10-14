//! `wincmd.ini`-style settings file: `[Configuration]`, `[left]`, `[right]`, `[DirMenu]`.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use crate::fs::SortBy;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QuickSearch {
    CtrlAlt,
    Alt,
    Letters,
}

#[derive(Clone, Debug)]
pub struct PanelCfg {
    pub path: PathBuf,
    pub sort: SortBy,
    pub reverse: bool,
    pub brief: bool,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub left: PanelCfg,
    pub right: PanelCfg,
    pub show_hidden: bool,
    pub quick_search: QuickSearch,
    pub nag: bool,
    pub editor: String,
    pub lynx: bool,
    pub right_click_selects: bool,
    pub win_w: f32,
    pub win_h: f32,
    pub maximized: bool,
    pub hotlist: Vec<(String, PathBuf)>,
    pub cmd_history: Vec<String>,
}

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}

impl Default for Config {
    fn default() -> Self {
        let h = home();
        let p = PanelCfg { path: h.clone(), sort: SortBy::Name, reverse: false, brief: false };
        Self {
            left: p.clone(),
            right: PanelCfg { path: if cfg!(windows) { PathBuf::from("C:\\") } else { PathBuf::from("/") }, ..p },
            show_hidden: false,
            quick_search: QuickSearch::CtrlAlt,
            nag: true,
            editor: String::new(),
            lynx: false,
            right_click_selects: true,
            win_w: 900.0,
            win_h: 640.0,
            maximized: false,
            hotlist: vec![("&Home".into(), h)],
            cmd_history: Vec::new(),
        }
    }
}

pub fn file_path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(home).join("total-commacer").join("wincmd.ini")
}

fn sort_name(s: SortBy) -> &'static str {
    match s {
        SortBy::Name => "name",
        SortBy::Ext => "ext",
        SortBy::Size => "size",
        SortBy::Date => "date",
        SortBy::Unsorted => "unsorted",
    }
}
fn sort_of(s: &str) -> SortBy {
    match s {
        "ext" => SortBy::Ext,
        "size" => SortBy::Size,
        "date" => SortBy::Date,
        "unsorted" => SortBy::Unsorted,
        _ => SortBy::Name,
    }
}

type Ini = BTreeMap<String, Vec<(String, String)>>;

fn parse(s: &str) -> Ini {
    let mut ini: Ini = BTreeMap::new();
    let mut sec = String::new();
    for line in s.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with(';') || l.starts_with('#') {
            continue;
        }
        if l.starts_with('[') && l.ends_with(']') {
            sec = l[1..l.len() - 1].to_string();
            continue;
        }
        if let Some((k, v)) = l.split_once('=') {
            ini.entry(sec.clone()).or_default().push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    ini
}

fn get<'a>(ini: &'a Ini, sec: &str, key: &str) -> Option<&'a str> {
    ini.get(sec)?.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

fn panel(ini: &Ini, sec: &str, def: &PanelCfg) -> PanelCfg {
    let path = get(ini, sec, "path").map(PathBuf::from).filter(|p| p.is_dir()).unwrap_or_else(|| def.path.clone());
    PanelCfg {
        path,
        sort: get(ini, sec, "sort").map(sort_of).unwrap_or(def.sort),
        reverse: get(ini, sec, "reverse") == Some("1"),
        brief: get(ini, sec, "brief") == Some("1"),
    }
}

impl Config {
    pub fn load() -> Self {
        let def = Self::default();
        let Ok(s) = fs::read_to_string(file_path()) else { return def };
        let ini = parse(&s);
        let c = "Configuration";
        let flag = |k: &str, d: bool| get(&ini, c, k).map(|v| v == "1").unwrap_or(d);
        let num = |k: &str, d: f32| get(&ini, c, k).and_then(|v| v.parse().ok()).unwrap_or(d);
        Self {
            left: panel(&ini, "left", &def.left),
            right: panel(&ini, "right", &def.right),
            show_hidden: flag("ShowHiddenSystem", def.show_hidden),
            quick_search: match get(&ini, c, "QuickSearch") {
                Some("alt") => QuickSearch::Alt,
                Some("letters") => QuickSearch::Letters,
                _ => QuickSearch::CtrlAlt,
            },
            nag: flag("NagScreen", def.nag),
            editor: get(&ini, c, "Editor").unwrap_or("").to_string(),
            lynx: flag("LynxLike", def.lynx),
            right_click_selects: flag("RightMouseSelects", def.right_click_selects),
            win_w: num("Width", def.win_w),
            win_h: num("Height", def.win_h),
            maximized: flag("Maximized", false),
            hotlist: ini
                .get("DirMenu")
                .map(|v| v.iter().filter(|(k, _)| k.starts_with("menu")).map(|(k, name)| {
                    let n = &k[4..];
                    let path = get(&ini, "DirMenu", &format!("cmd{n}")).unwrap_or("").trim_start_matches("cd ").to_string();
                    (name.clone(), PathBuf::from(path))
                }).collect())
                .filter(|v: &Vec<_>| !v.is_empty())
                .unwrap_or(def.hotlist),
            cmd_history: ini.get("Command line history").map(|v| v.iter().map(|(_, v)| v.clone()).collect()).unwrap_or_default(),
        }
    }

    pub fn save(&self) {
        let mut s = String::new();
        s.push_str("[Configuration]\n");
        s.push_str(&format!("ShowHiddenSystem={}\n", self.show_hidden as u8));
        s.push_str(&format!(
            "QuickSearch={}\n",
            match self.quick_search {
                QuickSearch::CtrlAlt => "ctrlalt",
                QuickSearch::Alt => "alt",
                QuickSearch::Letters => "letters",
            }
        ));
        s.push_str(&format!("NagScreen={}\n", self.nag as u8));
        s.push_str(&format!("Editor={}\n", self.editor));
        s.push_str(&format!("LynxLike={}\n", self.lynx as u8));
        s.push_str(&format!("RightMouseSelects={}\n", self.right_click_selects as u8));
        s.push_str(&format!("Width={}\nHeight={}\nMaximized={}\n", self.win_w as i32, self.win_h as i32, self.maximized as u8));
        for (sec, p) in [("left", &self.left), ("right", &self.right)] {
            s.push_str(&format!("\n[{sec}]\npath={}\nsort={}\nreverse={}\nbrief={}\n", p.path.to_string_lossy(), sort_name(p.sort), p.reverse as u8, p.brief as u8));
        }
        s.push_str("\n[DirMenu]\n");
        for (i, (name, path)) in self.hotlist.iter().enumerate() {
            s.push_str(&format!("menu{}={}\ncmd{}=cd {}\n", i + 1, name, i + 1, path.to_string_lossy()));
        }
        s.push_str("\n[Command line history]\n");
        for (i, h) in self.cmd_history.iter().take(50).enumerate() {
            s.push_str(&format!("{i}={h}\n"));
        }
        let p = file_path();
        if let Some(d) = p.parent() {
            let _ = fs::create_dir_all(d);
        }
        let _ = fs::write(p, s);
    }
}
