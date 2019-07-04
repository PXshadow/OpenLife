package states.launcher;
import sys.FileSystem;
import ui.Button;
import haxe.Json;
import openfl.ui.Keyboard;
import openfl.display.Bitmap;
import openfl.display.Shape;
import lime.ui.FileDialog;
import openfl.net.URLRequest;
import openfl.display.Sprite;
import format.SVG;
import openfl.Assets;
import openfl.events.MouseEvent;
import haxe.Http;
import ui.Text;
class Launcher extends states.State
{
    var updateBanner:Button;
    var updateBannerRect:Shape;
    var updateBannerText:Text;
    public static var dir:String;
    public function new()
    {
        super();
        //patch banner
        patch(function(bool:Bool)
        {
            if (!bool) return;
            //banner for update
            updateBanner = new Button();
            updateBanner.Click = function(_)
            {
                updateBanner.visible = false;
                openfl.Lib.navigateToURL(new openfl.net.URLRequest("https://pxshadow.itch.io/openlife"));
            }
            updateBannerRect = new Shape();
            updateBannerRect.graphics.beginFill(0x22AB2D);
            updateBannerRect.graphics.drawRect(0,0,Main.setWidth,60);
            addChild(updateBannerRect);
            updateBanner.graphics.beginFill(0,0);
            updateBanner.graphics.drawRect(0,0,updateBannerRect.width,updateBannerRect.height);
            updateBannerText = new Text("NEW UPDATE",CENTER,20,0xE0E0E0,Main.setWidth);
            updateBannerText.y = 16;
            updateBannerText.cacheAsBitmap = false;
            addChild(updateBannerText);
            addChild(updateBanner);
        });
        //figure out directory
        dir = lime.system.System.applicationDirectory;
        #if mac
        dir = dir.substring(0,dir.indexOf("/Contents/Resources/"));
        dir = dir.substring(0,dir.lastIndexOf("/") + 1);
        #end
        
        if (FileSystem.isDirectory(dir + "/groundTileCache"))
        {
            //portable
        }else{
            //check for mod json
        }
    }
    private function patch(finish:Bool->Void)
    {
        var channel:String = "";
        #if windows
        channel = win32-beta
        #elseif linux
        channel = "linux-universal";
        #elseif mac
        channel = "mac-os";
        #else
        finish(false);
        return;
        #end
        var http = new Http("https://itch.io/api/1/x/wharf/latest?target=pxshadow/openlife&channel_name=" + channel);
        http.onData = function(string:String)
        {
            var data = Json.parse(string);
            var errors:Array<String> = data.errors;
            if (errors.length > 0)
            {
                trace("error " + errors);
                finish(false);
            }else{
                //no errors
            }
        }
        http.onError = function(error:String)
        {
            trace("patch get error " + error);
            finish(false);
        }
        http.request(false);
    }
    override function update() 
    {
        super.update();
    }
    override function keyDown(code:Int) 
    {
        super.keyDown(code);
        switch(code)
        {
            case Keyboard.UP | Keyboard.W | Keyboard.PAGE_UP:

            case Keyboard.DOWN | Keyboard.DOWN | Keyboard.PAGE_DOWN:

            case Keyboard.SPACE | Keyboard.ENTER:

        }
    }
    override public function resize()
    {
        if (updateBanner != null) 
        {
            updateBannerRect.width = stage.stageWidth/Main.screen.scaleX;
            updateBannerText.width = updateBannerRect.width;
            updateBanner.y = -Main.screen.y/Main.screen.scaleY;
        }

    }
}