//! `ff-art` -- FerrisFilter's art extraction dev tool.
//!
//! Reads a Path of Exile 2 install **read-only** (its `Bundles2` index, through the
//! `poe_data_tools` library) and writes the UI art listed in `assets/poe2/manifest.toml` into the
//! repo as PNGs (plus the Fontin fonts). Re-run it after a game patch to refresh the art. It also
//! renders the static Ferris background (`assets/ui/ferris-bg.png`) from the committed SVG, which
//! needs no game install.
//!
//! This crate is a non-default member of the repo-root workspace and is never a dependency of the
//! `ff` overlay; the overlay embeds the committed PNGs with `include_bytes!`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use image::{GenericImageView, RgbaImage};
use poe_data_tools::fs::{FS, FileSystem};
use serde::Deserialize;

const DEFAULT_GAME: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Path of Exile 2";

#[derive(Parser)]
#[command(about = "Extract Path of Exile 2 UI art into FerrisFilter's assets/")]
struct Cli {
    /// The Path of Exile 2 install folder (read-only; must hold Bundles2/_.index.bin).
    #[arg(long, global = true, default_value = DEFAULT_GAME)]
    game: PathBuf,
    /// The FerrisFilter repo root (outputs are written below it). Defaults to the repo root this
    /// binary was built from.
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Extract every manifest entry and render both Ferris images (the default).
    Extract,
    /// Print the game's virtual paths containing PATTERN (case-insensitive).
    List { pattern: String },
    /// Convert every .dds whose path contains PATTERN to a PNG under DIR (for exploring; DIR
    /// should be a scratch folder, never the repo).
    Dump { pattern: String, dir: PathBuf },
    /// Render the Ferris background and the orange ABOUT Ferris (no game install needed).
    Ferris,
    /// Crop `[x, y, w, h]` out of a PNG and enlarge it `zoom` times (nearest neighbour), to
    /// inspect screenshots of the skinned UI up close.
    Zoom {
        input: PathBuf,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        out: PathBuf,
        #[arg(long, default_value_t = 3)]
        zoom: u32,
    },
    /// Print each PNG's size, the bounding box of its non-transparent pixels and the alpha of
    /// its four edge midpoints (to find padding/bleed gutters before writing a crop).
    Info { files: Vec<PathBuf> },
    /// Build a contact sheet (8 columns, square cells, on a dark-grey ground) of the PNGs listed
    /// one per line in LIST, to eyeball candidates found with `dump`. Prints the cell order.
    Sheet {
        list: PathBuf,
        out: PathBuf,
        /// Cell size in pixels.
        #[arg(long, default_value_t = 160)]
        cell: u32,
    },
}

#[derive(Deserialize)]
struct Manifest {
    #[serde(default)]
    image: Vec<ImageEntry>,
    #[serde(default)]
    atlas: Vec<AtlasEntry>,
    #[serde(default)]
    font: Vec<FontEntry>,
    ferris: Option<FerrisEntry>,
    ferris_about: Option<FerrisAboutEntry>,
}

#[derive(Deserialize)]
struct ImageEntry {
    /// Game virtual path of the .dds (the base-resolution one).
    path: String,
    /// Output file name, relative to `assets/poe2/`.
    out: String,
    /// Try the `4K/` sibling (same folder + `4K/`) first and fall back to `path`.
    #[serde(default)]
    prefer_4k: bool,
    /// Drop the transparent gutter the game's textures carry on their right and bottom edges
    /// (about 10px at 4K): crop to `[0, 0, max_x, max_y]` of the non-transparent pixels. Keeps
    /// the top-left anchor, so pieces that join (3-slice buttons, tabs) still line up.
    #[serde(default)]
    trim: bool,
    /// `[x, y, w, h]` crop, in the pixels of the image actually used (the 4K one when
    /// `prefer_4k` finds it; the tool warns when it falls back while a crop is set).
    crop: Option<[u32; 4]>,
    /// `[x, y, w, h]` regions (after cropping) made fully transparent -- e.g. the gear in the
    /// middle of the Options page frame, which the overlay replaces with its own background.
    #[serde(default)]
    clear: Vec<[u32; 4]>,
}

/// A trimmed atlas: picks cells out of a grid sheet (the minimap icon sheet) and packs them
/// into a smaller grid, so the repo carries only the cells the UI uses.
#[derive(Deserialize)]
struct AtlasEntry {
    /// Game virtual path of the source sheet (.dds).
    path: String,
    /// Output PNG, relative to `assets/poe2/`.
    out: String,
    /// Cell size in pixels (square cells, no spacing).
    cell: u32,
    /// Cells per row in the source sheet.
    columns: u32,
    /// Output rows, each a list of source cell numbers (row-major from 0). Every row must have
    /// the same length.
    cells: Vec<Vec<u32>>,
}

#[derive(Deserialize)]
struct FontEntry {
    path: String,
    out: String,
}

#[derive(Deserialize)]
struct FerrisAboutEntry {
    /// Source SVG, relative to the repo root.
    svg: String,
    /// Output PNG, relative to the repo root.
    out: String,
    /// Output width in pixels (the artwork is cropped tight and scaled to exactly this).
    width: u32,
}

#[derive(Deserialize)]
struct FerrisEntry {
    /// Source SVG, relative to the repo root.
    svg: String,
    /// Output PNG, relative to the repo root.
    out: String,
    /// Output width in pixels (height follows the window aspect below).
    width: u32,
    /// Output height in pixels.
    height: u32,
    /// Width of the crab inside the image, as a fraction of `width`.
    crab_fraction: f32,
    /// Vertical centre of the crab as a fraction of `height`.
    crab_center_y: f32,
    /// Peak alpha (0-255) of the white lines.
    line_alpha: u8,
    /// Line half-thickness in pixels.
    line_radius: u32,
}

/// The repo root this binary was built from (tools/ff-art -> tools -> root), independent of the
/// working directory. Not canonicalized, which would give a `\?` verbatim path on Windows.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("CARGO_MANIFEST_DIR has no grandparent")
        .to_path_buf()
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = match &cli.root {
        Some(r) => r.clone(),
        None => repo_root(),
    };
    let manifest_path = root.join("assets/poe2/manifest.toml");
    let manifest: Manifest = toml::from_str(
        &std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("reading {}", manifest_path.display()))?,
    )
    .with_context(|| format!("parsing {}", manifest_path.display()))?;

    match cli.cmd.unwrap_or(Cmd::Extract) {
        Cmd::Extract => {
            let fs = open_game(&cli.game)?;
            extract(&fs, &manifest, &root)?;
            ferris(&manifest, &root)?;
            ferris_about(&manifest, &root)
        }
        Cmd::List { pattern } => {
            let fs = open_game(&cli.game)?;
            let p = pattern.to_lowercase();
            for path in fs.list().filter(|f| f.to_lowercase().contains(&p)) {
                println!("{path}");
            }
            Ok(())
        }
        Cmd::Dump { pattern, dir } => {
            let fs = open_game(&cli.game)?;
            dump(&fs, &pattern, &dir)
        }
        Cmd::Ferris => {
            ferris(&manifest, &root)?;
            ferris_about(&manifest, &root)
        }
        Cmd::Sheet { list, out, cell } => sheet(&list, &out, cell),
        Cmd::Zoom {
            input,
            x,
            y,
            w,
            h,
            out,
            zoom,
        } => {
            let img = image::open(&input)?.to_rgba8();
            let w = w.min(img.width().saturating_sub(x));
            let h = h.min(img.height().saturating_sub(y));
            let part = img.view(x, y, w, h).to_image();
            let z = zoom.max(1);
            let big =
                image::imageops::resize(&part, w * z, h * z, image::imageops::FilterType::Nearest);
            save_png(&big, &out)
        }
        Cmd::Info { files } => {
            for f in files {
                let img = image::open(&f)?.to_rgba8();
                let (w, h) = img.dimensions();
                let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
                for (x, y, p) in img.enumerate_pixels() {
                    if p[3] > 8 {
                        x0 = x0.min(x);
                        y0 = y0.min(y);
                        x1 = x1.max(x + 1);
                        y1 = y1.max(y + 1);
                    }
                }
                let a = |x: u32, y: u32| img.get_pixel(x, y)[3];
                println!(
                    "{w}x{h} opaque=[{x0},{y0},{},{}] edges L{} R{} T{} B{} {}",
                    x1.saturating_sub(x0),
                    y1.saturating_sub(y0),
                    a(0, h / 2),
                    a(w - 1, h / 2),
                    a(w / 2, 0),
                    a(w / 2, h - 1),
                    f.display()
                );
            }
            Ok(())
        }
    }
}

fn open_game(game: &Path) -> Result<FS> {
    image_extras::register();
    FS::from_steam(game.to_path_buf())
        .map_err(|e| anyhow::anyhow!("opening the game install {}: {e:?}", game.display()))
}

fn read_dds(fs: &FS, path: &str) -> Result<image::DynamicImage> {
    let bytes = fs
        .read(path)
        .map_err(|e| anyhow::anyhow!("reading {path}: {e:?}"))?;
    image::load_from_memory(&bytes).with_context(|| format!("decoding {path}"))
}

/// `a/b/Name.dds` -> `a/b/4K/Name.dds`.
fn four_k_sibling(path: &str) -> String {
    match path.rfind('/') {
        Some(i) => format!("{}/4K{}", &path[..i], &path[i..]),
        None => format!("4K/{path}"),
    }
}

fn extract(fs: &FS, manifest: &Manifest, root: &Path) -> Result<()> {
    let out_dir = root.join("assets/poe2");
    for entry in &manifest.image {
        let (img, used) = if entry.prefer_4k {
            let k = four_k_sibling(&entry.path);
            match read_dds(fs, &k) {
                Ok(img) => (img, k),
                Err(_) => {
                    if entry.crop.is_some() || !entry.clear.is_empty() {
                        eprintln!(
                            "warning: {} has no 4K variant; its crop/clear rects are 4K pixels",
                            entry.path
                        );
                    }
                    (read_dds(fs, &entry.path)?, entry.path.clone())
                }
            }
        } else {
            (read_dds(fs, &entry.path)?, entry.path.clone())
        };
        let mut img = img.to_rgba8();
        if entry.trim {
            let (mut x1, mut y1) = (0, 0);
            for (x, y, p) in img.enumerate_pixels() {
                if p[3] > 0 {
                    x1 = x1.max(x + 1);
                    y1 = y1.max(y + 1);
                }
            }
            img = img.view(0, 0, x1.max(1), y1.max(1)).to_image();
        }
        if let Some([x, y, w, h]) = entry.crop {
            if x + w > img.width() || y + h > img.height() {
                bail!(
                    "crop {:?} is outside {used} ({}x{})",
                    entry.crop,
                    img.width(),
                    img.height()
                );
            }
            img = img.view(x, y, w, h).to_image();
        }
        for &[x, y, w, h] in &entry.clear {
            for yy in y..(y + h).min(img.height()) {
                for xx in x..(x + w).min(img.width()) {
                    img.put_pixel(xx, yy, image::Rgba([0, 0, 0, 0]));
                }
            }
        }
        let out = out_dir.join(&entry.out);
        std::fs::create_dir_all(out.parent().unwrap())?;
        save_png(&img, &out)?;
        println!(
            "{:>4}x{:<4} {} <- {used}",
            img.width(),
            img.height(),
            entry.out
        );
    }
    for atlas in &manifest.atlas {
        let sheet = read_dds(fs, &atlas.path)?.to_rgba8();
        let img = pack_atlas(&sheet, atlas)?;
        let out = out_dir.join(&atlas.out);
        std::fs::create_dir_all(out.parent().unwrap())?;
        save_png(&img, &out)?;
        println!(
            "{:>4}x{:<4} {} <- {} ({} cells)",
            img.width(),
            img.height(),
            atlas.out,
            atlas.path,
            atlas.cells.iter().map(Vec::len).sum::<usize>()
        );
    }
    for font in &manifest.font {
        let bytes = fs
            .read(&font.path)
            .map_err(|e| anyhow::anyhow!("reading {}: {e:?}", font.path))?;
        let out = out_dir.join(&font.out);
        std::fs::create_dir_all(out.parent().unwrap())?;
        std::fs::write(&out, &bytes)?;
        println!("{:>9} B  {} <- {}", bytes.len(), font.out, font.path);
    }
    Ok(())
}

/// Copies the listed cells of `sheet` into a new grid (one output row per `cells` row).
fn pack_atlas(sheet: &RgbaImage, atlas: &AtlasEntry) -> Result<RgbaImage> {
    let c = atlas.cell;
    let cols = atlas.cells.first().map_or(0, Vec::len) as u32;
    if cols == 0 || atlas.cells.iter().any(|r| r.len() as u32 != cols) {
        bail!(
            "{}: every cells row must have the same, non-zero length",
            atlas.out
        );
    }
    let mut img = RgbaImage::new(cols * c, atlas.cells.len() as u32 * c);
    for (row, cells) in atlas.cells.iter().enumerate() {
        for (col, &n) in cells.iter().enumerate() {
            let (sx, sy) = ((n % atlas.columns) * c, (n / atlas.columns) * c);
            if sx + c > sheet.width() || sy + c > sheet.height() {
                bail!("{}: cell {n} is outside {}", atlas.out, atlas.path);
            }
            let part = sheet.view(sx, sy, c, c).to_image();
            image::imageops::replace(
                &mut img,
                &part,
                (col as u32 * c) as i64,
                (row as u32 * c) as i64,
            );
        }
    }
    Ok(img)
}

fn dump(fs: &FS, pattern: &str, dir: &Path) -> Result<()> {
    let p = pattern.to_lowercase();
    let paths: Vec<String> = fs
        .list()
        .filter(|f| {
            let l = f.to_lowercase();
            l.ends_with(".dds") && l.contains(&p)
        })
        .collect();
    for path in &paths {
        match read_dds(fs, path) {
            Ok(img) => {
                let out = dir.join(path).with_extension("png");
                std::fs::create_dir_all(out.parent().unwrap())?;
                img.to_rgba8().save(&out)?;
                println!("{}x{} {path}", img.width(), img.height());
            }
            Err(e) => eprintln!("skip {path}: {e:#}"),
        }
    }
    Ok(())
}

/// Renders the Ferris SVG, turns it into pale white line art (edges of the filled shapes) and
/// places it on a transparent canvas sized for the prototype window.
fn ferris(manifest: &Manifest, root: &Path) -> Result<()> {
    let Some(f) = &manifest.ferris else {
        return Ok(());
    };
    let svg = std::fs::read(root.join(&f.svg)).with_context(|| format!("reading {}", f.svg))?;
    let tree = resvg::usvg::Tree::from_data(&svg, &resvg::usvg::Options::default())?;
    let size = tree.size();
    let crab_w = (f.width as f32 * f.crab_fraction).round() as u32;
    let scale = crab_w as f32 / size.width();
    let crab_h = (size.height() * scale).round() as u32;
    let mut pix = resvg::tiny_skia::Pixmap::new(crab_w, crab_h).context("pixmap")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pix.as_mut(),
    );

    // Edge strength from colour + alpha differences between neighbours (a Sobel-like gradient
    // over premultiplied RGBA), so every shape boundary -- the outline, the eyes, the mouth, the
    // claw splits -- becomes a line.
    let (w, h) = (crab_w as i32, crab_h as i32);
    let data = pix.data();
    let px = |x: i32, y: i32| -> [f32; 4] {
        let x = x.clamp(0, w - 1);
        let y = y.clamp(0, h - 1);
        let i = ((y * w + x) * 4) as usize;
        [
            data[i] as f32,
            data[i + 1] as f32,
            data[i + 2] as f32,
            data[i + 3] as f32,
        ]
    };
    let mut edge = vec![0f32; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let mut g = 0f32;
            for c in 0..4 {
                let gx = px(x + 1, y)[c] - px(x - 1, y)[c];
                let gy = px(x, y + 1)[c] - px(x, y - 1)[c];
                g = g.max((gx * gx + gy * gy).sqrt());
            }
            edge[(y * w + x) as usize] = (g / 120.0).min(1.0);
        }
    }
    // Thicken: max filter with a round kernel.
    let r = f.line_radius as i32;
    let mut thick = vec![0f32; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let mut m = 0f32;
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx * dx + dy * dy > r * r {
                        continue;
                    }
                    let (xx, yy) = (x + dx, y + dy);
                    if xx >= 0 && yy >= 0 && xx < w && yy < h {
                        m = m.max(edge[(yy * w + xx) as usize]);
                    }
                }
            }
            thick[(y * w + x) as usize] = m;
        }
    }

    let mut out = RgbaImage::new(f.width, f.height);
    let ox = (f.width - crab_w) / 2;
    let oy = ((f.height as f32 * f.crab_center_y) - crab_h as f32 / 2.0).max(0.0) as u32;
    for y in 0..crab_h {
        for x in 0..crab_w {
            let a = thick[(y as i32 * w + x as i32) as usize];
            if a > 0.0 && ox + x < f.width && oy + y < f.height {
                let alpha = (a * f.line_alpha as f32).round() as u8;
                out.put_pixel(ox + x, oy + y, image::Rgba([255, 255, 255, alpha]));
            }
        }
    }
    let out_path = root.join(&f.out);
    std::fs::create_dir_all(out_path.parent().unwrap())?;
    save_png(&out, &out_path)?;
    println!("{}x{} {} <- {}", f.width, f.height, f.out, f.svg);
    Ok(())
}

/// Renders `tree` at `scale` onto a transparent pixmap covering the whole SVG and returns it with
/// the bounding box `(min_x, min_y, max_x, max_y)` (inclusive) of its pixels with alpha > 0.
fn render_with_bbox(
    tree: &resvg::usvg::Tree,
    scale: f32,
) -> Result<(resvg::tiny_skia::Pixmap, (u32, u32, u32, u32))> {
    let size = tree.size();
    let pw = (size.width() * scale).ceil() as u32;
    let ph = (size.height() * scale).ceil() as u32;
    let mut pix = resvg::tiny_skia::Pixmap::new(pw, ph).context("pixmap")?;
    resvg::render(
        tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pix.as_mut(),
    );
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0u32, 0u32);
    for (i, p) in pix.pixels().iter().enumerate() {
        if p.alpha() > 0 {
            let (x, y) = (i as u32 % pw, i as u32 / pw);
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    anyhow::ensure!(x0 != u32::MAX, "the SVG rendered nothing");
    Ok((pix, (x0, y0, x1, y1)))
}

/// Renders the Ferris SVG in its original colours (no tint, no edge pass), cropped tight to the
/// artwork and exactly `width` pixels wide, for the ABOUT tab.
fn ferris_about(manifest: &Manifest, root: &Path) -> Result<()> {
    let Some(f) = &manifest.ferris_about else {
        return Ok(());
    };
    let svg = std::fs::read(root.join(&f.svg)).with_context(|| format!("reading {}", f.svg))?;
    let tree = resvg::usvg::Tree::from_data(&svg, &resvg::usvg::Options::default())?;
    // Pass 1 measures the artwork's share of the SVG; pass 2 scales so that share is `width`.
    let s1 = f.width as f32 / tree.size().width();
    let (_, (x0, _, x1, _)) = render_with_bbox(&tree, s1)?;
    let s2 = s1 * f.width as f32 / (x1 - x0 + 1) as f32;
    let (pix, (x0, y0, x1, y1)) = render_with_bbox(&tree, s2)?;
    let pw = pix.width();
    let h = y1 - y0 + 1;
    let mut out = RgbaImage::new(f.width, h);
    for y in 0..h {
        for x in 0..f.width.min(x1 - x0 + 1) {
            let p = pix.pixels()[((y0 + y) * pw + x0 + x) as usize].demultiply();
            out.put_pixel(x, y, image::Rgba([p.red(), p.green(), p.blue(), p.alpha()]));
        }
    }
    let out_path = root.join(&f.out);
    std::fs::create_dir_all(out_path.parent().unwrap())?;
    save_png(&out, &out_path)?;
    println!("{}x{} {} <- {}", f.width, h, f.out, f.svg);
    Ok(())
}

fn sheet(list: &Path, out: &Path, cell: u32) -> Result<()> {
    let cell = cell.max(16);
    let files: Vec<String> = std::fs::read_to_string(list)?
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect();
    let cols = (files.len() as u32).clamp(1, 8);
    let rows = (files.len() as u32).div_ceil(cols).max(1);
    let mut canvas =
        RgbaImage::from_pixel(cols * cell, rows * cell, image::Rgba([12, 12, 12, 255]));
    for (i, f) in files.iter().enumerate() {
        let img = image::open(f).with_context(|| format!("opening {f}"))?;
        let thumb = img.resize(cell - 8, cell - 8, image::imageops::FilterType::Triangle);
        let (cx, cy) = ((i as u32 % cols) * cell + 4, (i as u32 / cols) * cell + 4);
        image::imageops::overlay(&mut canvas, &thumb.to_rgba8(), cx as i64, cy as i64);
        println!(
            "{i:>3} r{} c{} {}x{} {f}",
            i as u32 / cols,
            i as u32 % cols,
            img.width(),
            img.height()
        );
    }
    canvas.save(out)?;
    Ok(())
}

/// Writes a PNG with the strongest compression the encoder offers (the files are committed).
fn save_png(img: &RgbaImage, path: &Path) -> Result<()> {
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    let file = std::io::BufWriter::new(
        std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?,
    );
    let enc = PngEncoder::new_with_quality(file, CompressionType::Best, FilterType::Adaptive);
    img.write_with_encoder(enc)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}
