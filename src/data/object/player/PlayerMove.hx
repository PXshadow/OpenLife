package data.object.player;
import game.Game;
import game.Player;
import data.Point;
#if nativeGen @:nativeGen #end
class PlayerMove 
{
    /**
     * Id of player
     */
    public var id:Int = 0;
    /**
     * x set
     */
    var xs:Int = 0;
    /**
     * y set
     */
    var ys:Int = 0;
    /**
     * N/A
     */
    var total:Float = 0;
    /**
     * N/A
     */
    var current:Float = 0;
    /**
     * N/A
     */
    var trunc:Bool = false;
    /**
     * Array of move points
     */
    var moves:Array<Point> = [];
    var time:Int = 0;
    var frames:Int = 0;
    var point:Point;
    var player:Player;
    public function  new(player:Player=null)
    {
        this.player = player;
    }
    public function parse(a:Array<String>)
    {
        var index:Int = 0;
        for(value in a)
        {
            switch(index++)
            {
                case 0:
                id = Std.parseInt(value);
                player = Game.data.playerMap.get(id);
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
                trace("trunc value " + value);
                trunc = value == "1" ? true : false;
                default:
                if(index > 6)
                {
                    if(index%2 == 0)
                    {
                        moves[moves.length - 1].y = Std.parseInt(value);
                    }else{
                        moves.push(new Point(Std.parseInt(value),0));
                    }
                }else{
                    throw("Player move parsing moves failed");
                }
            }
        }
    }
    public function equal(pos:Point,pos2:Point):Bool
    {
        if (pos.x == pos2.x && pos.y == pos2.y) return true;
        return false;
    }
    public function sub(pos:Point,pos2:Point):Point
    {
        var pos = new Point();
        pos.x = pos.x - pos2.x;
        pos.y = pos.y - pos2.y;
        return pos;
    }
    public function computePathSpeedMod():Float
    {
        var floorData = Game.data.objectMap.get(Game.data.map.floor.get(player.ix,player.iy));
        var multiple:Float = 1;
        if (floorData != null) multiple *= floorData.speedMult;
        if (player.oid.length == 0) return multiple;
        var objectData = Game.data.objectMap.get(player.oid[0]);
        if (objectData != null) multiple *= objectData.speedMult;
        return multiple;
    }
    public function measurePathLength():Float
    {
        var diagLength:Float = 1.4142356237;
        var totalLength:Float = 0;
        if (moves.length < 2)
        {
            return totalLength;
        }
        var lastPos = moves[0];
        for (i in 1...moves.length)
        {
            if (moves[i].x != lastPos.x && moves[i].y != lastPos.y)
            {
                totalLength += diagLength;
            }else{
                //not diag
                totalLength += 1;
            }
            lastPos = moves[i];
        }
        return totalLength;
    }
    public function movePos(x:Int,y:Int)
    {
        //10,10
        //0,0
        var currentX:Int = player.ix;
        var currentY:Int = player.iy;
        while (true)
        {
            if (!Game.data.blocking.get('$currentX.$currentY'))
            {
                //path find
                trace("blocking!");
            }

        }
    }
    public function movePlayer()
    {
        if (player == null) return;
        //player.ix = xs;
        //player.iy = ys;
        if (trunc) player.force();
        var currentX:Float = 0;
        var currentY:Float = 0;
        var pos:Point;
        for(i in 0...moves.length)
        {
            pos = new Point(moves[i].x - currentX,moves[i].y - currentY);
            trace('x ${pos.x} y ${pos.y}');
            currentX = moves[i].x;
            currentY = moves[i].y;
            moves[i] = pos;
        }
        time = 0;
        openfl.Lib.current.stage.removeEventListener(openfl.events.Event.ENTER_FRAME,update);
        openfl.Lib.current.stage.addEventListener(openfl.events.Event.ENTER_FRAME,update);
    }
    private function update(_)
    {
        if (moves.length == 0) return;
        if (frames > 0)
        {
            player.x += (point.x * Static.GRID)/time;
            player.y += -(point.y * Static.GRID)/time;
            frames--;
        }else{
            point = moves.shift();
            time = Std.int(Static.GRID/(Static.GRID * (player.instance.move_speed) * computePathSpeedMod()) * 60 * 1);
            player.ix += Std.int(point.x);
            player.iy += Std.int(point.y);
            frames = time;
        }
    }
}