package;

import haxe.Json;
import sys.io.File;
import openlife.data.object.ObjectData;
import haxe.ds.Vector;
import openlife.auto.Automation;
import sys.FileSystem;
import openlife.client.Relay;
import openlife.resources.ObjectBake;
import openlife.settings.Settings;
import haxe.ds.IntMap;
import openlife.data.object.player.PlayerInstance;
import openlife.engine.Program;
import openlife.data.map.MapInstance;
import openlife.engine.*;
import openlife.data.object.player.PlayerMove;
import openlife.data.map.MapChange;
using StringTools;

class App
{
    #if hscript var interp:hscript.Interp; #end
    var auto:Automation;
    public static var vector:Vector<Int>;
    var followingId:Int = -1;
    public function new()
    {
        Engine.dir = Utility.dir();
        vector = Bake.dummies();
        trace("baked chisel: " + ObjectBake.dummies.get(455));
        //start program
        var config:ConfigData = {relay: false,combo: true,syncSettings: false};
        var cred:CredData = new Settings().cred();
        if (!FileSystem.exists("cred.json") || config.syncSettings)
        {
            File.saveContent("cred.json",Json.stringify(cred));
        }else{
            cred = Json.parse(File.getContent("cred.json"));
        }
        if (!FileSystem.exists("config.json"))
        {
            File.saveContent("config.json",Json.stringify(config));
        }else{
            config = Json.parse(File.getContent("config.json"));
        }
        
    }
}
typedef ConfigData = {relay:Bool,combo:Bool,syncSettings:Bool}
