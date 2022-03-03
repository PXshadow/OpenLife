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
		/*try {
			anim = new AnimationData(data.id);
		} catch (_) {}*/
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
			sprite.x = x + data.spriteArray[i].pos.x;
			sprite.y = y - data.spriteArray[i].pos.y;
			sprite.rotation = data.spriteArray[i].rot * Math.PI * 2;
			sprite.x += getOscOffset(inFrameTime, param.offset.x, param.xOscPerSec, param.xAmp, param.xPhase);
			sprite.y += -getOscOffset(inFrameTime, param.offset.y, param.yOscPerSec, param.yAmp, param.yPhase);
			sprite.rotation += getOscOffset(inFrameTime, 0, param.rockOscPerSec, param.rockAmp, param.rockPhase) * Math.PI * 2;
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
			child.x += parent.x - x - parentData.pos.x;
			child.y += parent.y - y + parentData.pos.y;

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

	private inline function phase(x:Float):Float {
		if (x > 0.75) return x - 1;
		return (x * 2 - 1) * -2;
	}
}
