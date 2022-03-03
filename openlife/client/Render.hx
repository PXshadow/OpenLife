import BinPack.Rect;
import SpriteBatch.BatchElement;
import h2d.Tile;
import hxd.Pixels;
import openlife.data.object.ObjectData;
import openlife.graphics.TgaData;
import openlife.resources.ObjectBake;
import openlife.resources.Resource;

private var batches:Array<SpriteBatch> = [];
private var batchPixels:Array<Pixels> = [];
private var packers:Array<BinPack> = [];
private var objs:Array<Object> = [];
private inline var MAX_TEXTURE = 4096;
inline var GRID = 128;
private var spriteMap:Map<Int, SpriteRect> = [];

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
		elem.x = x + spriteData.pos.x;
		elem.y = y - spriteData.pos.y;
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
	objs.push(obj);
}

function update(dt:Float) {}
final tga = new TgaData();

function getSpriteElement(id:Int):BatchElement {
	final m = spriteMap.get(id);
	final e = new BatchElement(batches[m.batchId].tile.sub(m.rect.x, m.rect.y, m.rect.width, m.rect.height).center());
	batches[m.batchId].add(e);
	return e;
}

function loadSprites() {
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
				if (rect == null) continue;
				batchPixels[i].blit(rect.x, rect.y, pixels, 0, 0, rect.width, rect.height);
				spriteMap[spriteData.spriteID] = new SpriteRect(i, rect);
			}
			if (spriteMap.exists(spriteData.spriteID)) continue;

			packers.push(new BinPack(MAX_TEXTURE, MAX_TEXTURE));
			batchPixels.push(Pixels.alloc(MAX_TEXTURE, MAX_TEXTURE, BGRA));
			final i = packers.length - 1;
			rect = packers[i].pack(size);
			batchPixels[i].blit(rect.x, rect.y, pixels, 0, 0, rect.width, rect.height);
			spriteMap[spriteData.spriteID] = new SpriteRect(i, rect);
		}
	}
	for (pixels in batchPixels) {
		batches.push(new SpriteBatch(pixels, Game.s2d));
	}
	trace("batches count: " + batches.length); 
}

class SpriteRect {
	public var rect:Rect;
	public var batchId:Int;

	public inline function new(batchId, rect) {
		this.batchId = batchId;
		this.rect = rect;
	}
}
