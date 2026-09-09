# 2026-09-03 BASKET-PILE-CMD

**status:** **DONE**

Haxe `doCommandHelper` USE Basket 292 + Basket 292 / Stack of Baskets 1605 when either side has cargo: refuse (TODO hidden containers is still not implemented; the refuse is live in Haxe). Empty 292+292 still applies.

Tests: `basket_pile_refuse_pure` / `use_basket_with_cargo_on_basket_refuses` / `use_empty_basket_on_empty_basket_applies` / `use_empty_basket_on_stack_with_cargo_refuses`.

`cargo check -p ol-server --offline` ok.
