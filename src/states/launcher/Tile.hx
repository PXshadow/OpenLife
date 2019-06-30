package states.launcher;
import openfl.Assets;
import openfl.display.Sprite;
import ui.Text;
import openfl.display.Shape;
import openfl.display.Bitmap;
import format.SVG;
import openfl.events.MouseEvent;
import openfl.net.URLRequest;
class Tile extends Sprite
{
    var action:Shape;
    var actionBool:Bool = false;
    var title:Text;
    var desc:Text;
    var profile:Bitmap;
    var pannelX:Array<Int> = [];
    var drag:Bool = false;
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
        loadShape("assets/ui/bookmark.svg",0 + 4,0 + 4,20,26);
        loadShape("assets/ui/settings.svg",230 - 4,4,20,20);
        //profile

        profile = new Bitmap();
        profile.x = 85;
        profile.y = 10;
        profile.width = 80;
        profile.height = 80;
        addChild(profile);
        //text
        title = new Text("2HOL Reborn",CENTER,20,0xFFFFFF,250);
        title.y = 118;
        addChild(title);

        desc = new Text("",LEFT,12,0xFFFFFF,220);
        desc.x = 15;
        desc.y = 150;
        desc.height = 100;
        for(string in 
        [
            "- anti griefing policy",
            "- community based updates from suggestions",
            "- active team with frequent small updates",
            "- relatively new server with lots of space to grow",
            "- looking for new team members as well as players!"
        ])
        {
            desc.appendText(string + "\n");
        }
        desc.text = desc.text.substring(0,desc.text.length - 2);
        addChild(desc);


        loadShape("assets/ui/link.svg",15,268);
        loadShape("assets/ui/code.svg",55,272);
        loadShape("assets/ui/note.svg",163,272);
        loadShape("assets/ui/folder.svg",217,272);

        action = new Shape();
        action.x = 115 - 3;
        action.y = 263;
        addChild(action);
        loadAction("assets/ui/play.svg");

        pannelX = [46,95,150,198,250];
        //create lines
        graphics.lineStyle(1,0x979797);
        for(i in 0...pannelX.length - 1)
        {
            graphics.moveTo(pannelX[i],260);
            graphics.lineTo(pannelX[i],296);
        }
        graphics.endFill();
        addEventListener(MouseEvent.CLICK,click);
        addEventListener(MouseEvent.MOUSE_DOWN,down);
        addEventListener(MouseEvent.MOUSE_UP,up);
        addEventListener(MouseEvent.MOUSE_MOVE,move);
    }
    private function move(_)
    {
        if(mouseY > 260)
        {
            buttonMode = true;
        }else{
            buttonMode = false;
        }
    }
    private function down(_)
    {

    }
    private function up(_)
    {

    }
    private function click(_)
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
                            loadAction("assets/ui/play.svg",0,0);
                        }else{
                            loadAction("assets/ui/download.svg",-2,0);
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
    }
    public function remove()
    {
        removeEventListener(MouseEvent.MOUSE_DOWN,down);
        removeEventListener(MouseEvent.MOUSE_UP,up);
        removeEventListener(MouseEvent.CLICK,click);
    }
    public function loadShape(path:String,x:Int,y:Int,width:Int=-1,height:Int=-1)
    {
        Assets.loadText(path).onComplete(function(string:String)
        {
            new SVG(string).render(graphics,x,y,width,height);
        });
    }
    public function loadAction(path:String,x:Int=0,y:Int=0)
    {
        Assets.loadText(path).onComplete(function(string:String)
        {
            new SVG(string).render(action.graphics,x,y);
        });
    }
}