use std::time::Instant;
#[test]
fn timed_prefer_cache_onelife() {
    let root = std::path::Path::new(r"C:\OhOl\OpenLife\openlife\RustServer\content\OneLifeData7");
    if !root.join("objects").is_dir() { return; }
    let t0 = Instant::now();
    let mut events = Vec::new();
    let mut cb = |s: &ohol_headless::load_progress::LoadingState| {
        println!("progress {:.0}% {}", s.overall_fraction*100.0, s.label);
        events.push(s.label.clone());
    };
    let c = ohol_headless::content::ClientContent::load_prefer_cache_with_progress(root, Some(&mut cb)).unwrap();
    println!("loaded objects={} in {:.3}s", c.objects.len(), t0.elapsed().as_secs_f64());
    assert!(c.objects.len() > 100);
    assert!(events.iter().any(|e| e.contains("cache")), "should use cache not rebake: {:?}", events);
    assert!(!events.iter().any(|e| e.contains("rebake")), "must not rebake: {:?}", events);
}
