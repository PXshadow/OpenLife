//! Load versioned world from disk, or generate from PNG + natural spawn (Haxe WorldMap).

use ol_config::ServerConfig;
use ol_content::ContentDb;
use ol_world::{
    generate_from_png, load_world_file, map_generation_seed, populate_fresh_map, save_world_file,
    GenerateOptions, World,
};
use rand::SeedableRng;
use tracing::{info, warn};

/// Resolve map PNG: config path, or sibling OpenLife fallbacks.
fn resolve_map_png(cfg: &ServerConfig) -> Option<std::path::PathBuf> {
    let p = &cfg.map_png_path;
    if p.exists() {
        return Some(p.clone());
    }
    let candidates = [
        std::path::PathBuf::from("maps/mysteraV1Test.png"),
        std::path::PathBuf::from("../OpenLife/mysteraV1Test.png"),
        std::path::PathBuf::from(r"C:\OhOl\OpenLife\mysteraV1Test.png"),
    ];
    for c in candidates {
        if c.exists() {
            return Some(c);
        }
    }
    None
}

/// Bootstrap world: load save if present, else PNG → biomes → natural objects → save.
pub fn bootstrap_world(cfg: &ServerConfig, content: &ContentDb) -> World {
    let save_path = cfg.world_save_path();

    if !cfg.force_regenerate_map && save_path.exists() {
        match load_world_file(&save_path) {
            Ok(w) => {
                info!(
                    path = %save_path.display(),
                    w = w.width_tiles,
                    h = w.height_tiles,
                    chunks = w.resident_chunk_count(),
                    helpers = w.helper_count(),
                    "loaded world from disk"
                );
                ensure_generation_census(cfg, content, w.width_tiles, w.height_tiles);
                return w;
            }
            Err(e) => {
                warn!(error = %e, path = %save_path.display(), "world load failed; will generate");
            }
        }
    }

    if !cfg.generate_map_if_missing && !cfg.force_regenerate_map {
        info!("map generate disabled and no save — empty 512×512 world");
        return World::new(512, 512, true);
    }

    let Some(png) = resolve_map_png(cfg) else {
        warn!(
            configured = %cfg.map_png_path.display(),
            "map PNG not found — empty 512×512 world"
        );
        return World::new(512, 512, true);
    };

    let opts = GenerateOptions {
        wrap: true,
        density: cfg.natural_object_density,
    };

    let mut world = match generate_from_png(&png, &opts) {
        Ok(w) => w,
        Err(e) => {
            warn!(error = %e, path = %png.display(), "PNG generate failed — empty world");
            return World::new(512, 512, true);
        }
    };

    // Haxe: addExtraBiomes → generateObjects → generateExtraStuff, then originalObjects copy.
    let seed = map_generation_seed(world.width_tiles, world.height_tiles);
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let n = populate_fresh_map(&mut world, content, cfg.natural_object_density, &mut rng);
    info!(
        objects = n,
        density = cfg.natural_object_density,
        "initial natural objects placed"
    );
    save_generation_census(cfg, content, &world);

    match save_world_file(&world, &save_path) {
        Ok(()) => info!(path = %save_path.display(), "world permanently saved to disk"),
        Err(e) => warn!(error = %e, path = %save_path.display(), "world save failed"),
    }

    world
}

/// Rebuild original census from PNG generate when the on-disk file is missing or a live freeze.
fn ensure_generation_census(cfg: &ServerConfig, content: &ContentDb, width: i32, height: i32) {
    let path = cfg.original_census_save_path();
    if ol_sim::original_census_is_generation(&path) {
        return;
    }
    let Some(png) = resolve_map_png(cfg) else {
        warn!("cannot rebuild original census — map PNG missing");
        return;
    };
    let opts = GenerateOptions {
        wrap: true,
        density: cfg.natural_object_density,
    };
    let mut scratch = match generate_from_png(&png, &opts) {
        Ok(w) => w,
        Err(e) => {
            warn!(error = %e, "original census PNG generate failed");
            return;
        }
    };
    if scratch.width_tiles != width || scratch.height_tiles != height {
        warn!(
            png_w = scratch.width_tiles,
            png_h = scratch.height_tiles,
            live_w = width,
            live_h = height,
            "PNG size differs from live world; still using PNG for original census"
        );
    }
    let seed = map_generation_seed(scratch.width_tiles, scratch.height_tiles);
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let n = populate_fresh_map(
        &mut scratch,
        content,
        cfg.natural_object_density,
        &mut rng,
    );
    info!(objects = n, "rebuilt original object census from fresh-map generate");
    save_generation_census(cfg, content, &scratch);
}

fn save_generation_census(cfg: &ServerConfig, content: &ContentDb, world: &World) {
    let path = cfg.original_census_save_path();
    let mut lt = ol_sim::LongTermState::default();
    if let Err(e) = ol_sim::capture_and_save_original_census(&mut lt, world, content, &path) {
        warn!(error = %e, path = %path.display(), "original census save failed");
    } else {
        info!(
            path = %path.display(),
            originals = lt.original_counts.len(),
            "froze original object census from fresh map"
        );
    }
}

/// Periodic dirty-world save (best-effort).
pub fn save_world_if_present(cfg: &ServerConfig, world: &World) {
    let path = cfg.world_save_path();
    if let Err(e) = save_world_file(world, &path) {
        warn!(error = %e, "periodic world save failed");
    }
}
