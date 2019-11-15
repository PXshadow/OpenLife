package ui;

import openfl.display.Stage;
import openfl.events.Event;
import openfl.display.Bitmap;
import openfl.display.BitmapData;
import openfl.Lib;
import openfl.display.Sprite;

class Screen extends Sprite
{
    var background:Bitmap;
    public function new(stage:Stage)
    {
        super();
        stage.addChild(this);
        background = new Bitmap(new BitmapData(stage.stageWidth,stage.stageHeight,false,0));
        addChild(background);
        stage.addEventListener(Event.RESIZE,resize);
    }
    private function resize(_)
    {
        background.width = stage.stageWidth;
        background.height = stage.stageHeight;
    }
    public function set()
    {
        background = new Bitmap(new BitmapData(stage.stageWidth,stage.stageHeight,false,0));
        addChild(background);
    }
    public function remove()
    {
        stage.removeEventListener(Event.RESIZE,resize);
        stage.removeChild(this);
    }
}