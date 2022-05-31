import SpriteBatch.BatchElement;
import openlife.data.animation.AnimationData;
import openlife.data.animation.AnimationRecord;
import openlife.data.object.ObjectData;

class Object {
	public var sprites:Array<BatchElement> = [];
	public var id:Int;
	public var data:ObjectData;
	public var anim:AnimationData;
	public var x:Float = 0;
	public var y:Float = 0;
	public var workingSpriteFades:Map<Int, Float> = [];

	public function new(x, y, data) {
		this.x = x;
		this.y = y;
		this.data = data;
		try {
			anim = new AnimationData(data.id);
		} catch (_) {}
	}

	public function play() {}

	var inFrameTime = 0.0;

	public function update(dt:Float) {
		if (anim == null) return;
		final record = anim.record[2];

		for (i in 0...record.params.length) {
			final param = record.params[i];
			final sprite = sprites[i];
			final sprite = sprites[i];
			sprite.x = x + data.spriteArray[i].x;
			sprite.y = y - data.spriteArray[i].y;
			sprite.rotation = -data.spriteArray[i].rot * 2 * Math.PI;
			sprite.x += getOscOffset(inFrameTime, param.offsetX, param.xOscPerSec, param.xAmp, param.xPhase);
			sprite.y += -getOscOffset(inFrameTime, param.offsetY, param.yOscPerSec, param.yAmp, param.yPhase);
			sprite.rotation += getOscOffset(inFrameTime, 0, param.rockOscPerSec, param.rockAmp, param.rockPhase) * 2 * Math.PI;
			final totalRotOffset = param.rotPerSec * inFrameTime + param.rotPhase;

			final sinVal = getOscOffset(inFrameTime, 0, param.fadeOscPerSec, 1, param.fadePhase + 0.25);

			final hardness = param.fadeHardness;
			var hardVersion = 0.0;

			if (hardness == 1) {
				hardVersion = sinVal > 0 ? 1 : -1;
			} else {
				var absSinVal = Math.abs(sinVal);
				hardVersion = absSinVal != 0 ? (sinVal / absSinVal) * Math.pow(absSinVal, 1 / (hardness * 10 + 1)) : 0;
			}

			final fade = (param.fadeMax - param.fadeMin) * (0.5 * hardVersion + 0.5) + param.fadeMin;

			sprite.alpha = fade;

			// relative to 0 on circle
			var relativeRotOffset = totalRotOffset - Math.floor(totalRotOffset);

			// make positive
			if (relativeRotOffset < 0) {
				relativeRotOffset += 1;
			}

			// to take average of two rotations
			// rotate them both so that one is at 0.5,
			// take average, and then rotate them back
			// This ensures that we always move through closest average
			// point on circle (shortest path along circle).

			var offset = 0.5 - relativeRotOffset;

			sprite.rotation += relativeRotOffset * Math.PI * 2;
		}
		for (i in 0...record.params.length) {
			if (data.spriteArray[i].parent != -1) continue;
			transformChild(i, record);
		}
		inFrameTime += dt / 2;
	}

	private function transformChild(parentId:Int, record:AnimationRecord) {
		for (i in 0...record.params.length) {
			if (data.spriteArray[i].parent != parentId) continue;
			final parent = sprites[parentId];
			final parentData = data.spriteArray[parentId];
			final childData = data.spriteArray[i];
			final child = sprites[i];
			child.x += parent.x - x - parentData.x;
			child.y += parent.y - y + parentData.y;

			final rot = parent.rotation - parentData.rot * Math.PI * 2;
			child.rotation += rot;
			rotateTransform(child, parent, rot);

			transformChild(i, record);
		}
	}

	private inline function rotateTransform(child:BatchElement, parent:BatchElement, rot:Float) {
		child.x += -parent.x;
		child.y += -parent.y;

		final s = Math.sin(rot);
		final c = Math.cos(rot);

		final x = child.x * c - child.y * s;
		final y = child.x * s + child.y * c;

		child.x = x + parent.x;
		child.y = y + parent.y;
	}

	private inline function getOscOffset(inFrameTime:Float, inOffset:Float, inOscPerSec:Float, inAmp:Float, inPhase:Float)
		return inOffset + inAmp * Math.sin((inFrameTime * inOscPerSec + inPhase) * 2 * Math.PI);
}
