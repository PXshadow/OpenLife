package states.game;
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
        trace("animate " + index);
        var record = animation.record[index];
        var param:AnimationParameter;
        var tile:Tile;
        trace("type " + record.type + " record " + record.numSprites);
        for(i in 0...record.numSprites)
        {
            param = record.params[i];
            tile = get(i);
            Actuate.pause(tile);
            if (param.offset != null)
            {
                //trace("offset " + param.offset.x + " " + param.offset.y);
                //tile.x += param.offset.x;
                //tile.y += param.offset.y;
            }
            //tile.originX = param.rotationCenterOffset.x;
            //tile.originY = param.rotationCenterOffset.y;

            Actuate.tween(tile,0.2,{x:tile.x + param.xOscPerSec},false).repeat().reflect();
            Actuate.tween(tile,0.2,{y:tile.y + param.yOscPerSec},false).repeat().reflect();
        }
    }
    public function add(tile:Tile,i:Int,p:Int)
    {
        var vector:Vector<Int>;
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
                addTile(parent);
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

}