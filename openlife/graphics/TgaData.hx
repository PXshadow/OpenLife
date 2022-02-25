package openlife.graphics;

import format.tga.Tools;
import haxe.ds.Vector;
import haxe.io.Output;
import haxe.io.Bytes;
import openlife.data.Rectangle;
import haxe.io.BytesInput;
import format.tga.Data;
import format.tga.Reader;
import format.tga.Writer;

@:expose
class TgaData {
	public var bytes:Bytes;
	public var rect:Rectangle;

	// to read
	private var r:Reader;

	public var data:Data;

	public function new() {}

	public function read(bytes:Bytes, extractBool:Bool = true) {
		r = new Reader(new BytesInput(bytes, 0, bytes.length));
		this.data = r.read();
		if (extractBool)
			extract();
	}

	public function extract() {
		rect = new Rectangle(0, 0, data.header.width, data.header.height);
		this.bytes = Tools.extract32(data, alpha);
	}

	public function write(data:Data, output:Output) {
		var w:Writer = new Writer(output);
		w.write(data);
		output.close();
	}

	public function bytesToVector(bytes:Bytes):Vector<Int> {
		var imageData:Vector<Int> = new Vector<Int>(Std.int(bytes.length / 4));
		for (i in 0...imageData.length) {
			imageData[i] = bytes.getInt32(i);
		}
		return imageData;
	}

	/**
	 * crop alpha parts from the sides
	 */
	public var alpha:Bool = true;

	public function crop() {
		// shifting vars
		var minX:Int = getLeft(0, Std.int(data.header.width / 2), 0, data.header.height) - padding;
		var maxX:Int = getRight(0, data.header.width, 0, data.header.height) + padding;

		var minY:Int = getTop(0, data.header.width, 0, Std.int(data.header.height / 2)) - padding;
		var maxY:Int = getBottom(0, data.header.width, 0, data.header.height) + padding;
		// if padding is to much
		if (minX < 0)
			minX = 0;
		if (minY < 0)
			minY = 0;
		if (maxX > data.header.width)
			maxX = data.header.width;
		if (maxY > data.header.height)
			maxY = data.header.height;
		// create vector of image pixels
		var vector = new Vector<Int>((maxX - minX) * (maxY - minY));
		var index:Int = 0;
		var i:Int = 0;
		for (y in minY...maxY) {
			for (x in minX...maxX) {
				vector[index++] = data.imageData[x + y * data.header.width];
			}
		}
		data.imageData = vector;
		data.header.width = maxX - minX;
		data.header.height = maxY - minY;
		extract();
	}

	private static inline var threshold:Int = 3 * 255;
	private static inline var padding:Int = 15;

	private function getLeft(x0:Int, x1:Int, y0:Int, y1:Int):Int {
		// x
		for (i in x0...x1) {
			// y
			for (j in y0...y1) {
				if ((data.imageData[i + j * data.header.width] >> 24) & 0xff < threshold) {
					return i - 1;
				}
			}
		}
		return 0;
	}

	private function getRight(x0:Int, x1:Int, y0:Int, y1:Int):Int {
		// x
		var x:Int = x1;
		while (x > x0) {
			// y
			for (j in y0...y1) {
				if ((data.imageData[x + j * data.header.width] >> 24) & 0xff < threshold) {
					return x + 1;
				}
			}
			x--;
		}
		return 0;
	}

	private function getBottom(x0:Int, x1:Int, y0:Int, y1:Int, rev:Bool = false):Int {
		// y
		var y:Int = y1;
		while (y > y0) {
			// x
			for (i in x0...x1) {
				if ((data.imageData[i + y * data.header.width] >> 24) & 0xff < threshold) {
					return y + 1;
				}
			}
			y--;
		}
		return 0;
	}

	private function getTop(x0:Int, x1:Int, y0:Int, y1:Int, rev:Bool = false):Int {
		// y
		for (j in y0...y1) {
			// x
			for (i in x0...x1) {
				if ((data.imageData[i + j * data.header.width] >> 24) & 0xff < threshold) {
					return j - 1;
				}
			}
		}
		return 0;
	}
}
