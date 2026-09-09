/// Helpers for 1:1 Haxe `PatchObjectData` / `PatchTransitions` transcription.

#[allow(dead_code)]
fn obj_mut(db: &mut ContentDb, id: i32) -> Option<&mut ObjectDef> {
    db.objects.get_mut(&id)
}

#[allow(dead_code)]
fn trans_mut(db: &mut ContentDb, actor: i32, target: i32) -> Option<&mut Transition> {
    if let Some(t) = db.transitions.get_mut(&(actor, target)) {
        return Some(t);
    }
    db.transitions_last_use.get_mut(&(actor, target))
}

#[allow(dead_code)]
fn trans_mut_any(
    db: &mut ContentDb,
    actor: i32,
    target: i32,
    last_use_actor: bool,
    last_use_target: bool,
) -> Option<&mut Transition> {
    if last_use_actor || last_use_target {
        db.transitions_last_use.get_mut(&(actor, target))
    } else {
        db.transitions.get_mut(&(actor, target))
    }
}

#[allow(dead_code)]
fn upsert_transition(db: &mut ContentDb, t: Transition) {
    let key = (t.actor_id, t.target_id);
    let last = t.last_use_actor || t.last_use_target;
    if t.auto_decay_seconds != 0.0 && t.actor_id < 0 {
        db.auto_decays.insert(t.target_id, t.clone());
    }
    if last {
        if db.transitions_last_use.insert(key, t).is_none() {
            db.last_use_transition_count = db.last_use_transition_count.saturating_add(1);
        }
    } else if db.transitions.insert(key, t).is_none() {
        db.transition_count = db.transition_count.saturating_add(1);
    }
}

#[allow(dead_code)]
fn new_trans(actor: i32, target: i32, new_actor: i32, new_target: i32) -> Transition {
    Transition {
        actor_id: actor,
        target_id: target,
        new_actor_id: new_actor,
        new_target_id: new_target,
        ..Default::default()
    }
}

#[allow(dead_code)]
fn set_auto_decay(db: &mut ContentDb, actor: i32, target: i32, seconds: f32) {
    if let Some(t) = trans_mut(db, actor, target) {
        t.auto_decay_seconds = seconds;
        if actor < 0 {
            let copy = t.clone();
            db.auto_decays.insert(target, copy);
        }
    }
}

#[allow(dead_code)]
fn set_new_actor(db: &mut ContentDb, actor: i32, target: i32, new_actor: i32) {
    if let Some(t) = trans_mut(db, actor, target) {
        t.new_actor_id = new_actor;
    }
}

#[allow(dead_code)]
fn set_new_target(db: &mut ContentDb, actor: i32, target: i32, new_target: i32) {
    if let Some(t) = trans_mut(db, actor, target) {
        t.new_target_id = new_target;
    }
}

/// Haxe `ServerSettings.WoolClothDecayTime` (`-24 * 30`).
const WOOL_CLOTH_DECAY_TIME: f32 = -24.0 * 30.0;
/// Haxe `ServerSettings.RabbitFurClothDecayTime` (`-24 * 2`).
const RABBIT_FUR_CLOTH_DECAY_TIME: f32 = -24.0 * 2.0;
/// Haxe `ServerSettings.WoundHealingTimeFactor`.
#[allow(dead_code)]
const WOUND_HEALING_TIME_FACTOR: f32 = 2.0;
/// Haxe `ServerSettings.SemiHeavyItemSpeed`.
const SEMI_HEAVY_ITEM_SPEED: f32 = 0.9;
/// Haxe `ServerSettings.ObjDecayFactorOnFloor`.
#[allow(dead_code)]
const OBJ_DECAY_FACTOR_ON_FLOOR: f32 = 0.2;
/// Haxe `ServerSettings.HungryWorkCost` (object-level `+hungryWork` / id table).
const HUNGRY_WORK_COST: f32 = 5.0;

/// Haxe `BiomeTag.GREEN`.
const BIOME_TAG_GREEN: i32 = 0;
/// Haxe `BiomeTag.YELLOW`.
const BIOME_TAG_YELLOW: i32 = 2;
/// Haxe `BiomeTag.GREY`.
const BIOME_TAG_GREY: i32 = 3;
/// Haxe `BiomeTag.SNOW`.
const BIOME_TAG_SNOW: i32 = 4;

/// Haxe `ObjectData.parentId` (dummy → base).
fn parent_id(db: &ContentDb, id: i32) -> i32 {
    db.resolve_base_id(id)
}

/// Haxe `TransitionImporter.addTransition` last-use / reverseUse clone rules.
fn add_transition_haxe(
    db: &mut ContentDb,
    mut t: Transition,
    last_use_actor: bool,
    last_use_target: bool,
) {
    if !last_use_actor && !last_use_target {
        if !t.last_use_actor && t.reverse_use_actor && !t.last_use_target && t.reverse_use_target {
            add_transition_haxe(db, t.clone(), true, true);
        } else if !t.last_use_actor && t.reverse_use_actor {
            add_transition_haxe(db, t.clone(), true, t.last_use_target);
        } else if !t.last_use_target && t.reverse_use_target {
            add_transition_haxe(db, t.clone(), t.last_use_actor, true);
        }
    } else {
        if last_use_actor {
            t.last_use_actor = true;
        }
        if last_use_target {
            t.last_use_target = true;
        }
    }
    upsert_transition(db, t);
}

fn set_auto_decay_or_stored(db: &mut ContentDb, actor: i32, target: i32, seconds: f32) {
    if let Some(t) = trans_mut(db, actor, target) {
        t.auto_decay_seconds = seconds;
        if actor < 0 {
            let copy = t.clone();
            db.auto_decays.insert(target, copy);
        }
        return;
    }
    if actor < 0 {
        if let Some(t) = db.auto_decays.get_mut(&target) {
            t.auto_decay_seconds = seconds;
        }
    }
}

fn biome_push(obj: &mut ObjectDef, biome: i32) {
    if !obj.biomes.contains(&biome) {
        obj.biomes.push(biome);
    }
}

fn mark_ai_ignore(db: &mut ContentDb, actor: i32, target: i32) {
    db.ai_should_ignore.insert((actor, target));
}

fn for_each_trans_mut(db: &mut ContentDb, mut f: impl FnMut(&mut Transition)) {
    for t in db.transitions.values_mut() {
        f(t);
    }
    for t in db.transitions_last_use.values_mut() {
        f(t);
    }
    for t in db.transitions_max_use.values_mut() {
        f(t);
    }
    for t in db.auto_decays.values_mut() {
        f(t);
    }
}
