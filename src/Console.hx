import openfl.display.Shape;
import openfl.display.DisplayObjectContainer;

class Console extends DisplayObjectContainer
{
    var input:Text;
    var output:Text;
    var shape:Shape;
    public function new()
    {
        super();
        shape = new Shape();
        shape.cacheAsBitmap = true;
    }
    public function resize()
    {
        shape.graphics.clear();
        shape.graphics.beginFill(0);
        shape.graphics.drawRect(0,0,stage.stageWidth,296);
        shape.graphics.moveTo(0,260);
        shape.graphics.lineTo(stage.stageWidth,260);
    }
}