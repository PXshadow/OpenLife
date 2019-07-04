package states.launcher;

import openfl.display.Sprite;
import ui.Text;

class Item extends ui.Button
{
    public var info:Text;
    public var text:Text;
    private static var ix:Int = 0;
    private static var iy:Int = 0;
    public function new(titleString:String,descString:String)
    {
        super();
        //pos
        x = 225 + (250 + 40) * ix;
        y = 100 + 300 * iy;
        ix++;
        if (ix >= 3) 
        {
            ix = 0;
            iy += 1;
        }
        //rect
        graphics.beginFill(0x121212);
        graphics.drawRoundRect(0,0,250,250,24,24);
        graphics.beginFill(0x21307D);
        graphics.drawRoundRectComplex(0,224,250,34,0,0,24,24);
        graphics.endFill();
        //line
        graphics.lineStyle(1,0xD8D8D8);
        graphics.moveTo(15,145);
        graphics.lineTo(235,145);
        //text
        var title = new Text(titleString,CENTER,20,0xE0E0E0,220);
        title.x = 15;
        title.y = 2;
        title.cacheAsBitmap = false;
        addChild(title);
        var desc = new Text(descString,CENTER,12,0xE0E0E0,220);
        desc.x = 15;
        desc.y = 35;
        desc.cacheAsBitmap = false;
        addChild(desc);

        info = new Text("...",CENTER,12,0xE0E0E0,220);
        info.x = 15;
        info.y = 158;
        info.cacheAsBitmap = false;
        addChild(info);

        text = new Text("Play",CENTER,20,0xE0E0E0,250);
        text.y = 227;
        text.cacheAsBitmap = false;
        addChild(text);
    }
}