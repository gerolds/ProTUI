// ADR: `View` is the render-side state: the sample place's `World` (built once at start-up), its reused light `Frame`, the calibration picture (rebuilt only when its panel size or aspect changes) and the frame counter. Game state stays in `App`.
// ADR: Previews follow the setup draft: pictures use the renderer's encoder (ascii falls back to quadrant); the place draws lit in octant or quadrant, or as its raw ASCII view. Only a lit place flickers, so only it makes the screen animate.
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use terminal_graphics::{Encoder, ImageError, Ink, Rgb, StillImage, calibration, encode_image};
use terminal_tilemap::{
    Camera, CameraError, Frame, LightError, LoadError, TileRect, World, draw_raw, load_place,
};

use crate::settings::{CellAspect, Renderer};

/// The sample place, relative to the assets folder.
pub const PLACE: &str = "samples/place.toml";
/// Most cells per tile edge the place preview zooms to.
const ZOOM_LIMIT: f32 = 4.0;

/// Why a preview could not draw; shown in its panel.
#[derive(Debug)]
pub enum PreviewError {
    Image(ImageError),
    Camera(CameraError),
    Light(LightError),
}

impl fmt::Display for PreviewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PreviewError::Image(e) => write!(f, "picture: {e}"),
            PreviewError::Camera(e) => write!(f, "camera: {e}"),
            PreviewError::Light(e) => write!(f, "light: {e}"),
        }
    }
}

/// Why the sample place could not be loaded at start-up.
#[derive(Debug)]
pub enum StartError {
    NoAssets,
    Place(LoadError),
}

impl fmt::Display for StartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StartError::NoAssets => write!(
                f,
                "no assets folder found above the crate; pass --assets <dir>"
            ),
            StartError::Place(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for StartError {}

/// The encoder pictures use for `renderer`.
pub fn picture_encoder(renderer: Renderer) -> Encoder {
    match renderer {
        Renderer::Octant => Encoder::Octant,
        Renderer::Quadrant | Renderer::Ascii => Encoder::Quadrant,
    }
}

/// Frames drawn in the last whole second.
struct Fps {
    second: Instant,
    count: u32,
    last: u32,
}

impl Fps {
    fn tick(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.second);
        if elapsed >= Duration::from_secs(1) {
            // A gap over a second with no frames counts as zero.
            self.last = if elapsed < Duration::from_secs(2) {
                self.count
            } else {
                0
            };
            self.second += Duration::from_secs(elapsed.as_secs());
            self.count = 0;
        }
        self.count += 1;
    }
}

pub struct View {
    world: World,
    frame: Frame,
    epoch: Instant,
    /// The calibration picture and its key: columns, rows, aspect in hundredths.
    calibration: Option<((u16, u16, u16), StillImage)>,
    /// Whether the last drawn frame showed the lit place.
    place_lit: bool,
    fps: Fps,
}

impl View {
    /// The view of the sample place in `assets`; `epoch` starts the flicker clock.
    pub fn load(assets: Option<&Path>, epoch: Instant) -> Result<View, StartError> {
        let assets = assets.ok_or(StartError::NoAssets)?;
        let place = load_place(&place_path(assets)).map_err(StartError::Place)?;
        Ok(View::new(World::new(place), epoch))
    }

    pub fn new(world: World, epoch: Instant) -> View {
        View {
            world,
            frame: Frame::default(),
            epoch,
            calibration: None,
            place_lit: false,
            fps: Fps {
                second: epoch,
                count: 0,
                last: 0,
            },
        }
    }

    /// Whether the last frame showed something that moves (the lit place's flicker).
    pub fn animating(&self) -> bool {
        self.place_lit
    }

    /// Starts a frame at `now`: counts it and forgets what the last one showed.
    pub fn begin(&mut self, now: Instant) {
        self.fps.tick(now);
        self.place_lit = false;
    }

    /// Frames drawn in the last whole second.
    pub fn fps(&self) -> u32 {
        self.fps.last
    }

    /// The circle and square at `aspect`, encoded for `renderer`, filling `area`.
    pub fn draw_calibration(
        &mut self,
        area: Rect,
        aspect: CellAspect,
        renderer: Renderer,
        buf: &mut Buffer,
    ) -> Result<(), PreviewError> {
        if area.is_empty() {
            return Ok(());
        }
        let key = (area.width, area.height, aspect.hundredths());
        if self.calibration.as_ref().is_none_or(|(k, _)| *k != key) {
            let ground = Ink::opaque(Rgb::hex(0x14161a)).map_err(PreviewError::Image)?;
            let ink = Ink::opaque(Rgb::hex(0xe8e4da)).map_err(PreviewError::Image)?;
            let image = calibration(area.width, area.height, aspect.get(), ground, ink)
                .map_err(PreviewError::Image)?;
            self.calibration = Some((key, image));
        }
        if let Some((_, image)) = &self.calibration {
            encode_image(image.raster(), picture_encoder(renderer), area, buf);
        }
        Ok(())
    }

    /// The whole sample place fitted into `area` at `aspect` in `renderer`, lit at `now`.
    pub fn draw_place(
        &mut self,
        area: Rect,
        aspect: CellAspect,
        renderer: Renderer,
        now: Instant,
        buf: &mut Buffer,
    ) -> Result<(), PreviewError> {
        if area.is_empty() {
            return Ok(());
        }
        let place = self.world.place();
        let (w, h) = (place.width(), place.height());
        let centre = (w as f32 / 2.0, h as f32 / 2.0);
        let encoder = match renderer {
            Renderer::Octant => Encoder::Octant,
            Renderer::Quadrant => Encoder::Quadrant,
            Renderer::Ascii => {
                draw_raw(place, &Camera::raw(area, (w, h), centre), area, buf);
                return Ok(());
            }
        };
        let camera = Camera::overview(
            area,
            TileRect::whole(w, h),
            centre,
            ZOOM_LIMIT,
            aspect.get(),
        )
        .map_err(PreviewError::Camera)?;
        let t = now.saturating_duration_since(self.epoch).as_secs_f32();
        self.world
            .render(&mut self.frame, t, &camera, encoder, area, buf)
            .map_err(PreviewError::Light)?;
        self.place_lit = true;
        Ok(())
    }
}

pub fn place_path(assets: &Path) -> PathBuf {
    assets.join(PLACE)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A view of a tiny lit test room, cheap to build.
    pub fn small_view() -> View {
        let source = "seed = 1\nambient = \"#303030\"\nmap = '''\n#####\n#.L.#\n#####\n'''\n\
[legend.\"#\"]\nsolid = true\ncolor = \"#808080\"\npattern = \"flat\"\ngrain = 0.0\ngrain_scale = 1.0\n\
[legend.\".\"]\nsolid = false\ncolor = \"#a0a0a0\"\npattern = \"flat\"\ngrain = 0.0\ngrain_scale = 1.0\n\
[legend.\"L\"]\nsolid = false\ncolor = \"#a0a0a0\"\npattern = \"flat\"\ngrain = 0.0\ngrain_scale = 1.0\n\
light = { color = \"#ffffff\", radius = 3.0, intensity = 1.0, flicker = 0.2, glow = 1.0 }\n";
        let place = terminal_tilemap::parse_place(source, "test.toml").expect("test room parses");
        View::new(World::new(place), Instant::now())
    }

    #[test]
    fn fps_counts_the_last_whole_second() {
        let start = Instant::now();
        let mut fps = Fps {
            second: start,
            count: 0,
            last: 0,
        };
        for n in 0..30 {
            fps.tick(start + Duration::from_millis(n * 33));
        }
        assert_eq!(fps.last, 0, "the first second is not over");
        fps.tick(start + Duration::from_millis(1000));
        assert_eq!(fps.last, 30);
        fps.tick(start + Duration::from_millis(3500));
        assert_eq!(fps.last, 0, "a gap of over a second counts as no frames");
    }

    #[test]
    fn missing_assets_and_bad_place_are_start_errors() {
        let now = Instant::now();
        assert!(matches!(View::load(None, now), Err(StartError::NoAssets)));
        let dir = std::env::temp_dir().join(format!("prototype-view-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("samples")).expect("folder");
        let missing = View::load(Some(&dir), now)
            .err()
            .expect("no file")
            .to_string();
        assert!(missing.contains("place.toml"), "{missing}");
        std::fs::write(
            place_path(&dir),
            "seed = 1\nambient = \"#000000\"\nmap = 3\n",
        )
        .expect("writes");
        let bad = View::load(Some(&dir), now)
            .err()
            .expect("bad file")
            .to_string();
        assert!(bad.contains("place.toml:3:"), "{bad}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
