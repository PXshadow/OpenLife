import haxe.ds.Vector;

class BinPack {
	public var spaces:Array<Rect> = [];

	var width:Int;
	var height:Int;

	public function new(width:Int, height:Int) {
		this.width = width;
		this.height = height;
		spaces.push({width: width,height: height});
	}

	/**
		size = width, height
	**/
	public function pack(size:Rect):Rect {
		// width, height
		spaces.sort(function(a, b) {
			return a.width * a.height > b.width * b.height ? 1 : -1;
		});
		for (space in spaces) {
			// 2 = width, height = 3, image is bigger than space
			if (space.width < size.width || space.height < size.height) 
                continue;
			// size fits exactly
			if (space.width == size.width && space.height == size.height) {
				spaces.remove(space);
				return space;
			}
			if (space.width == size.width && space.height > size.height) {
				// width fits exactly, height does not
				spaces.push({
					width: space.width, // width
					height: space.height - size.height, // height
					x: space.x, // x
					y: space.y + size.height, // y
				});
				spaces.remove(space);
				// width, height, x, y
				return {width: size.width, height: size.height, x: space.x, y: space.y};
			}
			if (space.height == size.height && space.width > size.width) {
				// height fits exactly, width does not
				spaces.push({
					width: space.width - size.width, // width
					height: space.height, // height
					x: space.x + size.width, // x
					y: space.y, // y
				});
				spaces.remove(space);
				return {width: size.width, height: size.height, x: space.x, y: space.y};
			}
			// every other option gone, size is strictly smaller than space both width and height
			if (space.width - size.width > space.height - size.height) {
				// width has more space
				spaces.push({
					width: space.width - size.width, // space.width - size.width = width
					height: space.height, // space.height = height
					x: space.x + size.width, // space.x + size.width = x
					y: space.y // space.y = y
				});
				spaces.push({
					width: size.width, // width
					height: space.height - size.height, // height
					x: space.x, // x
					y: space.y + size.height // y
				});
			} else {
				// height has more space
				spaces.push({
					width: space.width, // width
					height: space.height - size.height, // height
					x: space.x, // x
					y: space.y + size.height // y
				});
				spaces.push({
					width: space.width - size.width, // space.width - size.width = width
					height: size.height, // space.height = height
					x: space.x + size.width, // space.x + size.width = x
					y: space.y // space.y = y
				});
			}
			spaces.remove(space);
			return {width: size.width, height: size.height, x: space.x, y: space.y};
		}
		return null;
	}

	public function volumeLeft():Int {
		var left = width * height;
		for (space in spaces) {
			left -= space.width * space.height;
		}
		return left;
	}
}

@:structInit
class Rect {
    public var x:Int = 0;
    public var y:Int = 0;
    public var width:Int = 0;
    public var height:Int = 0;
    public var maxX(get, never):Int;
    public var maxY(get, never):Int;
    
    function get_maxX():Int
        return x + width;
    function get_maxY():Int
        return y + height;

    public inline function new(x=0,y=0,width,height) {
        this.x = x;
        this.y = y;
        this.width = width;
        this.height = height;
    }
	public function toString()
		return '$x,$y,$width,$height';
}