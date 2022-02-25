package openlife.data.object.player;

@:expose("PlayerMove")
class PlayerMove {
	public var id:Int = 0;
	public var x:Int = 0;
	public var y:Int = 0;
	public var endX:Int = 0;
	public var endY:Int = 0;
	public var total:Float = 0;
	public var eta:Float = 0;
	public var trunc:Bool = false;
	public var moves:Array<Pos> = [];

	var movesString:String;

	public function new(a:Array<String>) {
		var i:Int = 0;
		id = Std.parseInt(a[i++]);
		x = Std.parseInt(a[i++]);
		y = Std.parseInt(a[i++]);
		total = Std.parseFloat(a[i++]);
		eta = Std.parseFloat(a[i++]);
		trunc = a[i++] == "1";
		a = a.splice(i, a.length);
		for (i in 0...Std.int(a.length / 2)) {
			moves.push(new Pos(Std.parseInt(a[i * 2]), Std.parseInt(a[i * 2 + 1])));
		}
		movesString = a.join(" ");
		var pos = moves.pop();
		endX = x + pos.x;
		endY = y + pos.y;
		moves.push(pos);
	}

	public function toData():String {
		return '$id $x $y $total $eta ${trunc ? 1 : 0} $movesString';
	}
}
