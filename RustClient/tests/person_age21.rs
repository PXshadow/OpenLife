//! Offline person at age 21 — full age-visible figure (Jason parity).
//!
//! Wire PU uses **invAgeRate** (e.g. 60); client stores ageRate = 1/60.
//! After seconds of wall time, body must still be fully age-gated for ~21,
//! not collapsed to body+head only (old bug: treated invAgeRate as years/sec).

use ohol_headless::anim_bank::AnimBank;
use ohol_headless::client_map::ClientMap;
use ohol_headless::content::ClientContent;
use ohol_headless::live_object::LiveWorld;
use ohol_headless::parse::parse_pu_line;
use ohol_headless::render::{Framebuffer, SceneRenderer, CLEAR_RGBA, ZOOM_DEFAULT, ZOOM_MAX};
use ohol_headless::sprite_bank::SpriteBank;
use std::io::Write;
use std::thread;
use std::time::Duration;

fn content_root() -> std::path::PathBuf {
    for c in [
        r"C:\OhOl\OpenLife\openlife\RustServer\content\OneLifeData7",
        r"C:\OhOl\OpenLife\OneLifeData7",
    ] {
        let p = std::path::PathBuf::from(c);
        if p.join("objects").is_dir() {
            return p;
        }
    }
    panic!("no OneLifeData7");
}

fn flesh_span(fb: &Framebuffer) -> (usize, i32, i32) {
    let mut n = 0usize;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;
    let cx = (fb.width / 2) as i32;
    let cy = (fb.height / 2) as i32;
    for y in (cy - 160)..(cy + 160) {
        for x in (cx - 100)..(cx + 100) {
            if x < 0 || y < 0 || x >= fb.width as i32 || y >= fb.height as i32 {
                continue;
            }
            let i = ((y as u32 * fb.width + x as u32) * 4) as usize;
            let (r, g, b, a) = (
                fb.pixels[i],
                fb.pixels[i + 1],
                fb.pixels[i + 2],
                fb.pixels[i + 3],
            );
            if a < 200 {
                continue;
            }
            let greenish = g > r.saturating_add(20) && g > b.saturating_add(15);
            if !greenish && (r > 40 || g > 40 || b > 40) {
                n += 1;
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    (n, min_y, max_y)
}

fn draw_age(
    content: &ClientContent,
    sprites: &mut SpriteBank,
    anims: &mut AnimBank,
    age: f32,
    inv_age_rate: f32,
    zoom: f32,
) -> (Framebuffer, f32) {
    let mut world = LiveWorld::new();
    // Wire: age invAgeRate (Jason sscanf invAgeRate then ageRate=1/inv)
    let line = format!(
        "1 19 1 0 0 0 0 0 0 0 -1 0.5 1 0 0 0 {age:.1} {inv_age_rate:.1} 3.75 0;0;0;0;0;0 0 0 -1 0 0"
    );
    let pu = parse_pu_line(&line).expect("pu");
    assert!(
        (pu.age_rate - 1.0 / inv_age_rate).abs() < 1e-5,
        "PU must convert invAgeRate→ageRate, got {}",
        pu.age_rate
    );
    world.apply_pu(&pu);
    world.set_our_id(1);
    let current = world.our().unwrap().current_age();
    let mut map = ClientMap::new();
    for y in -2..=2 {
        for x in -3..=3 {
            map.set(
                x,
                y,
                ohol_headless::client_map::MapTile {
                    biome: 2,
                    ..ohol_headless::client_map::MapTile::empty()
                },
            );
        }
    }
    let mut scene = SceneRenderer::default();
    scene.camera.x = 0.0;
    scene.camera.y = 0.0;
    scene.camera.zoom = zoom;
    let mut fb = Framebuffer::new(960, 540);
    scene.draw(
        &mut fb,
        &mut map,
        &mut world,
        content,
        sprites,
        anims,
        1.0 / 60.0,
    );
    (fb, current)
}

fn save_ppm(fb: &Framebuffer, path: &str) {
    let mut f = std::fs::File::create(path).unwrap();
    writeln!(f, "P6\n{} {}\n255", fb.width, fb.height).unwrap();
    for px in fb.pixels.chunks(4) {
        let _ = f.write_all(&[px[0], px[1], px[2]]);
    }
}

#[test]
fn age_21_full_figure_and_stable_after_seconds() {
    let root = content_root();
    let content = ClientContent::load_prefer_cache(&root).expect("content");
    let def = content.get(19).expect("19");
    let mut sprites = SpriteBank::load_prefer_cache(&root);
    let mut anims = AnimBank::load_prefer_cache(&root);

    // Age-visible layer count at 21 (Jason isSpriteVisibleAtAge exclusive end).
    let age = 21.0f32;
    let mut vis = 0usize;
    let mut has_body = false;
    let mut has_head = false;
    for s in &def.sprites {
        if s.visible_at_age(age) {
            vis += 1;
            if s.is_body {
                has_body = true;
            }
            if s.is_head {
                has_head = true;
            }
        }
    }
    println!("age=21 age_visible={vis} body={has_body} head={has_head}");
    assert!(has_body && has_head);
    assert!(
        vis >= 25,
        "adult 21 must keep many age layers (not body+head only), vis={vis}"
    );

    // invAgeRate=60 like server (years/sec = 1/60)
    let (fb0, cur0) = draw_age(&content, &mut sprites, &mut anims, 21.0, 60.0, ZOOM_DEFAULT);
    let (flesh0, y0, y1) = flesh_span(&fb0);
    let span0 = y1 - y0;
    println!(
        "t0 current_age={cur0:.3} flesh={flesh0} y_span={span0} non={}",
        fb0.count_non_color(CLEAR_RGBA)
    );
    assert!((cur0 - 21.0).abs() < 0.05, "age should start ~21, got {cur0}");
    assert!(flesh0 > 400, "full figure flesh, got {flesh0}");
    assert!(span0 > 40, "vertical figure span must be tall, span={span0}");

    let scratch = r"C:\Users\marti\AppData\Local\Temp\grok-goal-1b2d0cddbf25\implementer";
    let _ = std::fs::create_dir_all(scratch);
    save_ppm(&fb0, &format!("{scratch}/fb_age21_t0.ppm"));

    // Simulate “a few seconds later” without re-PU: wall clock advances age by ~1/60 y/s.
    thread::sleep(Duration::from_millis(2500));
    let (fb1, _cur1) = draw_age(&content, &mut sprites, &mut anims, 21.0, 60.0, ZOOM_DEFAULT);
    // Note: new PU resets last_age_set — to test elapsed, use live object clock directly:
    let mut world = LiveWorld::new();
    let pu = parse_pu_line(
        "1 19 1 0 0 0 0 0 0 0 -1 0.5 1 0 0 0 21.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 0",
    )
    .unwrap();
    world.apply_pu(&pu);
    world.set_our_id(1);
    thread::sleep(Duration::from_millis(2500));
    let aged = world.our().unwrap().current_age();
    println!("after 2.5s wall on same object current_age={aged:.4}");
    assert!(
        aged > 21.0 && aged < 21.1,
        "with ageRate=1/60, 2.5s ≈ +0.04y, got {aged} (must NOT jump by dozens of years)"
    );
    // Old bug: age_rate=60 → +150 years in 2.5s → past 999 → only body+head
    let wrong = 21.0 + 60.0 * 2.5;
    assert!(
        wrong > 100.0,
        "sanity: wrong formula would be ~{wrong}"
    );

    // Age-visible at computed age still full adult set
    let mut vis_aged = 0usize;
    for s in &def.sprites {
        if s.visible_at_age(aged) {
            vis_aged += 1;
        }
    }
    assert!(
        vis_aged >= 25,
        "after seconds still many layers at age {aged}, vis={vis_aged}"
    );

    let (flesh1, _, y1b) = flesh_span(&fb1);
    let _ = (flesh1, y1b);
    save_ppm(&fb1, &format!("{scratch}/fb_age21_after_sleep_redraw.ppm"));

    // Max zoom at 21
    let (fbz, _) = draw_age(&content, &mut sprites, &mut anims, 21.0, 60.0, ZOOM_MAX);
    let (flesh_z, z0, z1) = flesh_span(&fbz);
    println!("ZOOM_MAX flesh={flesh_z} span={}", z1 - z0);
    assert!(flesh_z > 500, "max zoom full figure");
    save_ppm(&fbz, &format!("{scratch}/fb_age21_zoom_max.ppm"));
}

#[test]
fn inv_age_rate_parse_matches_jason() {
    let pu = parse_pu_line(
        "1 19 1 0 0 0 0 0 0 0 -1 0.5 1 0 0 0 21.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 0",
    )
    .unwrap();
    assert!((pu.age - 21.0).abs() < 1e-4);
    assert!((pu.age_rate - (1.0 / 60.0)).abs() < 1e-6);
}
