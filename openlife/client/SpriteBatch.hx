package;

import h2d.Drawable;
import h2d.RenderContext;
import h2d.Tile;
import hxd.Pixels;

@:allow(h2d.SpriteBatch)
class BatchElement {
	public var x:Float;
	public var y:Float;
	public var scale(never, set):Float;
	public var scaleX:Float;
	public var scaleY:Float;
	public var rotation:Float;
	public var r:Float;
	public var g:Float;
	public var b:Float;
	public var a:Float;
	public var t:Tile;
	public var alpha(get, set):Float;
	public var visible:Bool;
	public var batch(default, null):SpriteBatch;

	public function new(t) {
		x = 0;
		y = 0;
		r = 1;
		g = 1;
		b = 1;
		a = 1;
		rotation = 0;
		scaleX = scaleY = 1;
		visible = true;
		this.t = t;
	}

	inline function set_scale(v) {
		return scaleX = scaleY = v;
	}

	inline function get_alpha() {
		return a;
	}

	inline function set_alpha(v) {
		return a = v;
	}

	function update(et:Float) {
		return true;
	}

	public function remove() {
		if (batch != null) @:privateAccess batch.delete(this);
	}
}

class SpriteBatch extends Drawable {
	public var tile:Tile;
	public var hasRotationScale:Bool;
	public var hasUpdate:Bool;

	var tmpBuf:hxd.FloatBuffer;
	var buffer:h3d.Buffer;
	var bufferVertices:Int;

	public var array:Array<BatchElement> = [];

	public function new(pixels, ?parent) {
		super(parent);
		this.hasRotationScale = true;
		tile = Tile.fromPixels(pixels);
	}

	public function add(e:BatchElement, before = false) {
		@:privateAccess e.batch = this;
		if (!before) {
			array.push(e);
		} else {
			array.unshift(e);
		}
		return e;
	}

	public function clear() {
		array = [];
		flush();
	}

	public function alloc(t) {
		return add(new BatchElement(t));
	}

	@:allow(h2d.BatchElement)
	function delete(e:BatchElement) {
		array.remove(e);
	}

	override function sync(ctx) {
		super.sync(ctx);
		flush();
	}

	override function getBoundsRec(relativeTo, out, forSize) {
		super.getBoundsRec(relativeTo, out, forSize);
		var index:Int = 0;
		var e = array[index];
		for (i in 0...array.length) {
			var t = e.t;
			if (hasRotationScale) {
				var ca = Math.cos(e.rotation), sa = Math.sin(e.rotation);
				var hx = t.width, hy = t.height;
				var px = t.dx * e.scaleX, py = t.dy * e.scaleY;
				var x, y;

				x = px * ca - py * sa + e.x;
				y = py * ca + px * sa + e.y;
				addBounds(relativeTo, out, x, y, 1e-10, 1e-10);

				var px = (t.dx + hx) * e.scaleX, py = t.dy * e.scaleY;
				x = px * ca - py * sa + e.x;
				y = py * ca + px * sa + e.y;
				addBounds(relativeTo, out, x, y, 1e-10, 1e-10);

				var px = t.dx * e.scaleX, py = (t.dy + hy) * e.scaleY;
				x = px * ca - py * sa + e.x;
				y = py * ca + px * sa + e.y;
				addBounds(relativeTo, out, x, y, 1e-10, 1e-10);

				var px = (t.dx + hx) * e.scaleX, py = (t.dy + hy) * e.scaleY;
				x = px * ca - py * sa + e.x;
				y = py * ca + px * sa + e.y;
				addBounds(relativeTo, out, x, y, 1e-10, 1e-10);
			} else {
				addBounds(relativeTo, out, e.x + t.dx, e.y + t.dy, t.width, t.height);
			}
			e = array[++index];
		}
	}

	function flush() {
		if (array.length == 0) {
			bufferVertices = 0;
			return;
		}
		if (tmpBuf == null) tmpBuf = new hxd.FloatBuffer();
		var pos = 0;
		var index = 0;
		var e = array[index];
		var tmp = tmpBuf;
		for (i in 0...array.length) {
			if (!e.visible) {
				e = array[++index];
				continue;
			}

			var t = e.t;

			tmp.grow(pos + 8 * 4);

			if (hasRotationScale) {
				var ca = Math.cos(e.rotation), sa = Math.sin(e.rotation);
				var hx = t.width, hy = t.height;
				var px = t.dx * e.scaleX, py = t.dy * e.scaleY;
				tmp[pos++] = px * ca - py * sa + e.x;
				tmp[pos++] = py * ca + px * sa + e.y;
				@:privateAccess tmp[pos++] = t.u;
				@:privateAccess tmp[pos++] = t.v;
				tmp[pos++] = e.r;
				tmp[pos++] = e.g;
				tmp[pos++] = e.b;
				tmp[pos++] = e.a;
				var px = (t.dx + hx) * e.scaleX, py = t.dy * e.scaleY;
				tmp[pos++] = px * ca - py * sa + e.x;
				tmp[pos++] = py * ca + px * sa + e.y;
				@:privateAccess tmp[pos++] = t.u2;
				@:privateAccess tmp[pos++] = t.v;
				tmp[pos++] = e.r;
				tmp[pos++] = e.g;
				tmp[pos++] = e.b;
				tmp[pos++] = e.a;
				var px = t.dx * e.scaleX, py = (t.dy + hy) * e.scaleY;
				tmp[pos++] = px * ca - py * sa + e.x;
				tmp[pos++] = py * ca + px * sa + e.y;
				@:privateAccess tmp[pos++] = t.u;
				@:privateAccess tmp[pos++] = t.v2;
				tmp[pos++] = e.r;
				tmp[pos++] = e.g;
				tmp[pos++] = e.b;
				tmp[pos++] = e.a;
				var px = (t.dx + hx) * e.scaleX, py = (t.dy + hy) * e.scaleY;
				tmp[pos++] = px * ca - py * sa + e.x;
				tmp[pos++] = py * ca + px * sa + e.y;
				@:privateAccess tmp[pos++] = t.u2;
				@:privateAccess tmp[pos++] = t.v2;
				tmp[pos++] = e.r;
				tmp[pos++] = e.g;
				tmp[pos++] = e.b;
				tmp[pos++] = e.a;
			} else {
				var sx = e.x + t.dx;
				var sy = e.y + t.dy;
				tmp[pos++] = sx;
				tmp[pos++] = sy;
				@:privateAccess tmp[pos++] = t.u;
				@:privateAccess tmp[pos++] = t.v;
				tmp[pos++] = e.r;
				tmp[pos++] = e.g;
				tmp[pos++] = e.b;
				tmp[pos++] = e.a;
				tmp[pos++] = sx + t.width + 0.1;
				tmp[pos++] = sy;
				@:privateAccess tmp[pos++] = t.u2;
				@:privateAccess tmp[pos++] = t.v;
				tmp[pos++] = e.r;
				tmp[pos++] = e.g;
				tmp[pos++] = e.b;
				tmp[pos++] = e.a;
				tmp[pos++] = sx;
				tmp[pos++] = sy + t.height + 0.1;
				@:privateAccess tmp[pos++] = t.u;
				@:privateAccess tmp[pos++] = t.v2;
				tmp[pos++] = e.r;
				tmp[pos++] = e.g;
				tmp[pos++] = e.b;
				tmp[pos++] = e.a;
				tmp[pos++] = sx + t.width + 0.1;
				tmp[pos++] = sy + t.height + 0.1;
				@:privateAccess tmp[pos++] = t.u2;
				@:privateAccess tmp[pos++] = t.v2;
				tmp[pos++] = e.r;
				tmp[pos++] = e.g;
				tmp[pos++] = e.b;
				tmp[pos++] = e.a;
			}
			e = array[++index];
		}
		bufferVertices = pos >> 3;
		if (buffer != null && !buffer.isDisposed()) {
			if (buffer.vertices >= bufferVertices) {
				buffer.uploadVector(tmpBuf, 0, bufferVertices);
				return;
			}
			buffer.dispose();
			buffer = null;
		}
		if (bufferVertices > 0) buffer = h3d.Buffer.ofSubFloats(tmpBuf, 8, bufferVertices, [Dynamic, Quads, RawFormat]);
	}

	override function draw(ctx:RenderContext) {
		drawWith(ctx, this);
	}

	@:allow(h2d)
	function drawWith(ctx:RenderContext, obj:Drawable) {
		if (array.length == 0 || buffer == null || buffer.isDisposed() || bufferVertices == 0) return;
		if (!ctx.beginDrawObject(obj, tile.getTexture())) return;
		ctx.engine.renderQuadBuffer(buffer, 0, bufferVertices >> 1);
	}

	public inline function isEmpty() {
		return array.length == 0;
	}

	public inline function getArray() {
		return array;
	}

	override function onRemove() {
		super.onRemove();
		if (buffer != null) {
			buffer.dispose();
			buffer = null;
		}
	}
}
