//! Defines how the .toml config file is read and interpreted.

use oxide::opts::*;
use oxide::ray::{Light, LightType, Material, Plane, RayInteraction, Sphere};
use oxide::vec::Vector;
use std::collections::HashMap;
use std::fs;
use std::thread;
use toml::Value;

type Triple = (
    Config,
    Vec<Box<dyn RayInteraction + Send + Sync>>,
    Vec<Light>,
);

/// Turns a .toml scene configuration file into a config struct.
/// The file `test_scene.toml` is pretty self-documenting, so check that for details.
pub fn read_cfg(path: &str) -> Option<Triple> {
    let cfg_str = fs::read_to_string(path).expect("could not read config file");
    let cfg = cfg_str
        .parse::<Value>()
        .expect("could not parse config file");

    // The base .toml file value should be a Table value type
    let cfg_base = cfg
        .try_into::<toml::map::Map<String, Value>>()
        .expect("could not cast into map");

    let mut c = Config {
        ..Default::default()
    }; // using struct update syntax to fill out field here

    // keep materials we've seen in a hash map for quick access
    let mut mats = HashMap::new();
    let mut objs: Vec<Box<dyn RayInteraction + Send + Sync>> = vec![];
    let mut lights = vec![];

    // turn a .toml array into a Vector with 3 elements
    let get_v3 = |parent: &Value, s| -> Vector {
        let vals = parent[s].as_array().expect("could not cast into array");
        Vector::from_3(
            vals[0].as_float().expect("could not cast into float") as f32,
            vals[1].as_float().expect("could not cast into float") as f32,
            vals[2].as_float().expect("could not cast into float") as f32,
        )
    };

    for (k, v) in cfg_base {
        match k.as_str() {
            "materials" => {
                let mat_map = v
                    .try_into::<toml::map::Map<String, Value>>()
                    .expect("could not cast material into map");
                for (name, mv) in mat_map {
                    let color = get_v3(&mv, "color");
                    let spec = mv["spec"].as_float().expect("could not cast into float") as f32;
                    let refl = mv["refl"].as_float().expect("could not cast into float") as f32;

                    mats.insert(name, Material { color, spec, refl });
                }
            }
            "objects" => {
                let obj_map = v
                    .try_into::<toml::map::Map<String, Value>>()
                    .expect("could not cast object into map");
                for (_, ov) in obj_map {
                    let t = ov["type"].as_str().expect("could not cast into string");
                    match t {
                        "sphere" => {
                            let c = get_v3(&ov, "center");
                            let r =
                                ov["radius"].as_float().expect("could not cast into float") as f32;
                            let mat =
                                mats[ov["material"].as_str().expect("could not cast into string")];

                            objs.push(Box::new(Sphere { c, r, mat }));
                        }
                        "plane" => {
                            let p = get_v3(&ov, "point");
                            let n = get_v3(&ov, "normal").norm();
                            let mat =
                                mats[ov["material"].as_str().expect("could not cast into string")];

                            objs.push(Box::new(Plane { p, n, mat }));
                        }
                        _ => panic!("found unknown object type!"),
                    }
                }
            }
            "lights" => {
                let light_map = v
                    .try_into::<toml::map::Map<String, Value>>()
                    .expect("could not cast light into map");
                for (_, lv) in light_map {
                    let kind = match lv["type"].as_str().expect("could not cast into string") {
                        "ambient" => LightType::Ambient,
                        "directional" => LightType::Directional(get_v3(&lv, "position")),
                        "point" => LightType::Point(get_v3(&lv, "position")),
                        _ => panic!("unknown light type"),
                    };
                    let color = get_v3(&lv, "color");

                    lights.push(Light { kind, color });
                }
            }
            "world" => {
                let f_arr = v["fog"].as_array().expect("could not cast into array");
                let fog = (
                    f_arr[0].as_float().expect("could not cast into float") as f32,
                    f_arr[1].as_float().expect("could not cast into float") as f32,
                );
                assert!(fog.1 > fog.0);

                c.world = World {
                    cam_pos: get_v3(&v, "camera_position"),
                    background: get_v3(&v, "background"),
                    fog,
                }
            }
            "render" => {
                let threads;
                if v["threads"].is_integer() {
                    threads = v["threads"]
                        .as_integer()
                        .expect("could not cast into integer")
                        as usize;
                } else if v["threads"].as_str().unwrap() == "auto" {
                    threads = thread::available_parallelism()
                        .expect("could not discover number of threads")
                        .get();
                } else {
                    panic!("unknown thread string value");
                }

                c.render = Render {
                    max_reflections: v["max_reflections"]
                        .as_integer()
                        .expect("could not cast into integer")
                        as u32,
                    threads,
                }
            }
            "output" => {
                let fmt = match v["format"].as_str().expect("could not cast into string") {
                    "png" => Format::Png,
                    "ppm" => Format::Ppm,
                    _ => panic!("unknown image format"),
                };

                let bits = v["bits"].as_integer().expect("could not cast into integer") as usize;

                // only 8 bits supported for now
                if bits != 8 {
                    panic!("invalid bit depth");
                }

                c.output = Output {
                    format: fmt,
                    width: v["width"]
                        .as_integer()
                        .expect("could not cast into integer") as usize,
                    height: v["height"]
                        .as_integer()
                        .expect("could not cast into integer") as usize,
                    bits,
                }
            }
            _ => panic!("unknown key!"),
        }
    }

    Some((c, objs, lights))
}
