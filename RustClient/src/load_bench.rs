//! Content / asset load timing for headless and graphics clients.
//!
//! Used by `ohol-headless --bench-load` and port workflows that optimize startup.
//! No new dependencies — `std::time::Instant` only.
//!
//! Progress callbacks (P5#36): set `OHOL_LOAD_PROGRESS=1` to print stage lines
//! during prefer_cache steps (same sink as session connect).
//!
//! // C++: LoadingPage progressive banks · Haxe: Resource + ObjectBake + cache

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::anim_bank::AnimBank;
use crate::content::ClientContent;
use crate::content_binary::{
    bake_content, cache_dir_for, load_from_cache, load_prefer_cache, load_prefer_cache_with_progress,
    read_data_version,
};
use crate::ground_sprites::GroundBank;
use crate::load_progress::{
    load_progress_env_enabled, log_progress_line, LoadingState, ProgressCb,
};
use crate::music_bank::MusicBank;
use crate::sound_bank::SoundBank;
use crate::sprite_bank::SpriteBank;

/// Optional env progress sink for bench steps (`OHOL_LOAD_PROGRESS=1`).
fn bench_progress_cb<'a>(
    enabled: bool,
    slot: &'a mut Option<Box<dyn FnMut(&LoadingState)>>,
) -> ProgressCb<'a> {
    if !enabled {
        return None;
    }
    if slot.is_none() {
        *slot = Some(Box::new(|s: &LoadingState| log_progress_line(s)));
    }
    slot.as_mut().map(|b| b.as_mut() as &mut dyn FnMut(&LoadingState))
}

/// Optional atlas-page timing steps used by graphics bench (P4#32 OLGA, P4#40 sprites).
///
/// When pixel-page caches are absent, bench still records **runtime** pack cost so
/// bake-vs-load comparisons have a baseline.

/// One timed step.
#[derive(Debug, Clone)]
pub struct TimedStep {
    pub name: String,
    pub duration: Duration,
    pub detail: String,
}

/// Full load profile (headless content + optional graphics banks).
#[derive(Debug, Clone)]
pub struct LoadProfile {
    pub content_root: PathBuf,
    pub cache_dir: PathBuf,
    pub mode: String,
    pub steps: Vec<TimedStep>,
    pub total: Duration,
    pub object_count: usize,
    pub transition_count: usize,
    pub anim_records: usize,
    pub sprite_pages: usize,
    pub used_binary_cache: bool,
}

impl LoadProfile {
    pub fn report_markdown(&self) -> String {
        let mut s = String::new();
        s.push_str("# Load bench\n\n");
        s.push_str(&format!("- **root:** `{}`\n", self.content_root.display()));
        s.push_str(&format!("- **cache:** `{}`\n", self.cache_dir.display()));
        s.push_str(&format!("- **mode:** {}\n", self.mode));
        s.push_str(&format!(
            "- **binary_cache:** {}\n",
            self.used_binary_cache
        ));
        s.push_str(&format!(
            "- **counts:** objects={} transitions={} anim_records={} sprite_pages={}\n",
            self.object_count, self.transition_count, self.anim_records, self.sprite_pages
        ));
        s.push_str(&format!(
            "- **total:** {:.3}s ({:.1} ms)\n\n",
            self.total.as_secs_f64(),
            self.total.as_secs_f64() * 1000.0
        ));
        s.push_str("| Step | ms | Detail |\n|------|-----|--------|\n");
        for st in &self.steps {
            s.push_str(&format!(
                "| {} | {:.1} | {} |\n",
                st.name,
                st.duration.as_secs_f64() * 1000.0,
                st.detail.replace('|', "/")
            ));
        }
        s
    }

    pub fn report_lines(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "load-bench: mode={} root={} total={:.3}s binary={}\n",
            self.mode,
            self.content_root.display(),
            self.total.as_secs_f64(),
            self.used_binary_cache
        ));
        for st in &self.steps {
            s.push_str(&format!(
                "  {:>10.1} ms  {:<28}  {}\n",
                st.duration.as_secs_f64() * 1000.0,
                st.name,
                st.detail
            ));
        }
        s.push_str(&format!(
            "  objects={} transitions={} anim_records={} sprite_pages={}\n",
            self.object_count, self.transition_count, self.anim_records, self.sprite_pages
        ));
        s
    }
}

fn time_step_result<T>(
    name: &str,
    detail: impl Into<String>,
    f: impl FnOnce() -> T,
) -> (TimedStep, T) {
    let t0 = Instant::now();
    let v = f();
    (
        TimedStep {
            name: name.into(),
            duration: t0.elapsed(),
            detail: detail.into(),
        },
        v,
    )
}

/// Resolve content root: arg, `OHOL_CONTENT_DIR`, then well-known trees.
pub fn resolve_content_root(explicit: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(p) = explicit {
        if p.join("objects").is_dir() {
            return Ok(p.to_path_buf());
        }
        return Err(format!("no objects/ under {}", p.display()));
    }
    if let Ok(e) = std::env::var("OHOL_CONTENT_DIR") {
        let p = PathBuf::from(e);
        if p.join("objects").is_dir() {
            return Ok(p);
        }
    }
    for cand in [
        r"C:\OhOl\OpenLife\openlife\RustServer\content\OneLifeData7",
        r"C:\OhOl\OpenLife\OneLifeData7",
    ] {
        let p = PathBuf::from(cand);
        if p.join("objects").is_dir() {
            return Ok(p);
        }
    }
    Err("no content root (set OHOL_CONTENT_DIR or --src)".into())
}

/// Headless path: time text load vs binary cache vs prefer_cache.
pub fn bench_headless_load(root: &Path, force_text: bool) -> Result<LoadProfile, String> {
    let total0 = Instant::now();
    let cache = cache_dir_for(root);
    let mut steps = Vec::new();
    let mut used_binary = false;
    let progress_on = load_progress_env_enabled();
    let mut progress_slot: Option<Box<dyn FnMut(&LoadingState)>> = None;

    // Stat cache presence (cheap).
    let olc1 = cache.join("olc1_objects.bin");
    let olt1 = cache.join("olt1_transitions.bin");
    let ola1 = cache.join("ola1_anims.bin");
    let olg1 = cache.join("olg1_ground_index.bin");
    let olsn = cache.join("olsn_sounds.bin");
    let man = cache.join("manifest.json");
    steps.push(TimedStep {
        name: "stat_cache".into(),
        duration: Duration::ZERO,
        detail: format!(
            "olc1={} olt1={} ola1={} olg1={} olsn={} manifest={}",
            olc1.exists(),
            olt1.exists(),
            ola1.exists(),
            olg1.exists(),
            olsn.exists(),
            man.exists()
        ),
    });

    if force_text {
        let (st, db) = time_step_result("text_load_objects_trans", "ClientContent::load_from_dir", || {
            ClientContent::load_from_dir(root)
        });
        steps.push(st);
        let db = db?;
        let objects = db.objects.len();
        let transitions = db.transitions.len();

        let (st, bank) = time_step_result("anim_text_lazy_bank", "AnimBank::new (lazy)", || {
            AnimBank::new(root)
        });
        steps.push(st);
        let anim_n = bank.len();

        return Ok(LoadProfile {
            content_root: root.to_path_buf(),
            cache_dir: cache,
            mode: "headless_text".into(),
            steps,
            total: total0.elapsed(),
            object_count: objects,
            transition_count: transitions,
            anim_records: anim_n,
            sprite_pages: 0,
            used_binary_cache: false,
        });
    }

    // Binary-only load when cache exists (timed separately from prefer_cache).
    if olc1.exists() {
        let tree_ver = read_data_version(root);
        let (st, res) = time_step_result(
            "binary_load_olc1_olt1",
            format!("load_from_cache ver={tree_ver:?}"),
            || load_from_cache(&cache, tree_ver),
        );
        steps.push(st);
        match res {
            Ok(db) => {
                used_binary = true;
                if let Some(last) = steps.last_mut() {
                    last.detail = format!(
                        "load_from_cache ver={tree_ver:?} objects={} transitions={}",
                        db.objects.len(),
                        db.transitions.len()
                    );
                }
            }
            Err(e) => {
                steps.push(TimedStep {
                    name: "binary_load_failed".into(),
                    duration: Duration::ZERO,
                    detail: e,
                });
            }
        }
    }

    // prefer_cache path (production headless boot).
    let (st, prefer) = time_step_result("prefer_cache_content", "load_prefer_cache", || {
        load_prefer_cache_with_progress(root, bench_progress_cb(progress_on, &mut progress_slot))
    });
    steps.push(st);
    let db = prefer?;
    let objects = db.objects.len();
    let transitions = db.transitions.len();
    used_binary = used_binary || cache.join("olc1_objects.bin").exists();

    // OLA1 / anim bank.
    let (st, bank) = time_step_result("anim_prefer_cache", "AnimBank::load_prefer_cache", || {
        AnimBank::load_prefer_cache_with_progress(
            root,
            bench_progress_cb(progress_on, &mut progress_slot),
        )
    });
    steps.push(st);
    let anim_n = bank.len();

    // OLSN sound index only — zero AIFF opens at boot (lazy ensure later).
    let (st, sounds) = time_step_result(
        "sound_index_load",
        "SoundBank::load_prefer_cache",
        || {
            SoundBank::load_prefer_cache_with_progress(
                root,
                bench_progress_cb(progress_on, &mut progress_slot),
            )
        },
    );
    steps.push(TimedStep {
        name: st.name,
        duration: st.duration,
        detail: format!(
            "index={} olsn={} aiff_opens={} pcm={}",
            sounds.len(),
            sounds.index_loaded,
            sounds.aiff_opens,
            sounds.pcm_count()
        ),
    });
    debug_assert_eq!(
        sounds.aiff_opens, 0,
        "boot must not open AIFF files for full decode"
    );

    // Music index only — zero OGG/Vorbis decode at boot (P3#24).
    let (st, music) = time_step_result(
        "music_index_load",
        "MusicBank::load_prefer_scan",
        || MusicBank::load_prefer_scan(root),
    );
    steps.push(TimedStep {
        name: st.name,
        duration: st.duration,
        detail: format!(
            "index={} ogg_opens={} pcm={}",
            music.len(),
            music.ogg_opens,
            music.pcm_count()
        ),
    });
    debug_assert_eq!(
        music.ogg_opens, 0,
        "boot must not open music OGG for full decode"
    );

    Ok(LoadProfile {
        content_root: root.to_path_buf(),
        cache_dir: cache,
        mode: "headless_prefer_cache".into(),
        steps,
        total: total0.elapsed(),
        object_count: objects,
        transition_count: transitions,
        anim_records: anim_n,
        sprite_pages: 0,
        used_binary_cache: used_binary,
    })
}

/// Graphics path: content + anim + ground OLG1 + sprite meta preload + sample TGA atlas.
pub fn bench_graphics_load(root: &Path, preload_ids: &[i32]) -> Result<LoadProfile, String> {
    let total0 = Instant::now();
    let cache = cache_dir_for(root);
    let mut steps = Vec::new();

    let (st, content) = time_step_result("prefer_cache_content", "load_prefer_cache", || {
        load_prefer_cache(root)
    });
    steps.push(st);
    let content = content?;
    let used_binary = cache.join("olc1_objects.bin").exists();

    let (st, mut anims) = time_step_result("anim_prefer_cache", "AnimBank::load_prefer_cache", || {
        AnimBank::load_prefer_cache(root)
    });
    steps.push(st);

    // OLG1 index load (binary / scan — no TGA pixels).
    let (st, mut ground) = time_step_result(
        "ground_index_load",
        "GroundBank::load_prefer_cache",
        || GroundBank::load_prefer_cache(root),
    );
    steps.push(TimedStep {
        name: st.name,
        duration: st.duration,
        detail: format!(
            "index={} olg1={} overlays_meta={} biome_tiles={}",
            ground.index_len(),
            ground.index_loaded,
            ground.overlay_count,
            ground.biome_tile_count
        ),
    });

    // OLSN sound index (no AIFF decode at boot).
    let (st, mut sounds) = time_step_result(
        "sound_index_load",
        "SoundBank::load_prefer_cache",
        || SoundBank::load_prefer_cache(root),
    );
    steps.push(TimedStep {
        name: st.name,
        duration: st.duration,
        detail: format!(
            "index={} aiff_opens={} (must be 0 at boot)",
            sounds.len(),
            sounds.aiff_opens
        ),
    });
    // Optional touch: decode one id only if index non-empty (not full naive dump).
    let (st, touch_detail) = time_step_result("sound_ensure_one", "lazy AIFF sample", || {
        let id = sounds.index.keys().next().copied();
        match id {
            Some(i) => {
                let ok = sounds.ensure(i).is_some();
                format!("id={i} ok={ok} aiff_opens={}", sounds.aiff_opens)
            }
            None => "empty index".into(),
        }
    });
    steps.push(TimedStep {
        name: st.name,
        duration: st.duration,
        detail: touch_detail,
    });

    // Editor overlay bank index (OLO1 / scan — no TGA dump at boot).
    let (st, mut overlays) = time_step_result(
        "overlay_index_load",
        "OverlayBank::load_prefer_cache",
        || crate::overlay_bank::OverlayBank::load_prefer_cache(root),
    );
    steps.push(TimedStep {
        name: st.name,
        duration: st.duration,
        detail: format!(
            "count={} olo1={} max_id={}",
            overlays.count(),
            overlays.index_loaded,
            overlays.max_id()
        ),
    });

    // Ground overlay sheets + sample biome TGA (lazy, index-guided — Haxe packs all eagerly).
    let (st, ground_touch) = time_step_result(
        "ground_tga_touch",
        "overlays + sample biome tiles",
        || {
            let n_ov = ground.preload_overlays();
            let mut n_tiles = 0usize;
            for b in 0u8..3 {
                for x in 0..2 {
                    for y in 0..2 {
                        if ground.ensure_tile(b, x, y).is_some() {
                            n_tiles += 1;
                        }
                    }
                }
            }
            // Touch at most one editor overlay TGA if indexed (lazy decode probe).
            let mut n_editor = 0usize;
            if let Some(&id) = overlays.ids().first() {
                if overlays.ensure_image(id).is_some() {
                    n_editor = 1;
                }
            }
            format!(
                "ground_overlays={} tiles={} pages={} editor_tga={}",
                n_ov,
                n_tiles,
                ground.page_count(),
                n_editor
            )
        },
    );
    steps.push(TimedStep {
        name: st.name,
        duration: st.duration,
        detail: ground_touch,
    });

    let ids: Vec<i32> = if preload_ids.is_empty() {
        // Sample first N object sprite ids from content when available.
        let mut v = Vec::new();
        for o in content.objects.values().take(40) {
            for sp in o.sprites.iter().take(2) {
                if sp.sprite_id > 0 && !v.contains(&sp.sprite_id) {
                    v.push(sp.sprite_id);
                }
            }
            if v.len() >= 24 {
                break;
            }
        }
        if v.is_empty() {
            v = vec![19, 33, 144, 1, 2, 3];
        }
        v
    } else {
        preload_ids.to_vec()
    };

    // OLS1 meta only (P4#31) — separate from TGA/pixel pack so we can compare
    // future P4#40 sprite atlas page load against runtime pack cost.
    let (st, mut sprites) = time_step_result(
        "sprite_ols1_meta_load",
        "SpriteBank::load_prefer_cache (meta; no TGA)",
        || SpriteBank::load_prefer_cache(root),
    );
    steps.push(TimedStep {
        name: st.name,
        duration: st.duration,
        detail: format!(
            "meta={} packed_before={} pages_before={} (pixel pages = P4#40 OPEN)",
            sprites.meta_count(),
            sprites.packed_count(),
            sprites.page_count()
        ),
    });

    // Runtime TGA decode + BinPack (baseline until P4#40 disk pages exist).
    let id_note = format!("n={}", ids.len());
    let (st, pages) = time_step_result("sprite_runtime_atlas_pack", id_note, || {
        sprites.preload(ids.iter().copied());
        sprites.page_count()
    });
    steps.push(TimedStep {
        name: st.name,
        duration: st.duration,
        detail: format!(
            "ids={} packed={} pages={} (TGA+BinPack; compare to future sprite_atlas_load)",
            ids.len(),
            sprites.packed_count(),
            pages
        ),
    });
    // Alias kept for older report greps.
    steps.push(TimedStep {
        name: "sprite_preload_atlas".into(),
        duration: st.duration,
        detail: format!("alias of sprite_runtime_atlas_pack; pages={pages}"),
    });

    // Optional ground OLGA pixel pages (P4#32) — measure load when present.
    let olga_path = cache.join("olga_ground_atlas.bin");
    if olga_path.exists() {
        let (st, load_detail) = time_step_result(
            "ground_atlas_load",
            "OLGA pixel pages from cache",
            || {
                match fs::read(&olga_path) {
                    Ok(bytes) => {
                        let mut bank = GroundBank::with_default_roots(Some(root));
                        match bank.load_olga_timed(&bytes) {
                            Ok(stats) => stats.report_line(),
                            Err(e) => format!("load_err={e}"),
                        }
                    }
                    Err(e) => format!("read_err={e}"),
                }
            },
        );
        steps.push(TimedStep {
            name: st.name,
            duration: st.duration,
            detail: load_detail,
        });
    } else {
        steps.push(TimedStep {
            name: "ground_atlas_load".into(),
            duration: Duration::ZERO,
            detail: "skipped (no cache/olga_ground_atlas.bin; bake with --bake-ground-atlas)"
                .into(),
        });
    }

    // Optional sprite OLSA pixel pages (P4#40) — measure load when present.
    let olsa_path = cache.join("olsa_sprite_atlas.bin");
    if olsa_path.exists() {
        let (st, load_detail) = time_step_result(
            "sprite_atlas_load",
            "OLSA pixel pages from cache",
            || match fs::read(&olsa_path) {
                Ok(bytes) => {
                    let mut bank = SpriteBank::new(root);
                    match bank.load_olsa_timed(&bytes) {
                        Ok(stats) => stats.report_line(),
                        Err(e) => format!("load_err={e}"),
                    }
                }
                Err(e) => format!("read_err={e}"),
            },
        );
        steps.push(TimedStep {
            name: st.name,
            duration: st.duration,
            detail: load_detail,
        });
    } else {
        steps.push(TimedStep {
            name: "sprite_atlas_load".into(),
            duration: Duration::ZERO,
            detail: "skipped (no cache/olsa_sprite_atlas.bin; bake with --bake-sprite-atlas)"
                .into(),
        });
    }

    // Touch a few anim samples (forces any remaining lazy work).
    let sample_ids: Vec<i32> = content.objects.keys().copied().take(8).collect();
    let (st, _) = time_step_result(
        "anim_sample_touch",
        format!("objects={}", sample_ids.len()),
        || {
            for id in &sample_ids {
                let _ = anims.get(*id, 0);
                let _ = anims.get(*id, 2);
            }
        },
    );
    steps.push(st);

    // Soft FB alloc (window not required).
    let (st, _) = time_step_result("soft_fb_alloc", "960x540", || {
        let _fb = crate::render::Framebuffer::new(960, 540);
    });
    steps.push(st);

    Ok(LoadProfile {
        content_root: root.to_path_buf(),
        cache_dir: cache,
        mode: "graphics_prefer_cache".into(),
        steps,
        total: total0.elapsed(),
        object_count: content.objects.len(),
        transition_count: content.transitions.len(),
        anim_records: anims.len(),
        sprite_pages: pages,
        used_binary_cache: used_binary,
    })
}

/// Ensure binary cache exists (bake if missing), then run both headless + graphics benches.
pub fn bench_full(
    root: &Path,
    ensure_bake: bool,
    also_text: bool,
) -> Result<Vec<LoadProfile>, String> {
    let cache = cache_dir_for(root);
    let mut out = Vec::new();

    if ensure_bake && !cache.join("olc1_objects.bin").exists() {
        let t0 = Instant::now();
        let res = bake_content(root, &cache)?;
        let total = t0.elapsed();
        let mut steps = vec![TimedStep {
            name: "bake_content".into(),
            duration: total,
            detail: format!(
                "wrote {} total={:.1}ms",
                cache.display(),
                res.timings.total.as_secs_f64() * 1000.0
            ),
        }];
        // Per-phase rows so load-bench reports show bake cost of each blob.
        let push = |steps: &mut Vec<TimedStep>, name: &str, d: Duration, detail: &str| {
            steps.push(TimedStep {
                name: name.into(),
                duration: d,
                detail: detail.into(),
            });
        };
        let t = &res.timings;
        push(&mut steps, "bake_text_load", t.text_load, "ClientContent::load_from_dir");
        push(&mut steps, "bake_dummies", t.dummies, "multi-use+variable");
        push(
            &mut steps,
            "bake_olc1",
            t.olc1,
            &format!("bytes={}", res.olc1_bytes),
        );
        push(
            &mut steps,
            "bake_olt1",
            t.olt1,
            &format!("bytes={}", res.olt1_bytes),
        );
        push(
            &mut steps,
            "bake_ola1",
            t.ola1,
            &format!("bytes={}", res.ola1_bytes),
        );
        push(
            &mut steps,
            "bake_olg1",
            t.olg1,
            &format!("index only; bytes={}", res.olg1_bytes),
        );
        push(
            &mut steps,
            "bake_olo1",
            t.olo1,
            &format!("bytes={}", res.olo1_bytes),
        );
        push(
            &mut steps,
            "bake_olsn",
            t.olsn,
            &format!("bytes={}", res.olsn_bytes),
        );
        push(
            &mut steps,
            "bake_ols1_meta",
            t.ols1,
            &format!(
                "meta only; bytes={} count={} (pixel pages = P4#40)",
                res.ols1_bytes, res.sprite_count
            ),
        );
        push(
            &mut steps,
            "bake_write_blobs",
            t.write_blobs,
            "disk write + manifest",
        );
        out.push(LoadProfile {
            content_root: root.to_path_buf(),
            cache_dir: cache.clone(),
            mode: "bake_once".into(),
            steps,
            total,
            object_count: res.object_count,
            transition_count: res.transition_count,
            anim_records: res.anim_count,
            sprite_pages: 0,
            used_binary_cache: true,
        });
    }

    if also_text {
        out.push(bench_headless_load(root, true)?);
    }
    out.push(bench_headless_load(root, false)?);
    out.push(bench_graphics_load(root, &[])?);
    Ok(out)
}

/// Write profiles to a markdown report path.
pub fn write_report(profiles: &[LoadProfile], path: impl AsRef<Path>) -> Result<(), String> {
    let mut md = String::from("# Client load bench report\n\n");
    md.push_str(&format!(
        "Generated for rust-client load optimization.\n\n"
    ));
    for p in profiles {
        md.push_str(&p.report_markdown());
        md.push_str("\n");
    }
    if let Some(parent) = path.as_ref().parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(path.as_ref(), md).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn bench_tiny_fixture() {
        let tmp = std::env::temp_dir().join(format!(
            "ohol_load_bench_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("objects")).unwrap();
        fs::create_dir_all(tmp.join("transitions")).unwrap();
        fs::create_dir_all(tmp.join("animations")).unwrap();
        fs::create_dir_all(tmp.join("sprites")).unwrap();
        fs::write(tmp.join("dataVersionNumber.txt"), "1\n").unwrap();
        fs::write(tmp.join("nextObjectNumber.txt"), "10\n").unwrap();
        // Minimal object
        let mut f = fs::File::create(tmp.join("objects").join("1.txt")).unwrap();
        writeln!(f, "id=1").unwrap();
        writeln!(f, "name=Test").unwrap();
        writeln!(f, "containable=0").unwrap();
        writeln!(f, "permanent=0").unwrap();
        writeln!(f, "minPickupAge=0").unwrap();
        writeln!(f, "heldInHand=0").unwrap();
        writeln!(f, "blocksWalking=0").unwrap();
        writeln!(f, "numSprites=0").unwrap();
        drop(f);

        let p = bench_headless_load(&tmp, true).expect("text load");
        assert_eq!(p.mode, "headless_text");
        assert!(p.steps.iter().any(|s| s.name == "text_load_objects_trans"));
        let _ = fs::remove_dir_all(&tmp);
    }
}
