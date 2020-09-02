package;

import openlife.auto.Script;
import haxe.Exception;
import openlife.client.Client;
import haxe.Json;
import sys.io.File;
import openlife.data.object.ObjectData;
import haxe.ds.Vector;
import openlife.auto.Automation;
import sys.FileSystem;
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
    public static var vector:Vector<Int>;
    var followingId:Int = -1;
    public function new()
    {
        Engine.dir = Utility.dir();
        vector = Bake.dummies();
        trace("baked chisel: " + ObjectBake.dummies.get(455));
        //start program
        var config:ConfigData = {relay: true,combo: 0,syncSettings: false,script: "Script.hx"};
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
        trace("config: " + config);
        if (!config.relay && config.combo > 0)
        {
            //multiple bots from combo
            if (!FileSystem.exists("combo.txt")) throw "no combo list found";
            var list = File.getContent("combo.txt").split("\n");
            var bots:Array<Bot> = [];
            for (account in list)
            {
                var cred = credClone(cred);
                var data = account.split(":");
                cred.email = data[0];
                cred.key = data[1];
                var client = new Client();
                client.cred = cred;
                var bot = new Bot(client);
                bot.connect(false,false);
                bots.push(bot);
                Sys.sleep(0.2);
            }
            while (true)
            {
                for (bot in bots) bot.update();
                Sys.sleep(1/40);
            }
        }else{
            var client = new Client();
            client.cred = cred;
            var bot = new Bot(client);
            bot.relayPort = 8000;
            #if hscript
            var interp = new hscript.Interp();
            var parser = new hscript.Parser();
            var runTime:Int = 20 * 4;
            var runTicks:Int = 0;
            parser.allowTypes = true;
            interp.variables.set("bot",bot);
            #end
            bot.connect(false,config.relay);
            while (true) 
            {
                bot.update();
                #if hscript
                if (runTicks > runTime)
                {
                    var script = Script.execute();
                    if (script.length > 0)
                    {
                        Sys.println("Executing Script.hx");
                        if (bot.auto == null) return;
                        try {
                            interp.execute(parser.parseString(script));
                        }catch(e:Exception)
                        {
                            Sys.println(e.details());
                        }
                    }
                    runTicks = 0;
                }
                runTicks++;
                #end
                Sys.sleep(1/20);
            }
        }
    }
    private function credClone(cred:CredData):CredData
    {
        return {email: cred.email, key: cred.key, ip: cred.ip, port: cred.port, tutorial: cred.tutorial, seed: cred.seed, twin: cred.twin,legacy: cred.legacy};
    }
}
typedef ConfigData = {relay:Bool,combo:Int,syncSettings:Bool,script:String}
