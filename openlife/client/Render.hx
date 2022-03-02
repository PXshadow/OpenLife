import BinPack.Rect;
import SpriteBatch.BatchElement;
import h2d.Tile;
import hxd.Pixels;
import openlife.data.object.ObjectData;
import openlife.graphics.TgaData;
import openlife.resources.Resource;

private var batches:Array<SpriteBatch> = [];
private var packers:Array<BinPack> = [];
private var objs:Array<Object> = [];
private inline var MAX_TEXTURE = 4096;
inline var GRID = 128;
private var spriteMap:Map<Int, SpriteRect> = [];

function genBatch() {
	packers.push(new BinPack(MAX_TEXTURE, MAX_TEXTURE));
	return batches.push(new SpriteBatch(Pixels.alloc(MAX_TEXTURE, MAX_TEXTURE, BGRA), Game.s2d));
}

function addObject(x:Int, y:Int, ids:Array<Int>) {
	if (ids == null || ids.length == 0 || ids[0] == 0) return;
	x *= GRID;
	y *= GRID;
	final objData = new ObjectData(ids[0]);
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

function update(dt:Float) {
	for (obj in objs)
		obj.update(dt);
}

final tga = new TgaData();

function getSpriteElement(id:Int):BatchElement {
	if (spriteMap.exists(id)) {
		final m = spriteMap.get(id);
		final e = new BatchElement(batches[m.batchId].tile.sub(m.rect.x, m.rect.y, m.rect.width, m.rect.height).center());
		batches[m.batchId].add(e);
		return e;
	}
	tga.read(Resource.spriteImage(id));
	final pixels = Pixels.alloc(tga.rect.width, tga.rect.height, BGRA);
	pixels.bytes = tga.bytes;
	var size:Rect = {width: pixels.width, height: pixels.height};
	var rect:Rect = null;
	function task(i:Int) {
		rect = packers[i].pack(size);
		if (rect == null) return false;
		batches[i].pixels.blit(rect.x, rect.y, pixels, 0, 0, rect.width, rect.height);
		batches[i].invalidate();
		return true;
	}
	for (i in 0...packers.length) {
		if (!task(i)) continue;
		final e = new BatchElement(batches[i].tile.sub(rect.x, rect.y, rect.width, rect.height).center());
		batches[i].add(e);
		return e;
	}
	final i = genBatch() - 1;
	task(i);
	final e = new BatchElement(batches[i].tile.sub(rect.x, rect.y, rect.width, rect.height).center());
	batches[i].add(e);
	return e;
}

class SpriteRect {
	public var rect:Rect;
	public var batchId:Int;

	public function new(batchId, rect) {
		this.batchId = batchId;
		this.rect = rect;
	}
}
