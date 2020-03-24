package game;
#if openfl
import openfl.display.Sprite;
import openfl.display.Tilemap;
import openfl.display.Shape;
#if nativeGen @:nativeGen #end
class Ui
{
    var sprite:Sprite;
    var objects:Objects;
    public function new(sprite:Sprite,objects:Objects)
    {
        this.sprite = sprite;
        this.objects = objects;
    }
}
#end