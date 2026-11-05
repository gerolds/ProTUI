// ADR: Wall outlines are derived from the tile grid: a corner is cut to 45° only where it is one step of a staircase, so rectangular rooms keep square corners and diagonal walls read smooth.
// ADR: Signed distance to the outline (exact Euclidean transform) drives wall lips, wall depth and ambient occlusion in the bake.
use crate::place::{Place, TILE_PX};

/// Pixel-resolution shape fields of a place, computed once per place.
pub struct Geometry {
    width: usize,
    height: usize,
    /// Distance in px to the outline: positive in walls, negative on open ground.
    sd: Vec<f32>,
    /// Open ground after corner cuts; light stops everywhere else.
    open: Vec<bool>,
    /// The legend index whose surface paints each pixel.
    owner: Vec<u8>,
}

struct Corners {
    width: usize,
    open_count: Vec<u8>,
    cut: Vec<bool>,
}

impl Corners {
    fn new(place: &Place) -> Corners {
        let (vw, vh) = (place.width() + 1, place.height() + 1);
        let mut open_count = vec![0u8; vw * vh];
        for vy in 0..vh {
            for vx in 0..vw {
                let (x, y) = (vx as i32, vy as i32);
                open_count[vy * vw + vx] = [(x - 1, y - 1), (x, y - 1), (x - 1, y), (x, y)]
                    .iter()
                    .filter(|&&(tx, ty)| !place.solid(tx, ty))
                    .count() as u8;
            }
        }
        let mut cut = vec![false; vw * vh];
        for vy in 0..vh {
            for vx in 0..vw {
                let count = open_count[vy * vw + vx];
                if count != 1 && count != 3 {
                    continue;
                }
                let opposite = 4 - count;
                cut[vy * vw + vx] =
                    [(0i32, -1i32), (0, 1), (-1, 0), (1, 0)]
                        .iter()
                        .any(|&(dx, dy)| {
                            let (nx, ny) = (vx as i32 + dx, vy as i32 + dy);
                            nx >= 0
                                && ny >= 0
                                && (nx as usize) < vw
                                && (ny as usize) < vh
                                && open_count[ny as usize * vw + nx as usize] == opposite
                        });
            }
        }
        Corners {
            width: vw,
            open_count,
            cut,
        }
    }

    /// Whether a point in tile units is open ground after corner cuts.
    fn open_at(&self, place: &Place, fx: f32, fy: f32) -> bool {
        let (vx, vy) = (fx.round(), fy.round());
        let index = vy as usize * self.width + vx as usize;
        if self.cut[index] && (fx - vx).abs() + (fy - vy).abs() < 0.5 {
            return self.open_count[index] == 3;
        }
        !place.solid(fx.floor() as i32, fy.floor() as i32)
    }
}

/// The legend index painting a pixel at `(fx, fy)` tile units whose openness is `open`: its own
/// tile when that matches, else the nearest neighbour toward the pixel's corner that does (a cut
/// corner shows the surface it was cut into).
fn owner(place: &Place, fx: f32, fy: f32, open: bool) -> u8 {
    let (tx, ty) = (fx.floor() as i32, fy.floor() as i32);
    let own = place.index(tx, ty).expect("pixels lie inside the grid");
    if place.solid(tx, ty) != open {
        return own;
    }
    let dx = if fx - (tx as f32) < 0.5 { -1 } else { 1 };
    let dy = if fy - (ty as f32) < 0.5 { -1 } else { 1 };
    [(tx + dx, ty), (tx, ty + dy), (tx + dx, ty + dy)]
        .into_iter()
        .find_map(|(x, y)| {
            let index = place.index(x, y)?;
            (place.solid(x, y) != open).then_some(index)
        })
        .unwrap_or(own)
}

impl Geometry {
    pub fn build(place: &Place) -> Geometry {
        let (width, height) = (place.width() * TILE_PX, place.height() * TILE_PX);
        let tile = TILE_PX as f32;
        let corners = Corners::new(place);

        const SUB: usize = 4;
        let mut coverage = vec![0.0f32; width * height];
        for y in 0..height {
            for x in 0..width {
                let mut open = 0;
                for sy in 0..SUB {
                    for sx in 0..SUB {
                        let fx = (x as f32 + (sx as f32 + 0.5) / SUB as f32) / tile;
                        let fy = (y as f32 + (sy as f32 + 0.5) / SUB as f32) / tile;
                        open += usize::from(corners.open_at(place, fx, fy));
                    }
                }
                coverage[y * width + x] = open as f32 / (SUB * SUB) as f32;
            }
        }

        let open: Vec<bool> = coverage.iter().map(|&c| c >= 0.5).collect();
        let solid: Vec<bool> = open.iter().map(|&o| !o).collect();
        let to_open = distance_transform(&open, width, height);
        let to_solid = distance_transform(&solid, width, height);
        let sd = (0..width * height)
            .map(|i| {
                let c = coverage[i];
                if c > 0.0 && c < 1.0 {
                    0.5 - c
                } else if open[i] {
                    -(to_solid[i] - 0.5)
                } else {
                    to_open[i] - 0.5
                }
            })
            .collect();
        let owner = (0..width * height)
            .map(|i| {
                let (fx, fy) = (
                    ((i % width) as f32 + 0.5) / tile,
                    ((i / width) as f32 + 0.5) / tile,
                );
                owner(place, fx, fy, open[i])
            })
            .collect();
        Geometry {
            width,
            height,
            sd,
            open,
            owner,
        }
    }

    /// Raster size in pixels.
    pub fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Whether the pixel at `(x, y)` is open ground; outside the raster it is not.
    pub fn open(&self, x: i32, y: i32) -> bool {
        !self.occluded(x, y)
    }

    /// Signed distance in px from the pixel at `(x, y)` to the outline: positive in walls,
    /// negative on open ground; `None` outside the raster.
    pub fn signed_distance(&self, x: i32, y: i32) -> Option<f32> {
        self.inside(x, y).map(|i| self.sd[i])
    }

    fn inside(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height)
            .then(|| y as usize * self.width + x as usize)
    }

    pub(crate) fn occluded(&self, x: i32, y: i32) -> bool {
        self.inside(x, y).is_none_or(|i| !self.open[i])
    }

    pub(crate) fn sd(&self) -> &[f32] {
        &self.sd
    }

    pub(crate) fn owner(&self) -> &[u8] {
        &self.owner
    }
}

/// Euclidean distance in px from every pixel to the nearest `target` pixel (Felzenszwalb and Huttenlocher).
pub fn distance_transform(target: &[bool], width: usize, height: usize) -> Vec<f32> {
    const FAR: f64 = 1e12;
    let mut grid: Vec<f64> = target.iter().map(|&t| if t { 0.0 } else { FAR }).collect();
    let n = width.max(height);
    let (mut line, mut out) = (vec![0.0f64; n], vec![0.0f64; n]);
    let (mut parabolas, mut bounds) = (vec![0usize; n], vec![0.0f64; n + 1]);
    for x in 0..width {
        for y in 0..height {
            line[y] = grid[y * width + x];
        }
        transform_line(&line[..height], &mut out, &mut parabolas, &mut bounds);
        for y in 0..height {
            grid[y * width + x] = out[y];
        }
    }
    for y in 0..height {
        line[..width].copy_from_slice(&grid[y * width..(y + 1) * width]);
        transform_line(&line[..width], &mut out, &mut parabolas, &mut bounds);
        grid[y * width..(y + 1) * width].copy_from_slice(&out[..width]);
    }
    grid.iter().map(|&d| d.sqrt() as f32).collect()
}

fn transform_line(f: &[f64], d: &mut [f64], v: &mut [usize], z: &mut [f64]) {
    let n = f.len();
    let mut k = 0usize;
    v[0] = 0;
    z[0] = f64::NEG_INFINITY;
    z[1] = f64::INFINITY;
    let intersect = |q: usize, p: usize| {
        let (qf, pf) = (q as f64, p as f64);
        ((f[q] + qf * qf) - (f[p] + pf * pf)) / (2.0 * qf - 2.0 * pf)
    };
    for q in 1..n {
        let mut s = intersect(q, v[k]);
        while s <= z[k] {
            k -= 1;
            s = intersect(q, v[k]);
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = f64::INFINITY;
    }
    k = 0;
    for (q, out) in d.iter_mut().enumerate().take(n) {
        while z[k + 1] < q as f64 {
            k += 1;
        }
        let diff = q as f64 - v[k] as f64;
        *out = diff * diff + f[v[k]];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::place::tests::legend;

    fn px(tiles: f32) -> i32 {
        (tiles * TILE_PX as f32) as i32
    }

    #[test]
    fn staircase_corners_are_cut_and_room_corners_stay_square() {
        let rows = "######\n#....#\n#...##\n#..###\n######";
        let place = Place::new(rows, legend(), [0; 3], 1).unwrap();
        let geo = Geometry::build(&place);
        // The room's top-left corner is not a staircase step: square.
        assert!(geo.open(px(1.0) + 1, px(1.0) + 1));
        assert!(geo.occluded(px(1.0) - 1, px(1.0) - 1));
        // Outer step corner at vertex (5, 2): open tile (4, 1) loses its corner to the wall.
        assert!(geo.occluded(px(5.0) - 2, px(2.0) - 2));
        assert!(
            geo.open(px(5.0) - 6, px(2.0) - 6),
            "the cut spans half a tile"
        );
        // Inner step corner at vertex (4, 2): solid tile (4, 2) gives its corner to the floor.
        assert!(geo.open(px(4.0) + 2, px(2.0) + 2));
        assert!(geo.occluded(px(4.0) + 6, px(2.0) + 6));
        // The cut is 45°: open below the diagonal line through the step, solid above.
        let sd = geo.signed_distance(px(4.5), px(2.0) - 1).unwrap();
        assert!(sd.abs() < 1.5, "pixel on the cut line, sd {sd}");
        assert!(geo.signed_distance(px(2.5), px(2.5)).unwrap() < -10.0);
        assert!(geo.signed_distance(-1, 0).is_none());
    }

    #[test]
    fn distance_transform_is_euclidean() {
        let mut target = vec![false; 25];
        target[0] = true;
        let d = distance_transform(&target, 5, 5);
        assert_eq!(d[0], 0.0);
        assert!((d[4 * 5 + 3] - 5.0).abs() < 1e-6);
    }
}
