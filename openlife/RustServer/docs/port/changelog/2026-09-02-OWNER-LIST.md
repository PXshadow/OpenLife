# 2026-09-02 OWNER-LIST

**status:** **DONE**

Haxe `Connection.sendOwners`: client `OWNER x y` → `OW`.

If `livingOwners` empty: silent unless `ObjectData.isOwned` (`+owned` / `+tempOwned` / `+followerOwned`), then `addOwner(querier)`. Else list living owner ids.

Tests: `owner_query_sends_owner_list` / `owner_query_claims_unowned_owned_object`.

`cargo check -p ol-server --offline` ok.
