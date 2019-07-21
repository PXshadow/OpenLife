package states.launcher;
import openfl.events.Event;
import haxe.io.Path;
import sys.io.File;
import sys.FileSystem;
import haxe.Json;
import openfl.ui.Keyboard;
import openfl.display.Bitmap;
import openfl.display.Shape;
import lime.ui.FileDialog;
import openfl.net.URLRequest;
import openfl.display.Sprite;
import openfl.Assets;
import openfl.events.MouseEvent;
import ui.Button;
import haxe.Http;
import ui.Text;
class Launcher extends states.State
{
    var updateBanner:Button;
    var updateBannerRect:Shape;
    var updateBannerText:Text;
    var assets:AssetLoader;
    var items:Array<Item> = [];
    var launch:Bool = false;
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
        Static.dir = lime.system.System.applicationDirectory;
        #if mac
        Static.dir = Static.dir.substring(0,Static.dir.indexOf("/Contents/Resources/"));
        Static.dir = Static.dir.substring(0,Static.dir.lastIndexOf("/") + 1);
        #end
        //mods
        if (FileSystem.isDirectory(Static.dir + "/groundTileCache"))
        {
            //portable, launch straight away
            launch = true;
        }else{
            //check for mod json
            if (FileSystem.isDirectory(Static.dir + "mods"))
            {
                var ext:String = "";
                var data:Dynamic;
                for (path in FileSystem.readDirectory(Static.dir + "mods"))
                {
                    //mac remove .DS_STORE
                    if (path.substring(0,1) == ".") continue;
                    path = "mods/" + path;
                    ext = Path.extension(path);
                    if (ext == "json")
                    {
                        //this is a project file
                        data = Json.parse(File.read(Static.dir + path,false).readAll().toString());
                        //remove old json
                        FileSystem.deleteFile(Static.dir + path);
                        path = Static.dir + "mods/" + data.title + "/";
                        //new folder
                        FileSystem.createDirectory(path);
                        //new project.json
                        writeJson(path,data);
                    }else{
                        if (ext != "") return;
                        //get project file in folder
                        path = Static.dir + path + "/";
                        data = Json.parse(File.read(path + "project.json").readAll().toString());
                    }
                    trace("add item");
                    var item = new Item(data);
                    if (FileSystem.isDirectory(path + "groundTileCache"))
                    {
                        //mod already exists check update
                        item.bottom.text = "Play";
                        item.info.text = "Folder";
                        item.path = path;
                        item.playable = true;
                    }
                    item.path = path;
                    item.version = data.version;
                    item.info.text = item.version;
                    item.Click = function(_)
                    {
                        if(mouseY > 250)
                        {
                            //bottom section
                            if (item.playable)
                            {
                                //play
                                Static.dir = item.path;
                                Main.state.remove();
                                Main.state = new states.game.Game();
                            }else{
                                //download
                                download(item.path,item);
                            }
                        }else{
                            if(path != "")
                            {
                                lime.system.System.openFile(path);
                            }
                        }
                    }
                    addChild(item);
                    items.push(item);
                }
            }else{
                //no mod folder
                trace("no mod folder " + Static.dir);
            }
        }
    }
    override function init(_:Event) {
        super.init(_);
        if(launch)
        {
            Main.state.remove();
            Main.state = new states.game.Game();
        }
    }
    private function writeJson(path:String,data:Dynamic)
    {
        File.write(path + "project.json",false).writeString(Json.stringify(data));
    }
    private function download(path:String,item:Item)
    {
        if(!FileSystem.isDirectory(path + "groundTileCache"))
        {
            assets.complete = function(sucess:Bool)
            {
                if(sucess)
                {
                    item.path = path;
                    Static.dir = item.path;
                    item.info.text = item.version;
                    item.bottom.text = "Play";
                    item.mouseEnabled = true;
                    item.playable = true;
                    //write project json version
                    item.data.version = item.version;
                    writeJson(path,item.data);
                }else{
                    trace("fail");
                    item.mouseEnabled = true;
                }
            }
            assets.progress = function(loaded:Float,complete:Float)
            {
                item.info.text = Math.round(loaded/(complete == 0 ? 48000000 : complete) * 100) + "%\nDownloading";
            }
            assets.update = function(version:String)
            {
                trace("version mismatch " + item.version + " " + version + " updating... " + path);
                item.version = version;
            }
            item.mouseEnabled = false;
            assets.loader(item.data.assets,path,item.version);
        }
        if(!FileSystem.isDirectory(path + "scripts") && item.data.scripts != "")
        {
            //todo make it so scripts can be embeded in the project.json
        }   
        if(!FileSystem.isDirectory(path + "/settings"))
        {
            //populate settings locally in the future
        }
    }
    private function patch(finish:Bool->Void)
    {
        var channel:String = "";
        #if windows
        channel = "win32-beta";
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