import hxd.snd.ChannelGroup;
import openlife.engine.Engine;
import sys.FileSystem;

var s2d:h2d.Scene = null;
var engineHeaps:h3d.Engine = null;
var sevents:hxd.SceneEvents = null;
var engine:Engine = null;
var world:World = null;
var soundChannel:ChannelGroup;

function init() {
	soundChannel = new ChannelGroup("master");
	engineHeaps.backgroundColor = 0x2b2932;
	world = new World();
	engine = new Engine(world, null, null, "OneLifeData7/");
	Bake.run();
	Render.genBatch();
	Render.addObject(3, 3, [19]); // 418,575]);
}

function update(dt:Float) {
	Render.update(dt);
}

function resize() {}
