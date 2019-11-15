package ui;

import openfl.Lib;
import openfl.display.Shape;
import openfl.display.Sprite;

class Loader extends Screen
{
    public var task:String;
    var text:Text;
    var bar:Shape;
    public function new()
    {
        super();
        text = new Text("",CENTER,30,0xFFFFFF,stage.stageWidth);
        addChild(text);
    }
    public function update(percent:Float)
    {
        text.text = task + " " + Std.string(Math.floor(percent * 100)) + "%";
    }
}