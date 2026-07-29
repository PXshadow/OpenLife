// FEED-OTHER-YUM live helpers — reference mirror of lib.rs inline (not included;
// lib already defines apply_eat_health_prestige / feed_other_full_eat).
// Haxe: GlobalPlayerInstance.doEating playerFrom != playerTo L3041–3247
//
// Live path features (lib.rs):
// - feeder MinAgeToEat + AllowEatingOrFeedingIfIll / yellow fever
// - can_feed_to_me_obj_ex_yum + compute_eat_full × world×starving
// - craving dontChange, yum/meh prestige, feeder 0.2 share
// - isDrugs fever resistance (yellowfeverCount + fever.timeToChange)
// - DoChangeNumberOfUsesOnActorManual multi-use on feeder held
// - FeedOtherEatPost for responsible_id + post-eat emotes
