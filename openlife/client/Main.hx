function main() {
	new Main();
}

class Main extends hxd.App {
	override function init() {
		super.init();
		Game.s2d = s2d;
		Game.engineHeaps = engine;
		Game.sevents = sevents;
		Game.init();
	}

	override function update(dt:Float) {
		super.update(dt);
		Game.update(dt);
	}
    override function onResize() {
        super.onResize();
        Game.resize();
    }
}
