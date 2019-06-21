import openfl.display.Bitmap;
import openfl.display.Shape;
import lime.ui.FileDialog;
import openfl.net.URLRequest;
import openfl.display.Sprite;
import format.SVG;
import openfl.Assets;
import openfl.events.MouseEvent;
class Launcher extends Sprite
{
    public function new()
    {
        super();
        var item = new Item();
        item.x = 58 + 55 * 0;
        item.y = 94 - 20 + 352 * 0;

        addChild(item);
    }
}
class Item extends Sprite
{
    var action:Shape;
    var actionBool:Bool = false;
    var title:Text;
    var desc:Text;
    var profile:Bitmap;
    public function new()
    {
        super();
        cacheAsBitmap = true;
        //rect
        graphics.beginFill(0x292929);
        graphics.drawRect(0,0,250,100);
        graphics.beginFill(0x121212);
        graphics.drawRect(0,100,250,200);
        //top buttons
        new SVG(Assets.getText("assets/bookmark.svg")).render(graphics,0 + 4,0 + 4,20,26);
        new SVG(Assets.getText("assets/settings.svg")).render(graphics,230 - 4,4,20,20);
        //profile
        profile = new Bitmap(Assets.getBitmapData("assets/profile.jpg"));
        profile.x = 85;
        profile.y = 10;
        profile.width = 80;
        profile.height = 80;
        addChild(profile);
        //text
        title = new Text("Community Crucible",CENTER,20,0xFFFFFF,250);
        title.y = 118;
        addChild(title);

        desc = new Text(
        "• Community contributed viable solutions.\n" +
        "• Community backed and focused\n" +
        "• new content and mechanics\n"
        ,LEFT,12,0xFFFFFF,220);
        desc.x = 15;
        desc.y = 150;
        desc.height = 100;
        addChild(desc);


        new SVG(Assets.getText("assets/link.svg")).render(graphics,15,268);
        new SVG(Assets.getText("assets/code.svg")).render(graphics,55,272);
        new SVG(Assets.getText("assets/note.svg")).render(graphics,163,272);
        new SVG(Assets.getText("assets/folder.svg")).render(graphics,217,272);

        action = new Shape();
        action.x = 115 - 3;
        action.y = 263;
        addChild(action);
        new SVG(Assets.getText("assets/play.svg")).render(action.graphics,0,0);
        //new SVG(Assets.getText("assets/download.svg")).render(action.graphics,-8,0);

        addEventListener(MouseEvent.MOUSE_MOVE,function(_)
        {
            if(mouseY > 260)
            {
                buttonMode = true;
            }else{
                buttonMode = false;
            }
        });
        var pannelX:Array<Int> = [46,95,150,198,250];
        //create lines
        graphics.lineStyle(1,0x979797);
        for(i in 0...pannelX.length - 1)
        {
            graphics.moveTo(pannelX[i],260);
            graphics.lineTo(pannelX[i],296);
        }
        graphics.endFill();
        addEventListener(MouseEvent.CLICK,function(_)
        {
            if(mouseY > 260)
            {
                for(i in 0...pannelX.length)
                {
                    if(mouseX < pannelX[i])
                    {
                        switch(i)
                        {
                            case 0:
                            //link
                            openfl.Lib.navigateToURL(new URLRequest("https://duckduckgo.com"));
                            case 1:
                            //source
                            openfl.Lib.navigateToURL(new URLRequest("https://github.com"));
                            case 2:
                            //play
                            action.graphics.clear();
                            if(actionBool)
                            {
                                new SVG(Assets.getText("assets/play.svg")).render(action.graphics,0,0);
                            }else{
                                new SVG(Assets.getText("assets/download.svg")).render(action.graphics,-2,0);
                            }
                            actionBool = !actionBool;
                            case 3:
                            //notes

                            case 4:
                            //folder
                            //new FileDialog().browse();
                            lime.system.System.openFile(lime.system.System.applicationDirectory);
                        }
                        return;
                    }
                }
            }
        });
    }
}