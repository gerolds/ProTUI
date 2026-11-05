// ADR: An ignored test draws the first-start setup screen at 120x40 in octant at the capture font's cell aspect and saves `captures/setup-120x40-octant.png` through terminal-graphics' capture; the font is `PROTOTYPE_CAPTURE_FONT` or Windows Terminal's bundled Cascadia Mono.
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use terminal_graphics::capture::CellFont;

use crate::app::{App, Screen};
use crate::preview::View;
use crate::settings::{CellAspect, Renderer};
use crate::ui;

/// The capture font: `PROTOTYPE_CAPTURE_FONT`, else `CascadiaMono.ttf` in a Windows Terminal
/// install folder (its copy has the octant glyphs; the system font does not).
fn font_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PROTOTYPE_CAPTURE_FONT") {
        return Some(PathBuf::from(path));
    }
    std::fs::read_dir("C:/Program Files/WindowsApps")
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|dir| {
            dir.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("Microsoft.WindowsTerminal_"))
        })
        .map(|dir| dir.join("CascadiaMono.ttf"))
        .find(|font| font.is_file())
}

#[test]
#[ignore = "needs a font with octant glyphs; writes captures/setup-120x40-octant.png"]
fn capture_setup_screen() {
    let font_path = font_path().expect("set PROTOTYPE_CAPTURE_FONT to a font with octants");
    let mut font = CellFont::load(&font_path).expect("font loads");
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let epoch = Instant::now();
    let mut view = View::load(Some(&repo.join("assets")), epoch).expect("sample place loads");
    let mut app = App::open(
        std::env::temp_dir().join("prototype-capture-unused"),
        None,
        None,
    );
    let aspect = (font.cell_aspect() * 100.0).round() as u16;
    let Screen::Setup(setup) = &mut app.screen else {
        panic!("a first start opens setup");
    };
    setup.draft.cell_aspect = CellAspect::new(aspect).expect("font aspect within 1.50-3.00");
    setup.draft.renderer = Renderer::Octant;
    let area = Rect::new(0, 0, 120, 40);
    let mut buf = Buffer::empty(area);
    ui::draw(
        &app,
        &mut view,
        area,
        &mut buf,
        epoch + Duration::from_secs(2),
    );
    let picture = font.rasterize(&buf).expect("every glyph is in the font");
    let out = repo.join("captures");
    std::fs::create_dir_all(&out).expect("captures folder");
    let path = out.join("setup-120x40-octant.png");
    picture.save_png(&path).expect("PNG written");
    println!(
        "{}: {:?} px, font {}, cell aspect {aspect}",
        path.display(),
        picture.size(),
        font_path.display()
    );
}
