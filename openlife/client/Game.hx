import h2d.Graphics;
import hxd.snd.ChannelGroup;
import openlife.data.object.ObjectData;
import openlife.engine.Engine;
import openlife.engine.Utility;
import sys.FileSystem;

var s2d:h2d.Scene = null;
var engineHeaps:h3d.Engine = null;
var sevents:hxd.SceneEvents = null;
var engine:Engine = null;
var world:World = null;
var soundChannel:ChannelGroup;
var g:Graphics;

function init() {
	Engine.dir = Utility.dir();
	if (!FileSystem.exists(ClientSettings.SaveDirectory)) FileSystem.createDirectory(ClientSettings.SaveDirectory);
	soundChannel = new ChannelGroup("master");
	engineHeaps.backgroundColor = 0x2b2932;
	world = new World();
	engine = new Engine(world, null, null, "OneLifeData7/");
	ObjectData.DoAllTheObjectInititalisationStuff(false);
	Render.loadGround(); // seralized
	Render.loadSprites(); // serialized
	g = new Graphics(s2d);
	// Render.addObject(3, 3, [19]);
	// Render.addObject(3, 4, [575]);
	/*for (x in 0...7) {
		for (y in 0...7) {
			Render.addGround(2, x, y);
		}
	}*/

	new Fps(150, s2d);
	engine.client.config = {ip: "localhost", port: 8005};
	engine.connect();
}

function update(dt:Float) {
	engine.client.update();
	Render.update(dt);
	/*g.clear();
		g.lineStyle(1, 0xFFFFFF);
		for (obj in Render.objs) {
			obj.updateBounds();
			g.drawRect(obj.bounds.x, obj.bounds.y, obj.bounds.width, obj.bounds.height);
	}*/
}

function resize() {}
