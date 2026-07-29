//! Integration: shipped person draw on real OneLife content (Jason parity).
//!
//! Drives `SceneRenderer::draw` — not a reimplementation of z-order / pose.

use ohol_headless::anim_bank::AnimBank;
use ohol_headless::client_map::ClientMap;
use ohol_headless::content::ClientContent;
use ohol_headless::live_object::LiveWorld;
use ohol_headless::parse::parse_pu_line;
use ohol_headless::render::{
    Framebuffer, SceneRenderer, CLEAR_RGBA, ZOOM_DEFAULT, ZOOM_MAX,
};
use ohol_headless::sprite_bank::SpriteBank;

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
    panic!("no OneLifeData7 content root");
}

/// Count age-visible person layers that ensure a TGA (Jason would draw these).
fn age_visible_ensured(
    def: &ohol_headless::content::ClientObjectDef,
    sprites: &mut SpriteBank,
    age: f32,
) -> (usize, usize) {
    let mut vis = 0usize;
    let mut ok = 0usize;
    for s in &def.sprites {
        if !s.visible_at_age(age) {
            continue;
        }
        vis += 1;
        if sprites.ensure(s.sprite_id).is_some() {
            ok += 1;
        }
    }
    (vis, ok)
}

/// Non-biome flesh/cloth pixels in a window around figure center.
fn flesh_near_center(fb: &Framebuffer, half_w: i32, half_h: i32) -> usize {
    let cx = (fb.width / 2) as i32;
    let cy = (fb.height / 2) as i32;
    let mut n = 0usize;
    for y in (cy - half_h)..(cy + half_h) {
        for x in (cx - half_w)..(cx + half_w) {
            if x < 0 || y < 0 || x >= fb.width as i32 || y >= fb.height as i32 {
                continue;
            }
            let i = ((y as u32 * fb.width + x as u32) * 4) as usize;
            let r = fb.pixels[i];
            let g = fb.pixels[i + 1];
            let b = fb.pixels[i + 2];
            let a = fb.pixels[i + 3];
            if a < 200 {
                continue;
            }
            // BodyWhite ~grey 155 or skin/cloth — not pure biome greens
            let greenish = g > r.saturating_add(20) && g > b.saturating_add(15);
            if !greenish && (r > 40 || g > 40 || b > 40) {
                n += 1;
            }
        }
    }
    n
}

fn draw_person(
    content: &ClientContent,
    sprites: &mut SpriteBank,
    anims: &mut AnimBank,
    zoom: f32,
    age: f32,
    clothing: &str,
    held: i32,
) -> Framebuffer {
    let mut world = LiveWorld::new();
    // display_id=19, clothing set, held, age, pos 0,0
    let line = format!(
        "1 19 1 0 0 0 {held} 0 0 0 -1 0.5 1 0 0 0 {age:.1} 60.0 3.75 {clothing} 0 0 -1 0 0"
    );
    let pu = parse_pu_line(&line).expect("pu");
    world.apply_pu(&pu);
    world.set_our_id(1);
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
    fb
}

#[test]
fn live_path_full_body_clothing_held() {
    let root = content_root();
    let content = ClientContent::load_prefer_cache(&root).expect("content");
    let def = content.get(19).expect("person 19");
    assert!(def.person != 0);
    assert!(def.sprites.len() > 50, "person must have full sprite list");

    let mut sprites = SpriteBank::load_prefer_cache(&root);
    let mut anims = AnimBank::load_prefer_cache(&root);
    let age = 20.0;
    let (vis, ok) = age_visible_ensured(def, &mut sprites, age);
    println!(
        "root={} sprites={} age_visible={vis} ensured={ok}",
        root.display(),
        def.sprites.len()
    );
    assert!(vis > 20, "adult must show many age-visible layers, got {vis}");
    assert!(
        ok * 10 >= vis * 8,
        "most age-visible layers must load TGA ({ok}/{vis})"
    );

    // Hat 1117 + held 33 at adult age
    let fb = draw_person(
        &content,
        &mut sprites,
        &mut anims,
        ZOOM_DEFAULT,
        age,
        "1117;0;0;0;0;0",
        33,
    );
    let non = fb.count_non_color(CLEAR_RGBA);
    let flesh = flesh_near_center(&fb, 80, 100);
    println!("default_zoom non_clear={non} flesh_center={flesh}");
    assert!(non > 500, "must paint world+person");
    assert!(
        flesh > 120,
        "full body/skin near center expected (not hat-only), flesh={flesh}"
    );
}

#[test]
fn live_path_max_zoom_full_figure() {
    let root = content_root();
    let content = ClientContent::load_prefer_cache(&root).expect("content");
    let mut sprites = SpriteBank::load_prefer_cache(&root);
    let mut anims = AnimBank::load_prefer_cache(&root);
    let fb = draw_person(
        &content,
        &mut sprites,
        &mut anims,
        ZOOM_MAX,
        20.0,
        "1117;0;0;0;0;0",
        0,
    );
    let flesh = flesh_near_center(&fb, 120, 140);
    println!("zoom_max={} flesh_center={flesh}", ZOOM_MAX);
    assert!(
        flesh > 200,
        "ZOOM_MAX must still paint full figure, flesh={flesh}"
    );
}


