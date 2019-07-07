package states.game;
import data.ObjectData;
import openfl.display.TileContainer;
class Object extends TileContainer
{
    var index:Int;
    public function new(index:Int)
    {
        this.index = index;
        super();
        var data = new ObjectData(index);
        for(i in 0...data.numSprites)
        {

        }
    }
}