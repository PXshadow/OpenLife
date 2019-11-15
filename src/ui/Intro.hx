package ui;

import openfl.events.Event;
import openfl.display.Stage;
import motion.easing.Quad;
import motion.Actuate;
import openfl.display.BitmapData;
import openfl.display.Bitmap;
import openfl.display.Sprite;
import openfl.Lib;
class Intro extends Screen
{
    var text:Text;
    var title:Text;
    public function new(stage:Stage)
    {
        super(stage);
        title = new Text("Open Life",CENTER,48,0xFFFFFF,stage.stageWidth);
        title.y = 10;
        title.bold = true;
        addChild(title);
        text = new Text("by PXshadow",CENTER,36,0xFFFFFF,stage.stageWidth);
        text.cacheAsBitmap = false;
        text.y = stage.stageHeight * 0.8;
        addChild(text);
        Actuate.tween(background,1,{alpha:0}).delay(1).ease(Quad.easeOut).onComplete(function()
        {
            remove();
        });
    }
}