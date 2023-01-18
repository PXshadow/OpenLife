import BinPack.Rect;
import SpriteBatch.BatchElement;
import h2d.Tile;
import h2d.TileGroup;
import h2d.col.Bounds;
import hxd.Pixels;
import openlife.data.object.ObjectData;
import openlife.graphics.TgaData;
import openlife.resources.ObjectBake;
import openlife.resources.Resource;

var map:TileGroup;
var groundUnknownIndex = 0;
private var batches:Array<SpriteBatch> = [];
private var batchPixels:Array<Pixels> = [];
private var packers:Array<BinPack> = [];
var objs:Array<Object> = [];
private inline var MAX_TEXTURE = 4096;
inline var GRID = 128;
private var spriteMap:Map<Int, SpriteRect> = [];
private var groundMap:Map<Int, Rect> = [];

function addObject(x:Int, y:Int, ids:Array<Int>) {
	if (ids == null || ids.length == 0 || ids[0] == 0) return;
	x *= GRID;
	y *= GRID;
	final objData = ObjectData.getObjectData(ids[0]);
	if (objData == null) {
		trace("obj data null for id: " + ids[0]);
		return;
	}
	if (objData.spriteArray == null) {
		trace("obj data sprite array null for id: " + ids[0]);
		return;
	}
	final age = 20;
	final obj = new Object(x, y, objData);
	for (spriteData in objData.spriteArray) {
		final elem = getSpriteElement(spriteData.spriteID);
		elem.rotation = spriteData.rot * Math.PI * 2;
		elem.x = x + spriteData.x;
		elem.y = y - spriteData.y;
		elem.t.dx += -spriteData.inCenterXOffset;
		elem.t.dy += -spriteData.inCenterYOffset;
		elem.rotation = spriteData.rot * Math.PI * 2;
		elem.r = spriteData.color[0];
		elem.g = spriteData.color[1];
		elem.b = spriteData.color[2];
		if ((spriteData.ageRange[0] > -1 || spriteData.ageRange[1] > -1)
			&& (spriteData.ageRange[0] >= age || spriteData.ageRange[1] < age)) elem.visible = false;
		if (spriteData.hFlip == 1) elem.scaleX = -1;
		obj.sprites.push(elem);
	}
	obj.updateBounds();
	objs.push(obj);
}

function update(dt:Float) {
	for (obj in objs) {
		obj.updateBounds();
		obj.update(dt);
	}
	ysort();
}

function ysort() {
	for (batch in batches)
		batch.clear();
	objs.sort((a, b) -> {
		return a.bounds.yMax > b.bounds.yMax ? 1 : -1;
	});
	for (obj in objs) {
		for (sprite in obj.sprites) {
			sprite.batch.add(sprite);
		}
	}
}

final tga = new TgaData();

function getSpriteElement(id:Int):BatchElement {
	final m = spriteMap.get(id);
	final e = new BatchElement(batches[m.batchId].tile.sub(m.rect.x, m.rect.y, m.rect.width, m.rect.height).center());
	batches[m.batchId].add(e);
	return e;
}

function addGround(x:Int, y:Int, id:Int) {
	var index = id * 16 + abs(x % 4) + abs(y % 4) * 4 + 4;
	var rect = groundMap[index];
	if (rect == null) {
		index = 99999 + abs(x % 4) + abs(y % 4) * 4;
		rect = groundMap[index];
	}
	map.add(x * GRID, y * GRID, map.tile.sub(rect.x, rect.y, rect.width, rect.height).center());
	if (abs(x % 4) == 0 && abs(y % 0) == 0) {
		index = abs(x % 8) + abs(y % 8) * 2;
		rect = groundMap[index];
		map.add(x * GRID, y * GRID, map.tile.sub(rect.x, rect.y, rect.width, rect.height).center());
	}
}

inline function abs(x:Int):Int
	return x >= 0 ? x : -x;

function loadGround() {
	map = new TileGroup(null, Game.s2d);
	if (sys.FileSystem.exists('${ClientSettings.SaveDirectory}/SaveGroundData.bin')) {
		try {
			final pixels = hxd.res.Any.fromBytes("", sys.io.File.getBytes('ground.png')).toImage().getPixels(BGRA);
			map.tile = Tile.fromPixels(pixels);
			groundMap = haxe.Unserializer.run(sys.io.File.getContent('${ClientSettings.SaveDirectory}/SaveGroundData.bin'));
			return;
		} catch (_) {}
	}
	final pixels = Pixels.alloc(MAX_TEXTURE, MAX_TEXTURE, BGRA);
	final packer = new BinPack(MAX_TEXTURE, MAX_TEXTURE);
	final a = ""; // "_square";
	var index = 0;
	for (id in 0...4) {
		tga.read(Resource.groundOverlay(id));
		final groundPixels = Pixels.alloc(tga.rect.width, tga.rect.height, BGRA);
		groundPixels.bytes = tga.bytes;
		final rect = packer.pack({width: groundPixels.width, height: groundPixels.height});
		groundMap[index++] = rect;
		pixels.blit(rect.x, rect.y, groundPixels, 0, 0, rect.width, rect.height);
	}
	for (id in 0...6 + 1) {
		for (y in 0...4) {
			for (x in 0...4) {
				tga.read(Resource.ground(id, x, y, a));
				final groundPixels = Pixels.alloc(tga.rect.width, tga.rect.height, BGRA);
				groundPixels.bytes = tga.bytes;
				final rect = packer.pack({width: groundPixels.width, height: groundPixels.height});
				groundMap[index++] = rect;
				pixels.blit(rect.x, rect.y, groundPixels, 0, 0, rect.width, rect.height);
			}
		}
	}
	// unknown
	groundUnknownIndex = index;
	for (color in [0, 0xFFFFFFFF, 0x0000FFFF, 0xFFFF00FF, 0xFFFF00FF]) {
		for (y in 0...4) {
			for (x in 0...4) {
				tga.read(Resource.ground(99999, x, y, a));
				final groundPixels = Pixels.alloc(tga.rect.width, tga.rect.height, BGRA);
				if (color != 0) {
					for (i in 0...Std.int(tga.bytes.length / 4)) {
						if (tga.bytes.getInt32(i * 4) == 0xFFFFFFFF) {
							tga.bytes.setInt32(i * 4, color);
						}
					}
				}
				groundPixels.bytes = tga.bytes;
				final rect = packer.pack({width: groundPixels.width, height: groundPixels.height});
				if (color != 0) {
					groundMap[index++] = rect;
				} else {
					groundMap[99999 + x + y * 4] = rect;
				}

				pixels.blit(rect.x, rect.y, groundPixels, 0, 0, rect.width, rect.height);
			}
		}
	}
	sys.io.File.saveBytes('ground.png', pixels.toPNG());
	sys.io.File.saveContent('${ClientSettings.SaveDirectory}/SaveGroundData.bin', haxe.Serializer.run(groundMap));
	map.tile = Tile.fromPixels(pixels);
}

function loadSprites() {
	if (sys.FileSystem.exists('${ClientSettings.SaveDirectory}/SaveSpriteData.bin')) {
		var batchExists = new Map<Int, Bool>();
		try {
			spriteMap = haxe.Unserializer.run(sys.io.File.getContent('${ClientSettings.SaveDirectory}/SaveSpriteData.bin'));
			for (_ => elem in spriteMap) {
				if (batchExists.exists(elem.batchId)) continue;
				batchExists[elem.batchId] = true;
				final b = sys.io.File.getBytes(elem.batchId + ".png");
				final pixels = hxd.res.Any.fromBytes("", sys.io.File.getBytes('${elem.batchId}.png')).toImage().getPixels(BGRA);
				final b = new SpriteBatch(pixels, Game.s2d);
				b.smooth = true;
				batches.push(b);
			}
			return; // quick load
		} catch (_) {}
	}

	for (id in ObjectBake.objectList()) {
		for (spriteData in ObjectData.getObjectData(id).spriteArray) {
			if (spriteMap.exists(spriteData.spriteID)) continue;
			tga.read(Resource.spriteImage(spriteData.spriteID));
			final pixels = Pixels.alloc(tga.rect.width, tga.rect.height, BGRA);
			pixels.bytes = tga.bytes;
			var size:Rect = {width: pixels.width, height: pixels.height};
			var rect:Rect = null;
			for (i in 0...packers.length) {
				rect = packers[i].pack(size);
				if (rect == null) continue; // if rect is null that means the texture is full
				batchPixels[i].blit(rect.x, rect.y, pixels, 0, 0, rect.width, rect.height);
				spriteMap[spriteData.spriteID] = new SpriteRect(i, rect);
			}
			if (spriteMap.exists(spriteData.spriteID)) continue;
			// create new texture as others are full
			packers.push(new BinPack(MAX_TEXTURE, MAX_TEXTURE));
			batchPixels.push(Pixels.alloc(MAX_TEXTURE, MAX_TEXTURE, BGRA));
			final i = packers.length - 1;
			rect = packers[i].pack(size);
			batchPixels[i].blit(rect.x, rect.y, pixels, 0, 0, rect.width, rect.height);
			spriteMap[spriteData.spriteID] = new SpriteRect(i, rect);
		}
	}
	for (i in 0...batchPixels.length) {
		final pixels = batchPixels[i];

		sys.io.File.saveBytes('$i.png', pixels.toPNG());
		batches.push(new SpriteBatch(pixels, Game.s2d));
	}
	sys.io.File.saveContent('${ClientSettings.SaveDirectory}/SaveSpriteData.bin', haxe.Serializer.run(spriteMap));
}

class SpriteRect {
	public var rect:Rect;
	public var batchId:Int;

	public inline function new(batchId, rect) {
		this.batchId = batchId;
		this.rect = rect;
	}
}
