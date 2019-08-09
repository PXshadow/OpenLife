package states.game;
import haxe.Timer;
import haxe.ds.Vector;
import motion.Actuate;
import openfl.display.Tile;
import data.AnimationData;
import data.ObjectData;
import openfl.display.TileContainer;
class Object extends TileContainer
{
    // base speed for animations that aren't speeded up or slowed down
    // when player moving at a different speed, anim speed is modified
    private static inline var BASE_SPEED:Float = 3.75;
    //object id
    public var oid:Int;
    public var animation:AnimationData;
    public var map:Map<Int,Int> = new Map<Int,Int>();
    public var parents:Map<Int,TileContainer> = new Map<Int,TileContainer>();
    public var numSprites:Int = 0;
    //local map postion
    public var tileX:Int = 0;
    public var tileY:Int = 0;
    public var type:ObjectType = OBJECT;
    //mark for cleanup
    public var clean:Bool = false;
    public function new()
    {
        super();
    }
    public function loadAnimation()
    {
        animation = new AnimationData(oid);
        if (animation.fail) 
        {
            animation = null;
            return;
        }
    }
    public function pos()
    {
        //local position
        x = tileX * Static.GRID;
        y = (Static.tileHeight - tileY) * Static.GRID;
    }
    public function animate(index:Int)
    {
        return;
        //trace("animate " + index);
        if (animation == null || animation.record == null) 
        {
            //trace("no records for animation");
            return;
        }
        var record = animation.record[index];
        var param:AnimationParameter;
        var tile:Tile;
        //trace("type " + record.type + " record " + record.numSprites);
        for(i in 0...record.numSprites)
        {
            param = record.params[i];
            tile = get(i);
            if (tile == null) continue;
            Actuate.stop(tile);
            //x
            if (param.xOscPerSec > 0)
            {
                tile.x += param.xPhase - param.xAmp/2;
                Actuate.tween(tile,1/param.xOscPerSec,{x:param.xPhase + param.xAmp/2},false).repeat(5).reflect();
            }
            if (param.yOscPerSec > 0)
            {
                tile.x += param.yPhase - param.xAmp/2;
                Actuate.tween(tile,1/param.yOscPerSec,{x:param.yPhase + param.yAmp/2},false).repeat(5).reflect();
            }
            if (param.rockOscPerSec > 0)
            {
                tile.rotation = (param.rockPhase - param.rockAmp/2) * 360;
                Actuate.tween(tile,1/param.rockOscPerSec,{rotation:(param.rockPhase + param.rockAmp/2) * 360},false).repeat(5).reflect();
            }
            /*var time = new Timer(param.durationSec * 1000);
            time.run = function()
            {
                Actuate.pause(tile);
                time.stop();
                time = new Timer(param.pauseSec * 1000);
                time.run = function()
                {
                    Actuate.resume(tile);
                    time.stop();
                }
            }
            Actuate.resume(tile);*/
        }
    }
    public function add(tile:Tile,i:Int,p:Int)
    {
        //i is the index p is the parent object
        if (p >= -1)
        {
            map.set(i,getTileIndex(addTile(tile)));
        }
    }
    public function get(index:Int):Tile
    {
        return getTileAt(map.get(index));
    }

}