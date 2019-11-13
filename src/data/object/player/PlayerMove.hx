package data.object.player;
import game.Player;
import console.Program.Pos;
class PlayerMove 
{
    public var id:Int = 0;
    var xs:Int = 0;
    var ys:Int = 0;
    var total:Float = 0;
    var current:Float = 0;
    var trunc:Bool = false;
    var moves:Array<{x:Int,y:Int}> = [];
    public function  new(a:Array<String>)
    {
        var index:Int = 0;
        for(value in a)
        {
            switch(index++)
            {
                case 0:
                id = Std.parseInt(value);
                case 1:
                xs = Std.parseInt(value);
                case 2:
                //flip
                ys = Std.parseInt(value);
                case 3:
                total = Std.parseFloat(value);
                case 4:
                current = Std.parseFloat(value);
                case 5:
                trunc = value == "1" ? true : false;
                default:
                if(index > 6)
                {
                    if(index%2 == 0)
                    {
                        moves[moves.length - 1].y = Std.parseInt(value);
                    }else{
                        moves.push({x:Std.parseInt(value),y:0});
                    }
                }else{
                    throw("Player move parsing moves failed");
                }
            }
        }
    }
    public function movePlayer(player:Player)
    {
        //set pos
        player.ix = xs;
        player.iy = ys;
        if (trunc) player.force();
        var currentX:Int = 0;
        var currentY:Int = 0;
        player.moves = [];
        var pos:Pos;
        for(move in moves)
        {
            pos = new Pos();
            pos.x = move.x - currentX;
            pos.y = move.y - currentY;
            player.moves.push(pos);
            currentX = move.x;
            currentY = move.y;
        }
        player.motion();
    }
}