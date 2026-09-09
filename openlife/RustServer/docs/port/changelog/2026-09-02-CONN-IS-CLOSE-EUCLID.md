# 2026-09-02 CONN-IS-CLOSE-EUCLID

**status:** **DONE**

## Closed

- Connection PU/MX/PM fans (`nearby_conn_ids`) now match Haxe `GlobalPlayerInstance.isClose` / `AiHelper.CalculateDistance`: torus squared-Euclidean `quadDist <= range²`, not Chebyshev.
- Wrap uses world `width/height` + `wrap` (Haxe round-map TODO is live when wrap is on).
- `range <= 0` still means all connected (`is_close_pu_wrap`).

## Tests

- `nearby_conn_ids_haxe_is_close_euclidean` — corner (4,4) out of range 5, in range 6
- `nearby_conn_ids_torus_wrap_is_close` — (0,0)↔(511,0) on 512 wrap
- `pm_fan_uses_live_cose_for_movement` still ok
