use ohol_headless::content::ClientContent;
use ohol_headless::sprite_bank::SpriteBank;
use ohol_headless::anim_bank::AnimBank;
use ohol_headless::live_object::LiveWorld;
use ohol_headless::client_map::ClientMap;
use ohol_headless::render::{SceneRenderer, Framebuffer, CLEAR_RGBA, ZOOM_DEFAULT};
use ohol_headless::parse::parse_pu_line;

#[test]
fn live_path_olc1_person_draws() {
    let roots = [
        r"C:\OhOl\OpenLife\openlife\RustServer\content\OneLifeData7",
        r"C:\OhOl\OpenLife\OneLifeData7",
    ];
    let root = roots.iter().map(std::path::Path::new).find(|p| p.join("objects").is_dir()).expect("content root");
    let content = ClientContent::load_prefer_cache(root).expect("content");
    let def = content.get(19).expect("19");
    println!("root={} person={} sprites={}", root.display(), def.person, def.sprites.len());
    let mut sprites = SpriteBank::load_prefer_cache(root);
    let mut anims = AnimBank::load_prefer_cache(root);
    let mut ok = 0usize;
    for s in &def.sprites {
        if sprites.ensure(s.sprite_id).is_some() { ok += 1; }
    }
    println!("ensured={ok}/{}", def.sprites.len());
    assert!(def.sprites.len() > 50);
    assert!(ok > 40);

    let mut world = LiveWorld::new();
    // age ~14, hat, held object 33
    let pu = parse_pu_line("1 19 1 0 0 0 33 0 0 0 -1 0.5 1 0 0 0 14.0 60.0 3.75 1117;0;0;0;0;0 0 0 -1 0 0").unwrap();
    world.apply_pu(&pu);
    world.set_our_id(1);
    let mut map = ClientMap::new();
    let mut scene = SceneRenderer::default();
    scene.camera.zoom = ZOOM_DEFAULT;
    let mut fb = Framebuffer::new(960, 540);
    scene.draw(&mut fb, &mut map, &mut world, &content, &mut sprites, &mut anims, 1.0/60.0);
    let non = fb.count_non_color(CLEAR_RGBA);
    println!("non_clear={non}");
    assert!(non > 200);
}
