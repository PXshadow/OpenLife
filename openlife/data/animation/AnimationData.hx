package openlife.data.animation;

import openlife.data.sound.SoundData;
import haxe.ds.Vector;
import openlife.resources.Resource;

@:expose
class AnimationData extends openlife.data.LineReader {
	/**
	 * If animation failed to load
	 */
	public var fail:Bool = false;

	/**
	 * Records of animation
	 */
	@:keep public var record:Vector<AnimationRecord>;

	public function new(id:Int) {
		super();
		// fail = true;
		record = new Vector<AnimationRecord>(5 + 1);
		for (i in 0...5 + 1) {
			// skip 3
			if (i == 3)
				continue;
			// read lines
			if (!readLines(Resource.animation(id, i)))
				return;
			record[i] = process();
		}
		line = null;
	}

	/**
	 * Process animation
	 * @return AnimationRecord
	 */
	public function process():AnimationRecord {
		// id
		var animation = new AnimationRecord();
		animation.id = getInt();
		// type
		var string = getString();
		var cut:Int = string.indexOf(",");
		var sep = string.indexOf(":");
		if (animation.type == moving)
			trace("moving process");
		if (sep == -1) {
			sep = cut;
		} else {
			// extra index read
		}
		switch (Std.parseInt(string.substring(0, sep))) {
			case 0:
				animation.type = ground;
			case 1:
				animation.type = held;
			case 2:
				animation.type = moving;
			case 3:
				animation.type = eating;
			case 4:
				animation.type = doing;
			case 5:
				animation.type = endAnimType;
		}
		// rand start phase
		animation.randStartPhase = Std.parseFloat(string.substring(cut + 1, string.length));

		if (readName("forceZeroStart")) {
			next++;
		}
		// next++;
		// num
		if (readName("numSounds")) {
			animation.numSounds = getInt();
			// skip over sounds
			if (animation.numSounds > 0) {
				animation.soundAnim = new Vector<SoundParameter>(animation.numSounds);
				for (i in 0...animation.numSounds) {
					animation.soundAnim[i] = new SoundParameter();
					var array = getString().split("#");
					var index = array[array.length - 1].indexOf(" ");
					array[array.length - 1] = array[array.length - 1].substring(0, index);
					var propString = array[array.length - 1].substring(index + 1, array[array.length - 1].length);
					animation.soundAnim[i].sounds = new Vector<SoundData>(array.length);
					for (j in 0...array.length) {
						animation.soundAnim[i].sounds[j] = new SoundData(array[j]);
					}
					array = propString.split(" ");
					animation.soundAnim[i].repeatPerSec = Std.parseFloat(array[0]);
					animation.soundAnim[i].repeatPhase = Std.parseFloat(array[1]);
					animation.soundAnim[i].ageStart = Std.parseFloat(array[2]);
					animation.soundAnim[i].ageEnd = Std.parseFloat(array[3]);
				}
			}
		}
		animation.numSprites = getInt();
		animation.numSlots = getInt();
		// sprites
		#if neko
		if (animation.numSprites == null)
			return animation;
		#end
		if (animation.numSprites <= 0)
			return animation;
		animation.params = new Vector<AnimationParameter>(animation.numSprites);
		for (i in 0...animation.params.length)
			animation.params[i] = processParam();
		// slots
		animation.slotAnim = new Vector<AnimationParameter>(animation.numSlots);
		for (i in 0...animation.slotAnim.length)
			animation.slotAnim[i] = processParam();
		// done
		return animation;
	}

	/**
	 * Process the paramaters of record
	 * @return AnimationParameter
	 */
	public function processParam():AnimationParameter {
		var param:AnimationParameter = new AnimationParameter();
		if (readName("offset")) {
			var s:String = getString();
			var cut = s.indexOf(",");
			param.offset = new Point(Std.parseFloat(s.substring(1, cut)), Std.parseFloat(s.substring(cut + 1, s.length - 1)));
		}
		if (readName("startPause")) {
			param.startPauseSec = getFloat();
		}
		// animation param
		var animParam = getString();
		param.process(animParam.split(" "));
		return param;
	}
}
