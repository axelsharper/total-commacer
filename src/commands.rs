//! Commands, Total Commander's default keyboard layout, and the menu tree.

use egui::Key;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Cmd {
    // files
    Help,
    Reread,
    View,
    ViewCursor,
    Edit,
    EditNew,
    Copy,
    CopySameDir,
    Move,
    Rename,
    MkDir,
    Delete,
    Exit,
    Pack,
    Unpack,
    Find,
    Properties,
    CalcSpace,
    CalcAllDirs,
    Attributes,
    CompareContent,
    CompareDirs,
    MultiRename,
    // navigation
    Open,
    OpenDir,
    Parent,
    Root,
    GoBack,
    GoForward,
    DirHistory,
    OpenInOther,
    DriveLeft,
    DriveRight,
    Tree,
    Hotlist,
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    SwitchPanel,
    SwapPanels,
    TargetEqSource,
    CursorUp,
    CursorDown,
    PageUp,
    PageDown,
    Home,
    End,
    SelUp,
    SelDown,
    SelPageUp,
    SelPageDown,
    SelHome,
    SelEnd,
    Left,
    Right,
    Escape,
    // selection
    ToggleSelDown,
    ToggleSelCount,
    SelectAll,
    UnselectAll,
    SelectGroup,
    UnselectGroup,
    InvertSelection,
    RestoreSelection,
    SelectSameExt,
    UnselectSameExt,
    CopyNamesToClip,
    CopyFullNamesToClip,
    // view
    Brief,
    Full,
    TreeView,
    QuickView,
    SortName,
    SortExt,
    SortDate,
    SortSize,
    SortUnsorted,
    ReverseSort,
    AllFiles,
    ProgramsOnly,
    CustomFilter,
    BranchView,
    ToggleHidden,
    // command line
    CopyNameToCmd,
    CopyPathNameToCmd,
    CopyPathToCmd,
    CmdHistory,
    CmdPrev,
    CmdNext,
    OpenTerminal,
    // misc
    MenuBar,
    Options,
    About,
    Keyboard,
    Homepage,
    SavePosition,
    Nothing,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

pub const NONE: Mods = Mods { ctrl: false, alt: false, shift: false };
pub const CTRL: Mods = Mods { ctrl: true, alt: false, shift: false };
pub const ALT: Mods = Mods { ctrl: false, alt: true, shift: false };
pub const SHIFT: Mods = Mods { ctrl: false, alt: false, shift: true };
pub const CTRL_SHIFT: Mods = Mods { ctrl: true, alt: false, shift: true };
pub const ALT_SHIFT: Mods = Mods { ctrl: false, alt: true, shift: true };
pub const CTRL_ALT: Mods = Mods { ctrl: true, alt: true, shift: false };

pub struct Bind {
    pub mods: Mods,
    pub key: Key,
    pub cmd: Cmd,
}

const fn b(mods: Mods, key: Key, cmd: Cmd) -> Bind {
    Bind { mods, key, cmd }
}

/// Total Commander's default layout (as documented in its help, "Keyboard layout").
/// Num* and Num/ have no `egui::Key`; they arrive as text and are handled in `app.rs`.
pub const KEYMAP: &[Bind] = &[
    b(NONE, Key::F1, Cmd::Help),
    b(NONE, Key::F2, Cmd::Reread),
    b(NONE, Key::F3, Cmd::View),
    b(NONE, Key::F4, Cmd::Edit),
    b(NONE, Key::F5, Cmd::Copy),
    b(NONE, Key::F6, Cmd::Move),
    b(NONE, Key::F7, Cmd::MkDir),
    b(NONE, Key::F8, Cmd::Delete),
    b(NONE, Key::F9, Cmd::MenuBar),
    b(NONE, Key::F10, Cmd::MenuBar),
    b(NONE, Key::Delete, Cmd::Delete),
    b(SHIFT, Key::F2, Cmd::CompareDirs),
    b(SHIFT, Key::F3, Cmd::ViewCursor),
    b(SHIFT, Key::F4, Cmd::EditNew),
    b(SHIFT, Key::F5, Cmd::CopySameDir),
    b(SHIFT, Key::F6, Cmd::Rename),
    b(SHIFT, Key::F8, Cmd::Delete),
    b(SHIFT, Key::Delete, Cmd::Delete),
    b(ALT, Key::F1, Cmd::DriveLeft),
    b(ALT, Key::F2, Cmd::DriveRight),
    b(ALT, Key::F3, Cmd::View),
    b(ALT, Key::F4, Cmd::Exit),
    b(ALT, Key::F5, Cmd::Pack),
    b(ALT, Key::F7, Cmd::Find),
    b(ALT, Key::F8, Cmd::CmdHistory),
    b(ALT, Key::F9, Cmd::Unpack),
    b(ALT, Key::F10, Cmd::Tree),
    b(ALT, Key::Enter, Cmd::Properties),
    b(ALT_SHIFT, Key::Enter, Cmd::CalcAllDirs),
    b(ALT, Key::ArrowLeft, Cmd::GoBack),
    b(ALT, Key::ArrowRight, Cmd::GoForward),
    b(ALT, Key::ArrowDown, Cmd::DirHistory),
    b(CTRL, Key::F1, Cmd::Brief),
    b(CTRL, Key::F2, Cmd::Full),
    b(CTRL, Key::F3, Cmd::SortName),
    b(CTRL, Key::F4, Cmd::SortExt),
    b(CTRL, Key::F5, Cmd::SortDate),
    b(CTRL, Key::F6, Cmd::SortSize),
    b(CTRL, Key::F7, Cmd::SortUnsorted),
    b(CTRL, Key::F8, Cmd::TreeView),
    b(CTRL, Key::F10, Cmd::AllFiles),
    b(CTRL, Key::F11, Cmd::ProgramsOnly),
    b(CTRL, Key::F12, Cmd::CustomFilter),
    b(CTRL, Key::A, Cmd::SelectAll),
    b(CTRL, Key::B, Cmd::BranchView),
    b(CTRL, Key::D, Cmd::Hotlist),
    b(CTRL, Key::E, Cmd::CmdPrev),
    b(CTRL, Key::H, Cmd::ToggleHidden),
    b(CTRL, Key::L, Cmd::CalcSpace),
    b(CTRL, Key::M, Cmd::MultiRename),
    b(CTRL, Key::P, Cmd::CopyPathToCmd),
    b(CTRL, Key::Q, Cmd::QuickView),
    b(CTRL, Key::R, Cmd::Reread),
    b(CTRL, Key::T, Cmd::NewTab),
    b(CTRL, Key::U, Cmd::SwapPanels),
    b(CTRL, Key::W, Cmd::CloseTab),
    b(CTRL, Key::Tab, Cmd::NextTab),
    b(CTRL_SHIFT, Key::Tab, Cmd::PrevTab),
    b(CTRL_SHIFT, Key::C, Cmd::CopyFullNamesToClip),
    b(CTRL, Key::Enter, Cmd::CopyNameToCmd),
    b(CTRL_SHIFT, Key::Enter, Cmd::CopyPathNameToCmd),
    b(CTRL, Key::ArrowLeft, Cmd::OpenInOther),
    b(CTRL, Key::ArrowRight, Cmd::OpenInOther),
    b(CTRL, Key::ArrowDown, Cmd::CmdHistory),
    b(CTRL, Key::PageUp, Cmd::Parent),
    b(CTRL, Key::PageDown, Cmd::OpenDir),
    b(CTRL, Key::Backslash, Cmd::Root),
    b(CTRL, Key::Plus, Cmd::SelectAll),
    b(CTRL, Key::Minus, Cmd::UnselectAll),
    b(ALT, Key::Plus, Cmd::SelectSameExt),
    b(ALT, Key::Minus, Cmd::UnselectSameExt),
    b(NONE, Key::Plus, Cmd::SelectGroup),
    b(NONE, Key::Minus, Cmd::UnselectGroup),
    b(NONE, Key::Insert, Cmd::ToggleSelDown),
    b(NONE, Key::Space, Cmd::ToggleSelCount),
    b(NONE, Key::Tab, Cmd::SwitchPanel),
    b(NONE, Key::Enter, Cmd::Open),
    b(NONE, Key::Backspace, Cmd::Parent),
    b(NONE, Key::Escape, Cmd::Escape),
    b(NONE, Key::ArrowUp, Cmd::CursorUp),
    b(NONE, Key::ArrowDown, Cmd::CursorDown),
    b(NONE, Key::ArrowLeft, Cmd::Left),
    b(NONE, Key::ArrowRight, Cmd::Right),
    b(NONE, Key::PageUp, Cmd::PageUp),
    b(NONE, Key::PageDown, Cmd::PageDown),
    b(NONE, Key::Home, Cmd::Home),
    b(NONE, Key::End, Cmd::End),
    b(SHIFT, Key::ArrowUp, Cmd::SelUp),
    b(SHIFT, Key::ArrowDown, Cmd::SelDown),
    b(SHIFT, Key::PageUp, Cmd::SelPageUp),
    b(SHIFT, Key::PageDown, Cmd::SelPageDown),
    b(SHIFT, Key::Home, Cmd::SelHome),
    b(SHIFT, Key::End, Cmd::SelEnd),
];

pub fn lookup(mods: Mods, key: Key) -> Option<Cmd> {
    KEYMAP.iter().find(|b| b.mods == mods && b.key == key).map(|b| b.cmd)
}

pub struct MenuItem {
    pub label: &'static str,
    pub cmd: Option<Cmd>,
    pub shortcut: &'static str,
}

pub struct Menu {
    pub title: &'static str,
    pub items: &'static [MenuItem],
}

const fn mi(label: &'static str, cmd: Cmd, shortcut: &'static str) -> MenuItem {
    MenuItem { label, cmd: Some(cmd), shortcut }
}
const fn off(label: &'static str, shortcut: &'static str) -> MenuItem {
    MenuItem { label, cmd: None, shortcut }
}
const SEP: MenuItem = MenuItem { label: "-", cmd: None, shortcut: "" };

pub const MENUS: &[Menu] = &[
    Menu {
        title: "&Files",
        items: &[
            mi("Change &Attributes...", Cmd::Attributes, ""),
            mi("&Pack...", Cmd::Pack, "Alt+F5"),
            mi("&Unpack Specific Files...", Cmd::Unpack, "Alt+F9"),
            off("&Test Archive(s)", "Alt+Shift+F9"),
            SEP,
            mi("Compare By &Content...", Cmd::CompareContent, ""),
            off("Associate &With...", ""),
            mi("P&roperties...", Cmd::Properties, "Alt+Enter"),
            mi("Calculate &Occupied Space...", Cmd::CalcSpace, "Ctrl+L"),
            mi("&Multi-Rename Tool...", Cmd::MultiRename, "Ctrl+M"),
            off("&Edit Comment...", "Ctrl+Z"),
            SEP,
            off("Pr&int", ""),
            off("&Split File...", ""),
            off("Com&bine Files...", ""),
            off("E&ncode File (MIME, UUE, XXE)...", ""),
            off("&Decode File (MIME, UUE, XXE, BinHex)...", ""),
            off("Create C&hecksum File(s) (CRC, MD5, SHA1)...", ""),
            off("&Verify Checksums (CRC, MD5, SHA1)...", ""),
            SEP,
            mi("&Quit", Cmd::Exit, "Alt+F4"),
        ],
    },
    Menu {
        title: "&Mark",
        items: &[
            mi("&Select Group...", Cmd::SelectGroup, "Num +"),
            mi("&Unselect Group...", Cmd::UnselectGroup, "Num -"),
            mi("Select &All", Cmd::SelectAll, "Ctrl+Num +"),
            mi("U&nselect All", Cmd::UnselectAll, "Ctrl+Num -"),
            mi("&Invert Selection", Cmd::InvertSelection, "Num *"),
            mi("Select All With Same &Extension", Cmd::SelectSameExt, "Alt+Num +"),
            mi("Unselect All With Same E&xtension", Cmd::UnselectSameExt, "Alt+Num -"),
            SEP,
            mi("&Restore Selection", Cmd::RestoreSelection, "Num /"),
            off("Sa&ve Selection", ""),
            off("Save Selection to &File...", ""),
            off("&Load Selection from File...", ""),
            off("Load Selection from &Clipboard", ""),
            SEP,
            mi("Copy Selected Names to Clip&board", Cmd::CopyNamesToClip, ""),
            mi("Copy Names with Full &Path", Cmd::CopyFullNamesToClip, "Ctrl+Shift+C"),
            SEP,
            mi("Compare &Directories", Cmd::CompareDirs, "Shift+F2"),
            off("Mark Newer, Hide Same Files", ""),
        ],
    },
    Menu {
        title: "&Commands",
        items: &[
            mi("CD &Tree...", Cmd::Tree, "Alt+F10"),
            mi("&Search...", Cmd::Find, "Alt+F7"),
            off("&Volume Label...", ""),
            off("System &Information...", ""),
            SEP,
            off("Synchronize Dirs...", ""),
            mi("Compare &Directories", Cmd::CompareDirs, "Shift+F2"),
            SEP,
            mi("Open Command &Prompt Window", Cmd::OpenTerminal, ""),
            off("Run DOS", ""),
            SEP,
            mi("Directory &Hotlist", Cmd::Hotlist, "Ctrl+D"),
            mi("Go &Back", Cmd::GoBack, "Alt+Left"),
            mi("Go &Forward", Cmd::GoForward, "Alt+Right"),
            mi("Target = Source", Cmd::TargetEqSource, ""),
            mi("Swap Panels", Cmd::SwapPanels, "Ctrl+U"),
            SEP,
            mi("&Multi-Rename Tool...", Cmd::MultiRename, "Ctrl+M"),
        ],
    },
    Menu {
        title: "&Net",
        items: &[
            off("FTP &Connect...", "Ctrl+F"),
            off("FTP &New Connection...", "Ctrl+N"),
            off("FTP &Disconnect", "Ctrl+Shift+F"),
            off("Show FTP &Log", ""),
            SEP,
            off("Network Neighborhood", ""),
            off("&Share Current Directory...", ""),
            off("&Unshare Directory...", ""),
            SEP,
            off("Download From List...", ""),
        ],
    },
    Menu {
        title: "&Show",
        items: &[
            mi("&Brief", Cmd::Brief, "Ctrl+F1"),
            mi("&Full", Cmd::Full, "Ctrl+F2"),
            mi("&Tree", Cmd::TreeView, "Ctrl+F8"),
            mi("&Quick View Panel", Cmd::QuickView, "Ctrl+Q"),
            off("&Vertical Arrangement", ""),
            SEP,
            mi("&Name", Cmd::SortName, "Ctrl+F3"),
            mi("&Extension", Cmd::SortExt, "Ctrl+F4"),
            mi("T&ime", Cmd::SortDate, "Ctrl+F5"),
            mi("&Size", Cmd::SortSize, "Ctrl+F6"),
            mi("&Unsorted", Cmd::SortUnsorted, "Ctrl+F7"),
            mi("&Reversed Order", Cmd::ReverseSort, ""),
            SEP,
            mi("&All Files", Cmd::AllFiles, "Ctrl+F10"),
            mi("&Programs", Cmd::ProgramsOnly, "Ctrl+F11"),
            mi("&Custom...", Cmd::CustomFilter, "Ctrl+F12"),
            mi("Show &Hidden/System Files", Cmd::ToggleHidden, "Ctrl+H"),
            SEP,
            mi("Reread &Source", Cmd::Reread, "Ctrl+R"),
            mi("Branch View (With Subdirs)", Cmd::BranchView, "Ctrl+B"),
        ],
    },
    Menu {
        title: "C&onfiguration",
        items: &[
            mi("&Options...", Cmd::Options, ""),
            off("&Display", ""),
            off("&Layout", ""),
            SEP,
            mi("&Save Position", Cmd::SavePosition, ""),
            off("Change Settings Files &Directory...", ""),
        ],
    },
    Menu { title: "S&tart", items: &[off("(user commands)", ""), SEP, off("Change Start Menu...", "")] },
    Menu {
        title: "&Help",
        items: &[
            mi("&Index", Cmd::Help, "F1"),
            mi("&Keyboard", Cmd::Keyboard, ""),
            off("&Registration Info", ""),
            SEP,
            mi("Visit total-commacer's &Homepage", Cmd::Homepage, ""),
            off("Check for new &version", ""),
            SEP,
            mi("&About total-commacer...", Cmd::About, ""),
        ],
    },
];

/// The keyboard reference shown by Help > Keyboard.
pub const KEYBOARD_HELP: &str = "F1  Help                          F2  Reread source directory
F3  View file                     F4  Edit file
F5  Copy                          F6  Rename or move
F7  Create directory              F8  Delete
F9/F10  Activate menu             Alt+F4  Exit

Shift+F2  Compare directories     Shift+F3  View file under cursor
Shift+F4  Edit new text file      Shift+F5  Copy files (same dir)
Shift+F6  Rename file             Alt+F1/F2  Choose left/right drive
Alt+F5  Pack files                Alt+F7  Find files
Alt+F8  Command line history      Alt+F9  Unpack
Alt+F10  Directory tree           Alt+Enter  Properties
Alt+Shift+Enter  Count all dirs   Alt+Left/Right  Back / forward

Ctrl+F1  Brief view               Ctrl+F2  Full view
Ctrl+F3  Sort by name             Ctrl+F4  Sort by extension
Ctrl+F5  Sort by date             Ctrl+F6  Sort by size
Ctrl+F7  Unsorted                 Ctrl+F8  Tree
Ctrl+F10  All files               Ctrl+F11  Programs only
Ctrl+F12  Custom filter           Ctrl+H  Show hidden/system files
Ctrl+R  Reread                    Ctrl+Q  Quick view
Ctrl+U  Swap panels               Ctrl+B  Branch view
Ctrl+T / Ctrl+W  New / close tab  Ctrl+Tab  Next tab
Ctrl+D  Directory hotlist         Ctrl+L  Calculate occupied space
Ctrl+Left/Right  Open dir in other panel
Ctrl+PgUp / Backspace  Parent     Ctrl+PgDn  Open directory
Ctrl+\\  Root directory           Ctrl+Enter  Name to command line
Ctrl+Shift+Enter  Path+name to command line
Ctrl+P  Path to command line     Ctrl+E / Ctrl+Down  Command history

Insert  Select and move down      Space  Select, count directory
Num +  Select group               Num -  Unselect group
Num *  Invert selection           Ctrl+Num +/-  Select/unselect all
Alt+Num +/-  (Un)select same ext  Shift+arrows  Select while moving
Tab  Switch panel                 Ctrl+Alt+Letter  Quick search
Letters  Go to the command line   Esc  Clear command line";
