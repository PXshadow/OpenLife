package states.launcher;
import haxe.io.Path;
import sys.io.File;
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
    var assets:AssetLoader;
    public static var dir:String;
    public function new()
    {
        super();
        assets = new AssetLoader();
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
        //mods
        if (FileSystem.isDirectory(dir + "/groundTileCache"))
        {
            //portable launch straight away
            Main.state.remove();
            Main.state = new states.game.Game();
        }else{
            //check for mod json
            if (FileSystem.isDirectory(dir + "mods"))
            {
                var ext:String = "";
                var data:Dynamic;
                for (path in FileSystem.readDirectory(dir + "mods"))
                {
                    //mac remove .DS_STORE
                    if (path.substring(0,1) == ".") continue;
                    path = "mods/" + path;
                    ext = Path.extension(path);
                    if (ext == "json")
                    {
                        //this is a project file
                        data = Json.parse(File.read(dir + path,false).readAll().toString());
                        //remove old json
                        FileSystem.deleteFile(dir + path);
                        path = dir + "mods/" + data.title + "/";
                        //new folder
                        FileSystem.createDirectory(path);
                        //new project.json
                        File.write(path + "project.json",false).writeString(Json.stringify(data));
                    }else{
                        if (ext == "") return;
                        //get project file in folder
                        path = dir + path + "/";
                        data = Json.parse(File.read(path + "project.json").readAll().toString());
                    }
                    var item = new Item(data);
                    item.Click = function(_)
                    {
                        if(mouseY > 250)
                        {
                            //bottom section
                            if (item.path != "")
                            {
                                //play
                                dir = item.data.dir;
                                Main.state.remove();
                                Main.state = new states.game.Game();
                            }else{
                                //download
                                if(!FileSystem.isDirectory(path + "assets") && data.assets != "")
                                {
                                    trace("new assets");
                                    assets.complete = function(sucess:Bool)
                                    {
                                        if(sucess)
                                        {
                                            item.path = path + "assets";
                                            Launcher.dir = item.path;
                                            item.bottom.text = item.path;
                                            Main.state.remove();
                                            Main.state = new states.game.Game();
                                        }else{
                                            trace("fail");
                                        }
                                    }
                                    assets.progress = function(loaded:Float,complete:Float)
                                    {
                                        trace(loaded + "/" + complete);
                                    }  
                                    assets.loader(data.assets,path + "assets");
                                }
                                if(!FileSystem.isDirectory(path + "scripts") && data.scripts != "")
                                {
                                    //todo make it so scripts can be embeded in the project.json
                                }   
                                if(!FileSystem.isDirectory(path + "/settings"))
                                {
                                    //populate settings locally in the future
                                }
                            }
                        }
                    }
                    addChild(item);
                }
            }else{
                //no mod folder
                trace("no mod folder");
            }
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
            //case Keyboard.UP | Keyboard.W | Keyboard.PAGE_UP:

            //case Keyboard.DOWN | Keyboard.DOWN | Keyboard.PAGE_DOWN:

            //case Keyboard.SPACE | Keyboard.ENTER:

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