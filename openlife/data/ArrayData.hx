package openlife.data;

/**
 * Int version of ArrayDataArray
 */
@:generic class ArrayData<T> {
	var array:Array<Array<T>> = [];

	// diffrence
	public var dx:Int = 0;
	public var dy:Int = 0;

	public function new() {
		array[0] = [];
	}

	public function clear() {
		array = [];
		dx = 0;
		dy = 0;
	}

	public function row(y:Int):Array<T> {
		return array[y - dy];
	}

	public function get(x:Int, y:Int):T {
		if (array[y - dy] != null) {
			return array[y - dy][x - dx];
		}
		return null;
	}

	public function shiftY(y:Int) {
		// shift
		if (y < dy) {
			for (i in 0...dy - y)
				array.unshift([]);
			dy = y;
		}
	}

	public function shiftX(x:Int, value:T) {
		if (x < dx) {
			for (j in 0...array.length) {
				if (array[j] == null)
					array[j] = [];
				for (i in 0...dx - x) {
					array[j].unshift(null);
				}
			}
			dx = x;
		}
	}

	public function set(x:Int, y:Int, value:T) {
		shiftY(y);
		shiftX(x, value);
		// set value
		if (array[y - dy] == null)
			array[y - dy] = [];
		array[y - dy][x - dx] = value;
	}
}
