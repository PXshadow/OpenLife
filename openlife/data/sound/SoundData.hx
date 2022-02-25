package openlife.data.sound;

@:expose
class SoundData {
	/**
	 * id of the sound
	 */
	public var id:Int = 0;

	/**
	 * multiplyer of the volume
	 */
	public var multi:Float = 0;

	/**
	 * whether music or sound
	 */
	public var music:Bool = false;

	/**
	 * New sound data generate
	 * @param string "id:multi"
	 */
	public function new(string:String) {
		var array = string.split(":");
		id = Std.parseInt(array[0]);
		multi = Std.parseFloat(array[1]);
	}
}
