package states.game;
import motion.Actuate;
import openfl.display.Tile;
import data.AnimationData;
import data.ObjectData;
import openfl.display.TileContainer;
class Object extends TileContainer
{
    public var oid:Int;
    public var animation:AnimationData;
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
        for(i in 0...record.numSprites)
        {
            param = record.params[i];
            tile = getTileAt(i);
            tile.originX = param.rotationCenterOffset.x;
            tile.originY = param.rotationCenterOffset.y;
            
        }
    }

}