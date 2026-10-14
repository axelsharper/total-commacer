//! 16×16 pixel icons, drawn from scratch in the XP style (two-tone shading, 1-px dark outline).
//! Each icon is 16 rows of 16 characters; the characters index the palette in `color()`.

use egui::{Color32, ColorImage, Context, TextureHandle, TextureOptions};

fn color(c: char) -> Color32 {
    match c {
        '.' => Color32::TRANSPARENT,
        'K' => Color32::from_rgb(0x00, 0x00, 0x00),
        'D' => Color32::from_rgb(0x40, 0x40, 0x40),
        'G' => Color32::from_rgb(0x80, 0x80, 0x80),
        'g' => Color32::from_rgb(0xC0, 0xC0, 0xC0),
        'l' => Color32::from_rgb(0xE4, 0xE4, 0xE4),
        'W' => Color32::from_rgb(0xFF, 0xFF, 0xFF),
        'Y' => Color32::from_rgb(0xFF, 0xE6, 0x8C), // folder front
        'y' => Color32::from_rgb(0xE8, 0xB9, 0x3A), // folder shade
        'd' => Color32::from_rgb(0xB6, 0x8A, 0x1A), // folder outline
        'O' => Color32::from_rgb(0xFF, 0x9A, 0x1A),
        'B' => Color32::from_rgb(0x2B, 0x5E, 0xC5),
        'b' => Color32::from_rgb(0x9C, 0xC4, 0xF3),
        'N' => Color32::from_rgb(0x0A, 0x24, 0x6A),
        'C' => Color32::from_rgb(0x59, 0xC8, 0xE8),
        'R' => Color32::from_rgb(0xD8, 0x30, 0x20),
        'r' => Color32::from_rgb(0x8C, 0x10, 0x10),
        'E' => Color32::from_rgb(0x3C, 0xB0, 0x3C),
        'e' => Color32::from_rgb(0x1B, 0x6E, 0x1B),
        'T' => Color32::from_rgb(0xE8, 0xE0, 0xC8),
        'P' => Color32::from_rgb(0xC8, 0x60, 0xC8),
        'S' => Color32::from_rgb(0xF0, 0xD0, 0xA0),
        _ => Color32::from_rgb(0xFF, 0x00, 0xFF),
    }
}

pub type Pix = [&'static str; 16];

pub const FOLDER: Pix = [
    "................",
    "................",
    "..ddddd.........",
    ".dYYYYYd........",
    ".dYYYYYYddddddd.",
    ".dYyyyyyyyyyyyd.",
    ".dyYYYYYYYYYYYd.",
    ".dyYYYYYYYYYYYd.",
    ".dyYYYYYYYYYYYd.",
    ".dyYYYYYYYYYYYd.",
    ".dyYYYYYYYYYYYd.",
    ".dyYYYYYYYYYYYd.",
    ".dyyyyyyyyyyyyd.",
    ".ddddddddddddddd",
    "................",
    "................",
];

pub const FOLDER_UP: Pix = [
    "................",
    "................",
    "..ddddd.........",
    ".dYYYYYd........",
    ".dYYYYYYddddddd.",
    ".dYyyyyyyyyyyyd.",
    ".dyYYYYYeYYYYYd.",
    ".dyYYYYeeeYYYYd.",
    ".dyYYYeeeeeYYYd.",
    ".dyYYeeeeeeeYYd.",
    ".dyYYYYeeeYYYYd.",
    ".dyYYYYeeeYYYYd.",
    ".dyyyyyeeeyyyyd.",
    ".ddddddddddddddd",
    "................",
    "................",
];

pub const FILE: Pix = [
    "................",
    "...GGGGGGGGG....",
    "...GWWWWWWWGG...",
    "...GWWWWWWWGlG..",
    "...GWWWWWWWGGGG.",
    "...GWWWWWWWWWWG.",
    "...GWWWWWWWWWWG.",
    "...GWWWWWWWWWWG.",
    "...GWWWWWWWWWWG.",
    "...GWWWWWWWWWWG.",
    "...GWWWWWWWWWWG.",
    "...GWWWWWWWWWWG.",
    "...GWWWWWWWWWWG.",
    "...GWWWWWWWWWWG.",
    "...GGGGGGGGGGGG.",
    "................",
];

pub const FILE_TEXT: Pix = [
    "................",
    "...GGGGGGGGG....",
    "...GWWWWWWWGG...",
    "...GWWWWWWWGlG..",
    "...GWWWWWWWGGGG.",
    "...GWWWWWWWWWWG.",
    "...GWGGGGGGGWWG.",
    "...GWWWWWWWWWWG.",
    "...GWGGGGGGGWWG.",
    "...GWWWWWWWWWWG.",
    "...GWGGGGGGGWWG.",
    "...GWWWWWWWWWWG.",
    "...GWGGGGGWWWWG.",
    "...GWWWWWWWWWWG.",
    "...GGGGGGGGGGGG.",
    "................",
];

pub const FILE_EXE: Pix = [
    "................",
    ".KKKKKKKKKKKKKK.",
    ".KBBBBBBBBBBBBK.",
    ".KBBBBBBBBBBWBK.",
    ".KKKKKKKKKKKKKK.",
    ".KWWWWWWWWWWWWK.",
    ".KWWggggggWWWWK.",
    ".KWWWWWWWWWWWWK.",
    ".KWWggggWWWWWWK.",
    ".KWWWWWWWWWWWWK.",
    ".KWWggggggggWWK.",
    ".KWWWWWWWWWWWWK.",
    ".KWWWWWWWWWWWWK.",
    ".KKKKKKKKKKKKKK.",
    "................",
    "................",
];

pub const FILE_ZIP: Pix = [
    "................",
    "...GGGGGGGGG....",
    "...GWWWyyWWGG...",
    "...GWWWyyWWGlG..",
    "...GWWyyyyWGGGG.",
    "...GWWWyyWWWWWG.",
    "...GWWyyyyWWWWG.",
    "...GWWWyyWWWWWG.",
    "...GWWyyyyWWWWG.",
    "...GWWWyyWWWWWG.",
    "...GWWyyyyWWWWG.",
    "...GWWWyyWWWWWG.",
    "...GWWyOOyWWWWG.",
    "...GWWyOOyWWWWG.",
    "...GGGGGGGGGGGG.",
    "................",
];

pub const FILE_IMAGE: Pix = [
    "................",
    ".GGGGGGGGGGGGGG.",
    ".GbbbbbbbbbbbbG.",
    ".GbbbbbbbbYYbbG.",
    ".GbbbbbbbbYYbbG.",
    ".GbbbbbbbbbbbbG.",
    ".GbbbbbbbbbbbbG.",
    ".GbbbEEbbbbbbbG.",
    ".GbbEEEEbbbbbbG.",
    ".GbEEEEEEbbEEbG.",
    ".GEEEEEEEEEEEEG.",
    ".GeeEEEEeeEEEeG.",
    ".GeeeeeeeeeeeeG.",
    ".GGGGGGGGGGGGGG.",
    "................",
    "................",
];

pub const FILE_AUDIO: Pix = [
    "................",
    "...GGGGGGGGG....",
    "...GWWWWWWWGG...",
    "...GWWWWWWWGlG..",
    "...GWWWWWKKKGGG.",
    "...GWWWWKKKKKWG.",
    "...GWWWWKWWWKWG.",
    "...GWWWWKWWWKWG.",
    "...GWWWWKWWWKWG.",
    "...GWWKKKWKKKWG.",
    "...GWKKKKWKKKWG.",
    "...GWWKKWWWKWWG.",
    "...GWWWWWWWWWWG.",
    "...GWWWWWWWWWWG.",
    "...GGGGGGGGGGGG.",
    "................",
];

pub const FILE_VIDEO: Pix = [
    "................",
    ".KKKKKKKKKKKKKK.",
    ".KWKWKWKWKWKWKK.",
    ".KKKKKKKKKKKKKK.",
    ".KbbbbbbbbbbbbK.",
    ".KbbbbbCCbbbbbK.",
    ".KbbbbCCCCbbbbK.",
    ".KbbbbCCCCbbbbK.",
    ".KbbbbbCCbbbbbK.",
    ".KbbbbbbbbbbbbK.",
    ".KKKKKKKKKKKKKK.",
    ".KWKWKWKWKWKWKK.",
    ".KKKKKKKKKKKKKK.",
    "................",
    "................",
    "................",
];

pub const FILE_HTML: Pix = [
    "................",
    "....GGGGGGG.....",
    "...GbbbBBBbbG...",
    "..GbbBBBBBbbbbG.",
    ".GbbBBBBBbbbbbbG",
    ".GbbbBBBbbbbbbbG",
    ".GbbbbBbbbBBbbbG",
    ".GbbbbbbbBBBBbbG",
    ".GbbbbbbbBBBBBbG",
    ".GbbbbbbbbBBBBbG",
    ".GbbbBbbbbbBBbbG",
    ".GbbBBBbbbbbbbbG",
    "..GbbBbbbbbbbbG.",
    "...GbbbbbbbbbG..",
    "....GGGGGGGGG...",
    "................",
];

pub const DRIVE_HDD: Pix = [
    "................",
    "................",
    "................",
    "................",
    ".GGGGGGGGGGGGGG.",
    ".GllllllllllllG.",
    ".GggggggggggggG.",
    ".GggggggggggggG.",
    ".GggggggggggggG.",
    ".GgggggggggggEG.",
    ".GDDDDDDDDDDDDG.",
    ".GGGGGGGGGGGGGG.",
    "................",
    "................",
    "................",
    "................",
];

pub const DRIVE_FLOPPY: Pix = [
    "................",
    ".NNNNNNNNNNNNN..",
    ".NWWWWWWWWWWWNN.",
    ".NWGGGGGGGGGWNN.",
    ".NWGWWWWWWWGWNN.",
    ".NWGGGGGGGGGWNN.",
    ".NWWWWWWWWWWWNN.",
    ".NNNNNNNNNNNNNN.",
    ".NNNNNNNNNNNNNN.",
    ".NNNgggggggNNNN.",
    ".NNNgKKgggggNNN.",
    ".NNNgKKgggggNNN.",
    ".NNNgKKgggggNNN.",
    ".NNNgggggggNNNN.",
    ".NNNNNNNNNNNNNN.",
    "................",
];

pub const DRIVE_CD: Pix = [
    "................",
    ".....ggggg......",
    "...gglllllgg....",
    "..glllCllllllg..",
    ".gllllllllllllg.",
    ".glllllggllllCg.",
    ".gllllgWWglllll.",
    ".gllllgWWgllllg.",
    ".glllllggllllgg.",
    ".gCllllllllllgg.",
    ".gllllllllllllg.",
    "..gllllllllllg..",
    "...ggllllllgg...",
    ".....ggggg......",
    "................",
    "................",
];

pub const DRIVE_NET: Pix = [
    "................",
    "................",
    ".GGGGGGGGGGGGGG.",
    ".GllllllllllllG.",
    ".GggggggggggggG.",
    ".GgggggggggggEG.",
    ".GDDDDDDDDDDDDG.",
    ".GGGGGGGGGGGGGG.",
    ".......E........",
    ".......E........",
    "...EEEEEEEEE....",
    "...E.......E....",
    ".EEEEE...EEEEE..",
    ".EeeeE...EeeeE..",
    ".EEEEE...EEEEE..",
    "................",
];

pub const DRIVE_REMOVABLE: Pix = [
    "................",
    "................",
    "......GGGG......",
    "......GWWG......",
    "......GWWG......",
    "....GGGGGGGG....",
    "....GllllllG....",
    "....GggggggG....",
    "....GggggggG....",
    "....GggggggG....",
    "....GggggggG....",
    "....GggEggEG....",
    "....GggggggG....",
    "....GGGGGGGG....",
    "................",
    "................",
];

pub const DRIVE_HOME: Pix = [
    "................",
    ".......RR.......",
    "......RrrR......",
    ".....RrrrrR.....",
    "....RrrrrrrR....",
    "...RrrrrrrrrR...",
    "..RrrrrrrrrrrR..",
    ".RRRRRRRRRRRRRR.",
    "...GTTTTTTTTG...",
    "...GTTBBTTTTG...",
    "...GTTBBTTTTG...",
    "...GTTTTTyyyG...",
    "...GTTTTTyyyG...",
    "...GTTTTTyyyG...",
    "...GGGGGGGGGG...",
    "................",
];

// ---- toolbar ---------------------------------------------------------------------------
pub const TB_REFRESH: Pix = [
    "................",
    "......EEEE......",
    "....EEEEEEEE....",
    "...EEE....EEE...",
    "..EEE......EE...",
    "..EE.....EEEE...",
    ".........EEEE...",
    ".........EEEE...",
    "...EEEE.........",
    "...EEEE.........",
    "...EEEE.....EE..",
    "...EE......EEE..",
    "...EEE....EEE...",
    "....EEEEEEEE....",
    "......EEEE......",
    "................",
];

pub const TB_BRIEF: Pix = [
    "................",
    ".KKKKKKKKKKKKKK.",
    ".KWWWWWWWWWWWWK.",
    ".KWKKKKWWKKKKWK.",
    ".KWWWWWWWWWWWWK.",
    ".KWKKKKWWKKKKWK.",
    ".KWWWWWWWWWWWWK.",
    ".KWKKKKWWKKKKWK.",
    ".KWWWWWWWWWWWWK.",
    ".KWKKKKWWKKKKWK.",
    ".KWWWWWWWWWWWWK.",
    ".KWKKKKWWKKKKWK.",
    ".KWWWWWWWWWWWWK.",
    ".KKKKKKKKKKKKKK.",
    "................",
    "................",
];

pub const TB_FULL: Pix = [
    "................",
    ".KKKKKKKKKKKKKK.",
    ".KWWWWWWWWWWWWK.",
    ".KWKKKKWKKWKKWK.",
    ".KWWWWWWWWWWWWK.",
    ".KWKKKKWKKWKKWK.",
    ".KWWWWWWWWWWWWK.",
    ".KWKKKKWKKWKKWK.",
    ".KWWWWWWWWWWWWK.",
    ".KWKKKKWKKWKKWK.",
    ".KWWWWWWWWWWWWK.",
    ".KWKKKKWKKWKKWK.",
    ".KWWWWWWWWWWWWK.",
    ".KKKKKKKKKKKKKK.",
    "................",
    "................",
];

pub const TB_TREE: Pix = [
    "................",
    "..ddddd.........",
    "..dYYYd.........",
    "..ddddd.........",
    "....K...........",
    "....KKK.ddddd...",
    "....K...dYYYd...",
    "....K...ddddd...",
    "....K...........",
    "....KKK.ddddd...",
    "....K...dYYYd...",
    "....K...ddddd...",
    "....K...........",
    "....KKK.ddddd...",
    "........dYYYd...",
    "........ddddd...",
];

pub const TB_QUICKVIEW: Pix = [
    "................",
    "..GGGGGGGG......",
    "..GWWWWWWGG.....",
    "..GWGGGGWGWG....",
    "..GWWWWWWGGGG...",
    "..GWGGGGGKKKKG..",
    "..GWWWWKKWWWKK..",
    "..GWGGKWWWWWWK..",
    "..GWWWKWWWWWWK..",
    "..GWGGKWWWWWWK..",
    "..GWWWWKKWWWKK..",
    "..GGGGGGKKKKKKK.",
    ".............KKK",
    "..............KK",
    "................",
    "................",
];

pub const TB_SEARCH: Pix = [
    "................",
    ".....KKKK.......",
    "...KKWWWWKK.....",
    "..KWWWWWWWWK....",
    "..KWWWWWWWWK....",
    ".KWWWWWWWWWWK...",
    ".KWWWWWWWWWWK...",
    ".KWWWWWWWWWWK...",
    "..KWWWWWWWWK....",
    "..KWWWWWWWWK....",
    "...KKWWWWKKK....",
    ".....KKKK.KKK...",
    "...........KKK..",
    "............KKK.",
    ".............KK.",
    "................",
];

pub const TB_PACK: Pix = [
    "................",
    "..dddddddddddd..",
    ".dYYYYYYYYYYYYd.",
    ".dYYYYYyyYYYYYd.",
    ".ddddddyydddddd.",
    ".dyyyyyOOyyyyyd.",
    ".dyyyyyyyyyyyyd.",
    ".dyyyyyOOyyyyyd.",
    ".dyyyyyyyyyyyyd.",
    ".dyyyyyOOyyyyyd.",
    ".dyyyyyyyyyyyyd.",
    ".dyyyyyOOyyyyyd.",
    ".dyyyyyyyyyyyyd.",
    ".dddddddddddddd.",
    "................",
    "................",
];

pub const TB_UNPACK: Pix = [
    "................",
    "...GGGGGGGG.....",
    "...GWWWWWWGG....",
    "...GWWWWWWGWG...",
    "...GWGGGGWGGGG..",
    "...GWWWWWWWWWG..",
    "...GWGGGGGGWWG..",
    ".ddddddddddddd..",
    ".dyyyyyyyyyyyd..",
    ".dyyyyyOOyyyyd..",
    ".dyyyyyyyyyyyd..",
    ".dyyyyyOOyyyyd..",
    ".dyyyyyyyyyyyd..",
    ".ddddddddddddd..",
    "................",
    "................",
];

pub const TB_COMPARE: Pix = [
    "................",
    ".GGGGGG..GGGGGG.",
    ".GWWWWG..GWWWWG.",
    ".GWGGWG..GWGGWG.",
    ".GWWWWG..GWWWWG.",
    ".GWGGWG..GWGGWG.",
    ".GWWWWG..GWWWWG.",
    ".GWGGWGEEGWGGWG.",
    ".GWWWWG..GWWWWG.",
    ".GWGGWGEEGWGGWG.",
    ".GWWWWG..GWWWWG.",
    ".GWGGWG..GWGGWG.",
    ".GWWWWG..GWWWWG.",
    ".GGGGGG..GGGGGG.",
    "................",
    "................",
];

pub const TB_NOTEPAD: Pix = [
    "................",
    "...GGGGGGGGG....",
    "...GWWWWWWWGG...",
    "...GWWWWWWWGlG..",
    "...GWWWWWWWGGGG.",
    "...GWGGGGGGWWWG.",
    "...GWWWWWWWWWWG.",
    "...GWGGGGGGWWWG.",
    "...GWWWWWWWWOOG.",
    "...GWGGGGGWOOyG.",
    "...GWWWWWWOOyWG.",
    "...GWGGGGOOyWWG.",
    "...GWWWWKKyWWWG.",
    "...GWWWWKWWWWWG.",
    "...GGGGGGGGGGGG.",
    "................",
];

pub const TB_CMD: Pix = [
    "................",
    ".GGGGGGGGGGGGGG.",
    ".GWWWWWWWWWWWWG.",
    ".GGGGGGGGGGGGGG.",
    ".GKKKKKKKKKKKKG.",
    ".GKWKKKKKKKKKKG.",
    ".GKKWKKKKKKKKKG.",
    ".GKKKWKKKKKKKKG.",
    ".GKKWKKKKKKKKKG.",
    ".GKWKKKKKKKKKKG.",
    ".GKKKKKKKKKKKKG.",
    ".GKKKKKKKKKKKKG.",
    ".GGGGGGGGGGGGGG.",
    "................",
    "................",
    "................",
];

pub const TB_SYNC: Pix = [
    "................",
    "..ddddd...ddddd.",
    "..dYYYd...dYYYd.",
    "..dYYYd...dYYYd.",
    "..ddddd...ddddd.",
    "......BBBB......",
    ".....BBBBBB.....",
    "....BB....BB....",
    "..BBBB....BBBB..",
    "...BBB....BBB...",
    "....BB....BB....",
    "......BBBB......",
    "......BBBB......",
    "................",
    "................",
    "................",
];

pub const APP: Pix = [
    "................",
    ".KKKKKKKKKKKKKK.",
    ".KBBBBBBBBBBBBK.",
    ".KBBBBBBBBBBBBK.",
    ".KKKKKKKKKKKKKK.",
    ".KWWWWWKKWWWWWK.",
    ".KWYYWWKKWWYYWK.",
    ".KWWWWWKKWWWWWK.",
    ".KWYYWWKKWWYYWK.",
    ".KWWWWWKKWWWWWK.",
    ".KWYYWWKKWWYYWK.",
    ".KWWWWWKKWWWWWK.",
    ".KWWWWWKKWWWWWK.",
    ".KKKKKKKKKKKKKK.",
    "................",
    "................",
];

pub fn image(pix: &Pix) -> ColorImage {
    let mut px = Vec::with_capacity(256);
    for row in pix.iter() {
        let mut n = 0;
        for c in row.chars().take(16) {
            px.push(color(c));
            n += 1;
        }
        while n < 16 {
            px.push(Color32::TRANSPARENT);
            n += 1;
        }
    }
    ColorImage::new([16, 16], px)
}

pub struct Icons {
    pub folder: TextureHandle,
    pub folder_up: TextureHandle,
    pub file: TextureHandle,
    pub file_text: TextureHandle,
    pub file_exe: TextureHandle,
    pub file_zip: TextureHandle,
    pub file_image: TextureHandle,
    pub file_audio: TextureHandle,
    pub file_video: TextureHandle,
    pub file_html: TextureHandle,
    pub drive_hdd: TextureHandle,
    pub drive_floppy: TextureHandle,
    pub drive_cd: TextureHandle,
    pub drive_net: TextureHandle,
    pub drive_removable: TextureHandle,
    pub drive_home: TextureHandle,
    pub tb_refresh: TextureHandle,
    pub tb_brief: TextureHandle,
    pub tb_full: TextureHandle,
    pub tb_tree: TextureHandle,
    pub tb_quickview: TextureHandle,
    pub tb_search: TextureHandle,
    pub tb_pack: TextureHandle,
    pub tb_unpack: TextureHandle,
    pub tb_compare: TextureHandle,
    pub tb_notepad: TextureHandle,
    pub tb_cmd: TextureHandle,
    pub tb_sync: TextureHandle,
    pub app: TextureHandle,
}

impl Icons {
    pub fn load(ctx: &Context) -> Self {
        let l = |name: &str, p: &Pix| ctx.load_texture(name, image(p), TextureOptions::NEAREST);
        Self {
            folder: l("folder", &FOLDER),
            folder_up: l("folder_up", &FOLDER_UP),
            file: l("file", &FILE),
            file_text: l("file_text", &FILE_TEXT),
            file_exe: l("file_exe", &FILE_EXE),
            file_zip: l("file_zip", &FILE_ZIP),
            file_image: l("file_image", &FILE_IMAGE),
            file_audio: l("file_audio", &FILE_AUDIO),
            file_video: l("file_video", &FILE_VIDEO),
            file_html: l("file_html", &FILE_HTML),
            drive_hdd: l("drive_hdd", &DRIVE_HDD),
            drive_floppy: l("drive_floppy", &DRIVE_FLOPPY),
            drive_cd: l("drive_cd", &DRIVE_CD),
            drive_net: l("drive_net", &DRIVE_NET),
            drive_removable: l("drive_removable", &DRIVE_REMOVABLE),
            drive_home: l("drive_home", &DRIVE_HOME),
            tb_refresh: l("tb_refresh", &TB_REFRESH),
            tb_brief: l("tb_brief", &TB_BRIEF),
            tb_full: l("tb_full", &TB_FULL),
            tb_tree: l("tb_tree", &TB_TREE),
            tb_quickview: l("tb_quickview", &TB_QUICKVIEW),
            tb_search: l("tb_search", &TB_SEARCH),
            tb_pack: l("tb_pack", &TB_PACK),
            tb_unpack: l("tb_unpack", &TB_UNPACK),
            tb_compare: l("tb_compare", &TB_COMPARE),
            tb_notepad: l("tb_notepad", &TB_NOTEPAD),
            tb_cmd: l("tb_cmd", &TB_CMD),
            tb_sync: l("tb_sync", &TB_SYNC),
            app: l("app", &APP),
        }
    }

    /// Icon for a file by extension (lower-case, without the dot).
    pub fn for_ext(&self, ext: &str) -> &TextureHandle {
        match ext {
            "txt" | "ini" | "log" | "md" | "cfg" | "conf" | "nfo" | "diz" | "csv" | "json" | "xml" | "toml" | "yaml" | "yml" | "rs" | "py" | "c" | "h" | "js" | "ts" | "css" | "sh" | "bat" | "cmd" => &self.file_text,
            "exe" | "com" | "app" | "msi" | "dmg" | "appimage" | "deb" | "rpm" | "bin" | "run" => &self.file_exe,
            "zip" | "rar" | "7z" | "gz" | "tgz" | "bz2" | "xz" | "tar" | "arj" | "lha" | "lzh" | "ace" | "cab" | "jar" => &self.file_zip,
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "tif" | "tiff" | "svg" | "webp" | "pcx" | "tga" | "psd" => &self.file_image,
            "mp3" | "wav" | "ogg" | "flac" | "m4a" | "aac" | "wma" | "mid" | "xm" | "mod" | "s3m" | "it" => &self.file_audio,
            "mp4" | "avi" | "mkv" | "mov" | "mpg" | "mpeg" | "wmv" | "webm" | "flv" | "mxf" => &self.file_video,
            "htm" | "html" | "url" | "mht" | "xhtml" => &self.file_html,
            _ => &self.file,
        }
    }
}

/// Draw an icon at integer pixel position.
pub fn draw(p: &egui::Painter, tex: &TextureHandle, pos: egui::Pos2) {
    let r = egui::Rect::from_min_size(pos.round(), egui::Vec2::splat(16.0));
    p.image(tex.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
}
