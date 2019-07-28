/*package states.game;
import haxe.Timer;
import haxe.ds.Vector;
import motion.Actuate;
import openfl.display.Tile;
import data.AnimationData;
import data.ObjectData;
import openfl.display.TileContainer;
class Object extends TileContainer
{
    public var oid:Int;
    public var animation:AnimationData;
    public var map:Map<Int,Vector<Int>> = new Map<Int,Vector<Int>>();
    public var parents:Map<Int,TileContainer> = new Map<Int,TileContainer>();
    public var numSprites:Int = 0;
    public var tileX:Int = 0;
    public var tileY:Int = 0;
    public var type:ObjectType = OBJECT;
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
    public function animate(index:Int)
    {
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
            var time = new Timer(param.durationSec * 1000);
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
            Actuate.resume(tile);
        }
    }
    public function add(tile:Tile,i:Int,p:Int)
    {
        numSprites++;
        var vector:Vector<Int>;
        //p = -1;
        if (p == -1)
        {
            vector = new Vector<Int>(1);
            addTile(tile);
            vector[0] = getTileIndex(tile);
        }else{
            vector = new Vector<Int>(2);
            var parent = parents.get(p);
            if (parent == null)
            {
                parent = new TileContainer();
                parents.set(p,parent);
                addTile(parent);
            }else{
                //trace("parent already exists " + p);
            }
            vector[0] = getTileIndex(parent);
            if (i == p)
            {
                //tile is parent
                parent.x = tile.x;
                parent.y = tile.y;
                parent.rotation = tile.rotation;
                parent.alpha = tile.alpha;
                parent.colorTransform = tile.colorTransform;

                tile.x = 0;
                tile.y = 0;
                tile.rotation = 0;
                tile.alpha = 0;
                tile.colorTransform = null;
            }
            parent.addTile(tile);
            vector[1] = parent.getTileIndex(tile);
        }
        map.set(i,vector);
    }
    public function get(index:Int):Tile
    {
        var vector = map.get(index);
        if (vector.length == 1)
        {
            return getTileAt(vector[0]);
        }else{
            //parent
            return cast(getTileAt(vector[0]),TileContainer).getTileAt(vector[1]);
        }
    }

}*/