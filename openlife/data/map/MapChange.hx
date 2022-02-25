package openlife.data.map;

@:expose("MapChange")
class MapChange {
	/**
	 * Tile X
	 */
	public var x:Int = 0;

	/**
	 * Tile Y
	 */
	public var y:Int = 0;

	/**
	 * Floor boolean
	 */
	public var floor:Int = 0;

	/**
	 * Id array
	 */
	public var id:Array<Int> = [];

	/**
	 * Player id that did the map change
	 */
	public var pid:Int = 0;

	/**
	 * Old position x
	 */
	public var oldX:Int = 0;

	/**
	 * Old position y
	 */
	public var oldY:Int = 0;

	/**
	 * Speed of changed object
	 */
	public var speed:Float = 0;

	/**
	 * New map change data
	 * @param array properties of map change
	 */
	public function new(array:Array<String>) {
		x = Std.parseInt(array[0]);
		y = Std.parseInt(array[1]);
		floor = Std.parseInt(array[2]);
		id = MapData.id(array[3]);
		if (id.length == 0)
			id = [0];
		pid = Std.parseInt(array[4]);
		// optional speed
		if (array.length > 5) {
			oldX = Std.parseInt(array[5]);
			oldY = Std.parseInt(array[6]);
			speed = Std.parseFloat(array[7]);
		}
	}

	/**
	 * string for debug
	 * @return String
	 */
	public function toString():String {
		var string:String = "";
		for (field in Reflect.fields(this)) {
			string += field + ": " + Reflect.getProperty(this, field) + "\n";
		}
		return string;
	}
}
