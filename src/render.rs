//! Contains overall render logic.

use crate::draw::*;
use crate::mat::*;
use crate::opts::*;
use crate::ray::*;
use crate::vec::*;

/// Iterates through all objects in objs and return the index of the _closest_ object and the hit point.
/// Returns `None` if nothing hits.
pub fn closest_hit(
    r: &Ray,
    objs: &[Box<dyn RayInteraction>],
    lim: (f32, f32),
) -> Option<(usize, Vector)> {
    let mut best_t = f32::INFINITY;
    let mut best = None;

    for (i, obj) in objs.iter().enumerate() {
        if let HitType::Hit(t) = obj.hit(r, lim) {
            let p = r.o + Vector::from_s(t, 3) * r.d;
            if t < best_t {
                best = Some((i, p));
                best_t = t;
            }
        }
    }

    best
}

/// Ierates through all objects in objs and return the index of the _first_ object to hit and the hit point.
/// Returns `None` if nothing hits.
pub fn any_hit(
    r: &Ray,
    objs: &[Box<dyn RayInteraction>],
    lim: (f32, f32),
) -> Option<(usize, Vector)> {
    for (i, obj) in objs.iter().enumerate() {
        if let HitType::Hit(t) = obj.hit(r, lim) {
            let p = r.o + Vector::from_s(t, 3) * r.d;
            return Some((i, p));
        }
    }

    None
}

/// Renders a scene containing objects in `objs`, lights in `lights`, and configuration information in `cfg`.
/// Returns a Matrix of colors representing RGB values of the final image.
pub fn render(
    start: (i32, i32),
    dims: (usize, usize),
    objs: &[Box<dyn RayInteraction>],
    lights: &[Light],
    cfg: &Config,
) -> Matrix<Color> {
    let view_dist = 0.5; // distance from camera to viewport
    let view_width = 1.0; // width of viewport
    let view_height = 1.0 * dims.1 as f32 / dims.0 as f32; // height of viewport, transformed to make the viewport square regardless of the output dimensions

    let di = (dims.0 as i32, dims.1 as i32);

    // adding an extra row and column to make canvas bounds symmetrical
    let pixels = dims.0 * dims.1 + dims.0 + dims.1 + 1;
    let mut buf = Matrix {
        mat: vec![
            Color {
                r: 255,
                g: 255,
                b: 255
            };
            pixels
        ],
        rlen: dims.0 as usize + 1, // write increased bounds to matrix dimensions as well, since we don't use it here
        clen: dims.1 as usize + 1,
    };

    for y in -di.1 / 2..di.1 / 2 {
        for x in -di.0 / 2..di.0 / 2 {
            let xf = x as f32;
            let yf = y as f32;

            // transform canvas coordinates to viewport coordinates
            // note that the viewport axis and scale is the same of the canvas, so the transform is just a scaling op
            let view_coord = Vector::from_3(
                (xf + start.0 as f32) * view_width / dims.0 as f32,
                (yf - start.1 as f32) * view_height / dims.1 as f32,
                view_dist,
            );

            let cv = cfg.world.cam_pos;
            // create ray coming off viewport
            let v_ray = Ray {
                o: cv,
                d: view_coord, // can adjust rotation by multiplying by rotation matrix here
            };

            if let Some((i, p)) = closest_hit(&v_ray, objs, (view_dist, f32::INFINITY)) {
                let mut color_v = Vector::zero(3);
                for l in lights {
                    color_v = color_v + light(i, objs, &p, l, cfg.render.max_reflections);
                }

                // clamp sum of light colors to correct output range and multiply by surface color
                let color_v = (objs[i].material(&p).color * color_v).clamp(0.0, 1.0);
                draw_pixel(&mut buf, (x, y), dims, map_color(color_v));
            }
        }
    }

    buf
}
