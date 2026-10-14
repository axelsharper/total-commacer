//! File operations in a background thread with progress and overwrite questions.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Local};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OpKind {
    Copy,
    Move,
    Delete,
    Pack,
    Unpack,
}

#[derive(Clone, Debug)]
pub struct OpRequest {
    pub kind: OpKind,
    pub sources: Vec<PathBuf>,
    /// Destination directory (Copy/Move/Unpack) or archive file (Pack); unused for Delete.
    pub target: PathBuf,
    /// Single-file copy/move with a new name.
    pub rename_to: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Progress {
    pub from: String,
    pub to: String,
    pub file_frac: f32,
    pub total_frac: f32,
    pub done_files: usize,
    pub total_files: usize,
}

#[derive(Clone, Debug)]
pub enum Query {
    Overwrite { src: PathBuf, dst: PathBuf, src_size: u64, dst_size: u64, src_time: Option<DateTime<Local>>, dst_time: Option<DateTime<Local>> },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reply {
    Overwrite,
    OverwriteAll,
    Skip,
    SkipAll,
    Cancel,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Progress(Progress),
    Query(Query),
    Done,
    Error(String),
    Cancelled,
}

pub struct OpHandle {
    pub kind: OpKind,
    rx: Receiver<Msg>,
    reply_tx: Sender<Reply>,
    cancel: Arc<AtomicBool>,
    pub progress: Progress,
    pub pending: Option<Query>,
    pub finished: Option<Msg>,
}

impl OpHandle {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
    pub fn reply(&mut self, r: Reply) {
        self.pending = None;
        let _ = self.reply_tx.send(r);
    }
    /// Drain the channel; returns true when the operation has ended.
    pub fn poll(&mut self) -> bool {
        loop {
            match self.rx.try_recv() {
                Ok(Msg::Progress(p)) => self.progress = p,
                Ok(Msg::Query(q)) => self.pending = Some(q),
                Ok(m) => {
                    self.finished = Some(m);
                    return true;
                }
                Err(TryRecvError::Empty) => return self.finished.is_some(),
                Err(TryRecvError::Disconnected) => {
                    if self.finished.is_none() {
                        self.finished = Some(Msg::Done);
                    }
                    return true;
                }
            }
        }
    }
}

struct Ctx {
    tx: Sender<Msg>,
    reply_rx: Receiver<Reply>,
    cancel: Arc<AtomicBool>,
    overwrite_all: bool,
    skip_all: bool,
    total_bytes: u64,
    done_bytes: u64,
    total_files: usize,
    done_files: usize,
    last_report: Instant,
}

struct Cancelled;

impl Ctx {
    fn check(&self) -> Result<(), Cancelled> {
        if self.cancel.load(Ordering::Relaxed) {
            Err(Cancelled)
        } else {
            Ok(())
        }
    }
    fn report(&mut self, from: &Path, to: &Path, file_done: u64, file_size: u64, force: bool) {
        if !force && self.last_report.elapsed() < Duration::from_millis(40) {
            return;
        }
        self.last_report = Instant::now();
        let _ = self.tx.send(Msg::Progress(Progress {
            from: from.to_string_lossy().to_string(),
            to: to.to_string_lossy().to_string(),
            file_frac: if file_size > 0 { file_done as f32 / file_size as f32 } else { 1.0 },
            total_frac: if self.total_bytes > 0 { (self.done_bytes + file_done) as f32 / self.total_bytes as f32 } else { self.done_files as f32 / self.total_files.max(1) as f32 },
            done_files: self.done_files,
            total_files: self.total_files,
        }));
    }
    /// Ask the UI whether to overwrite `dst`. Ok(true) = overwrite, Ok(false) = skip.
    fn ask_overwrite(&mut self, src: &Path, dst: &Path) -> Result<bool, Cancelled> {
        if self.overwrite_all {
            return Ok(true);
        }
        if self.skip_all {
            return Ok(false);
        }
        let sm = fs::metadata(src).ok();
        let dm = fs::metadata(dst).ok();
        let _ = self.tx.send(Msg::Query(Query::Overwrite {
            src: src.to_path_buf(),
            dst: dst.to_path_buf(),
            src_size: sm.as_ref().map(|m| m.len()).unwrap_or(0),
            dst_size: dm.as_ref().map(|m| m.len()).unwrap_or(0),
            src_time: sm.and_then(|m| m.modified().ok()).map(DateTime::<Local>::from),
            dst_time: dm.and_then(|m| m.modified().ok()).map(DateTime::<Local>::from),
        }));
        loop {
            match self.reply_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(Reply::Overwrite) => return Ok(true),
                Ok(Reply::OverwriteAll) => {
                    self.overwrite_all = true;
                    return Ok(true);
                }
                Ok(Reply::Skip) => return Ok(false),
                Ok(Reply::SkipAll) => {
                    self.skip_all = true;
                    return Ok(false);
                }
                Ok(Reply::Cancel) => return Err(Cancelled),
                Err(_) => self.check()?,
            }
        }
    }
}

pub fn start(req: OpRequest) -> OpHandle {
    let (tx, rx) = channel();
    let (reply_tx, reply_rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let kind = req.kind;
    let total_files = req.sources.len();
    let c2 = cancel.clone();
    let tx2 = tx.clone();
    std::thread::spawn(move || {
        let mut ctx = Ctx {
            tx: tx2,
            reply_rx,
            cancel: c2,
            overwrite_all: false,
            skip_all: false,
            total_bytes: 0,
            done_bytes: 0,
            total_files: 0,
            done_files: 0,
            last_report: Instant::now() - Duration::from_secs(1),
        };
        let r = match req.kind {
            OpKind::Copy => run_copy(&mut ctx, &req, false),
            OpKind::Move => run_copy(&mut ctx, &req, true),
            OpKind::Delete => run_delete(&mut ctx, &req),
            OpKind::Pack => run_pack(&mut ctx, &req),
            OpKind::Unpack => run_unpack(&mut ctx, &req),
        };
        let _ = match r {
            Ok(()) => ctx.tx.send(Msg::Done),
            Err(OpErr::Cancel) => ctx.tx.send(Msg::Cancelled),
            Err(OpErr::Io(e)) => ctx.tx.send(Msg::Error(e)),
        };
    });
    OpHandle {
        kind,
        rx,
        reply_tx,
        cancel,
        progress: Progress { from: String::new(), to: String::new(), file_frac: 0.0, total_frac: 0.0, done_files: 0, total_files },
        pending: None,
        finished: None,
    }
}

enum OpErr {
    Cancel,
    Io(String),
}
impl From<Cancelled> for OpErr {
    fn from(_: Cancelled) -> Self {
        OpErr::Cancel
    }
}
fn ioerr(ctx: &str, p: &Path, e: &io::Error) -> OpErr {
    OpErr::Io(format!("{ctx}: {}\n{}", p.to_string_lossy(), crate::fs::err_text(e)))
}

fn walk_total(paths: &[PathBuf]) -> (u64, usize) {
    let mut bytes = 0;
    let mut files = 0;
    for p in paths {
        if p.is_dir() {
            for e in walkdir::WalkDir::new(p).into_iter().flatten() {
                if e.file_type().is_file() {
                    bytes += e.metadata().map(|m| m.len()).unwrap_or(0);
                    files += 1;
                }
            }
        } else {
            bytes += fs::metadata(p).map(|m| m.len()).unwrap_or(0);
            files += 1;
        }
    }
    (bytes, files)
}

fn run_copy(ctx: &mut Ctx, req: &OpRequest, mv: bool) -> Result<(), OpErr> {
    let (bytes, files) = walk_total(&req.sources);
    ctx.total_bytes = bytes;
    ctx.total_files = files.max(1);
    for src in &req.sources {
        ctx.check()?;
        let name = match &req.rename_to {
            Some(n) if req.sources.len() == 1 => n.clone(),
            _ => src.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
        };
        let dst = req.target.join(&name);
        if dst == *src {
            return Err(OpErr::Io("Source and destination are the same file!".into()));
        }
        if mv {
            // fast path: rename on the same volume
            if !dst.exists() && fs::rename(src, &dst).is_ok() {
                ctx.done_files += 1;
                ctx.report(src, &dst, 0, 0, true);
                continue;
            }
        }
        if src.is_dir() {
            if dst.starts_with(src) {
                return Err(OpErr::Io("Cannot copy a directory into itself!".into()));
            }
            copy_dir(ctx, src, &dst)?;
            if mv {
                fs::remove_dir_all(src).map_err(|e| ioerr("Cannot delete", src, &e))?;
            }
        } else {
            copy_file(ctx, src, &dst)?;
            if mv {
                fs::remove_file(src).map_err(|e| ioerr("Cannot delete", src, &e))?;
            }
        }
    }
    Ok(())
}

fn copy_dir(ctx: &mut Ctx, src: &Path, dst: &Path) -> Result<(), OpErr> {
    fs::create_dir_all(dst).map_err(|e| ioerr("Cannot create directory", dst, &e))?;
    let mut entries: Vec<_> = fs::read_dir(src).map_err(|e| ioerr("Cannot read", src, &e))?.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        ctx.check()?;
        let p = e.path();
        let d = dst.join(e.file_name());
        if p.is_dir() {
            copy_dir(ctx, &p, &d)?;
        } else {
            copy_file(ctx, &p, &d)?;
        }
    }
    Ok(())
}

fn copy_file(ctx: &mut Ctx, src: &Path, dst: &Path) -> Result<(), OpErr> {
    if dst.exists() && !ctx.ask_overwrite(src, dst)? {
        let n = fs::metadata(src).map(|m| m.len()).unwrap_or(0);
        ctx.done_bytes += n;
        ctx.done_files += 1;
        return Ok(());
    }
    let size = fs::metadata(src).map(|m| m.len()).unwrap_or(0);
    ctx.report(src, dst, 0, size, true);
    let mut fin = fs::File::open(src).map_err(|e| ioerr("Cannot open", src, &e))?;
    let mut fout = fs::File::create(dst).map_err(|e| ioerr("Cannot create", dst, &e))?;
    let mut buf = vec![0u8; 1 << 20];
    let mut done = 0u64;
    loop {
        if ctx.check().is_err() {
            drop(fout);
            let _ = fs::remove_file(dst);
            return Err(OpErr::Cancel);
        }
        let n = fin.read(&mut buf).map_err(|e| ioerr("Read error", src, &e))?;
        if n == 0 {
            break;
        }
        fout.write_all(&buf[..n]).map_err(|e| ioerr("Write error", dst, &e))?;
        done += n as u64;
        ctx.report(src, dst, done, size, false);
    }
    drop(fout);
    if let Ok(m) = fs::metadata(src) {
        if let Ok(t) = m.modified() {
            let _ = fs::File::options().write(true).open(dst).and_then(|f| f.set_modified(t));
        }
        let _ = fs::set_permissions(dst, m.permissions());
    }
    ctx.done_bytes += size;
    ctx.done_files += 1;
    ctx.report(src, dst, size, size, true);
    Ok(())
}

fn run_delete(ctx: &mut Ctx, req: &OpRequest) -> Result<(), OpErr> {
    let (_, files) = walk_total(&req.sources);
    ctx.total_files = files.max(req.sources.len()).max(1);
    for p in &req.sources {
        ctx.check()?;
        ctx.report(p, Path::new(""), 0, 0, true);
        if p.is_dir() && !p.is_symlink() {
            delete_dir(ctx, p)?;
        } else {
            fs::remove_file(p).or_else(|_| fs::remove_dir(p)).map_err(|e| ioerr("Cannot delete", p, &e))?;
            ctx.done_files += 1;
        }
    }
    ctx.report(Path::new(""), Path::new(""), 0, 0, true);
    Ok(())
}

fn delete_dir(ctx: &mut Ctx, dir: &Path) -> Result<(), OpErr> {
    for e in fs::read_dir(dir).map_err(|e| ioerr("Cannot read", dir, &e))?.flatten() {
        ctx.check()?;
        let p = e.path();
        if p.is_dir() && !p.is_symlink() {
            delete_dir(ctx, &p)?;
        } else {
            fs::remove_file(&p).or_else(|_| fs::remove_dir(&p)).map_err(|e| ioerr("Cannot delete", &p, &e))?;
            ctx.done_files += 1;
            ctx.report(&p, Path::new(""), 0, 0, false);
        }
    }
    fs::remove_dir(dir).map_err(|e| ioerr("Cannot delete", dir, &e))
}

fn run_pack(ctx: &mut Ctx, req: &OpRequest) -> Result<(), OpErr> {
    let (bytes, files) = walk_total(&req.sources);
    ctx.total_bytes = bytes;
    ctx.total_files = files.max(1);
    let f = fs::File::create(&req.target).map_err(|e| ioerr("Cannot create", &req.target, &e))?;
    let mut zw = zip::ZipWriter::new(f);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for src in &req.sources {
        let base = src.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        if src.is_dir() {
            for e in walkdir::WalkDir::new(src).into_iter().flatten() {
                ctx.check()?;
                let rel = e.path().strip_prefix(&base).unwrap_or(e.path()).to_string_lossy().replace('\\', "/");
                if e.file_type().is_dir() {
                    zw.add_directory(format!("{rel}/"), opts).map_err(|e| OpErr::Io(e.to_string()))?;
                } else {
                    pack_one(ctx, &mut zw, e.path(), &rel, opts)?;
                }
            }
        } else {
            let rel = src.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            pack_one(ctx, &mut zw, src, &rel, opts)?;
        }
    }
    zw.finish().map_err(|e| OpErr::Io(e.to_string()))?;
    Ok(())
}

fn pack_one(ctx: &mut Ctx, zw: &mut zip::ZipWriter<fs::File>, path: &Path, rel: &str, opts: zip::write::SimpleFileOptions) -> Result<(), OpErr> {
    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    ctx.report(path, Path::new(rel), 0, size, true);
    zw.start_file(rel, opts).map_err(|e| OpErr::Io(e.to_string()))?;
    let mut fin = fs::File::open(path).map_err(|e| ioerr("Cannot open", path, &e))?;
    let mut buf = vec![0u8; 1 << 20];
    let mut done = 0;
    loop {
        ctx.check()?;
        let n = fin.read(&mut buf).map_err(|e| ioerr("Read error", path, &e))?;
        if n == 0 {
            break;
        }
        zw.write_all(&buf[..n]).map_err(|e| OpErr::Io(e.to_string()))?;
        done += n as u64;
        ctx.report(path, Path::new(rel), done, size, false);
    }
    ctx.done_bytes += size;
    ctx.done_files += 1;
    Ok(())
}

fn run_unpack(ctx: &mut Ctx, req: &OpRequest) -> Result<(), OpErr> {
    let target = &req.target;
    fs::create_dir_all(target).map_err(|e| ioerr("Cannot create directory", target, &e))?;
    for arch in &req.sources {
        let f = fs::File::open(arch).map_err(|e| ioerr("Cannot open", arch, &e))?;
        let mut za = zip::ZipArchive::new(f).map_err(|e| OpErr::Io(format!("{}: {e}", arch.to_string_lossy())))?;
        ctx.total_files = za.len().max(1);
        let mut total = 0u64;
        for i in 0..za.len() {
            if let Ok(f) = za.by_index(i) {
                total += f.size();
            }
        }
        ctx.total_bytes = total;
        for i in 0..za.len() {
            ctx.check()?;
            let mut zf = za.by_index(i).map_err(|e| OpErr::Io(e.to_string()))?;
            let rel = match zf.enclosed_name() {
                Some(p) => p,
                None => continue,
            };
            let out = target.join(&rel);
            if zf.is_dir() {
                fs::create_dir_all(&out).map_err(|e| ioerr("Cannot create directory", &out, &e))?;
                continue;
            }
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent).map_err(|e| ioerr("Cannot create directory", parent, &e))?;
            }
            if out.exists() && !ctx.ask_overwrite(arch, &out)? {
                ctx.done_bytes += zf.size();
                ctx.done_files += 1;
                continue;
            }
            let size = zf.size();
            ctx.report(&rel, &out, 0, size, true);
            let mut fout = fs::File::create(&out).map_err(|e| ioerr("Cannot create", &out, &e))?;
            let mut buf = vec![0u8; 1 << 20];
            let mut done = 0;
            loop {
                ctx.check()?;
                let n = zf.read(&mut buf).map_err(|e| OpErr::Io(e.to_string()))?;
                if n == 0 {
                    break;
                }
                fout.write_all(&buf[..n]).map_err(|e| ioerr("Write error", &out, &e))?;
                done += n as u64;
                ctx.report(&rel, &out, done, size, false);
            }
            ctx.done_bytes += size;
            ctx.done_files += 1;
        }
    }
    Ok(())
}
