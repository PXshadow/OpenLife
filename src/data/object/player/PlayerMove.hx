package data.object.player;

class PlayerMove
{
    public var id:Int = 0;
    public var x:Int = 0;
    public var y:Int = 0;
    public var total:Float = 0;
    public var eta:Float = 0;
    public var trunc:Bool = false;
    public var moves:Array<Pos> = [];
    public function new(a:Array<String>)
    {
        var index:Int = 0;
        var i:Int = 0;
        id = Std.parseInt(a[i++]);
        x = Std.parseInt(a[i++]);
        y = Std.parseInt(a[i++]);
        total = Std.parseFloat(a[i++]);
        eta = Std.parseFloat(a[i++]);
        trunc = a[i++] == "1" ? true : false;
        for (j in i...a.length)
        {
            if (j % 2 != 0)
            {
                index = moves.push(new Pos(Std.parseInt(a[j])));
            }else{
                moves[index].y = Std.parseInt(a[j]);
            }
        }
    }
}