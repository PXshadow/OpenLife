package states.game;
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
    public function animate()
    {
        animation = new AnimationData(oid);
        if (animation.fail) 
        {
            animation = null;
            return;
        }
        for (record in animation.record)
        {
            trace("record " + record);
        }
        
    }
}