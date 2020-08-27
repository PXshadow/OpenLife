package;

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
        Sys.println("(y)es (n)o relay system to use a client:");
        var relay:Bool = Sys.stdin().readLine() == "y";
        var bool:Bool = false;
        var data = Config.run(false);
        if (data.email == "") data = new Settings().cred();
        var seed:String = "";
        if (data.legacy && !relay)
        {
            Sys.println("set seed for 2HOL:");
            seed = Sys.stdin().readLine();
            if (seed.length > 0) seed = '|$seed';
        }
        if (!relay && FileSystem.exists("combo.txt"))
        {
            var lines = File.getContent("combo.txt").split("\r\n");
            var botamount = lines.length;
            Sys.println("(y)es (n)o deploy " + botamount + " bots:");
            var deploy:Bool = Sys.stdin().readLine() == "y";
            if (deploy)
            {
                Sys.println("spawning bots " + botamount);
                var bots:Array<Bot> = [];
                for (i in 0...botamount)
                {
                    var regex = ~/#.*/;
                    if(regex.match(lines[i]))
                        continue;
                    #if target.threaded
                    sys.thread.Thread.create(function()
                    {
                        var bot = new Bot(lines[i],data.ip + ":" + data.port,data.legacy,false,seed);
                        bot.connect(false,false);
                        bots.push(bot);
                        while (true)
                        {
                            bot.update();
                            Sys.sleep(1/40);
                        }
                    });
                    Sys.sleep(0.2);
                    //break;
                }
                #if hscript
                interp = new hscript.Interp();
                var parser = new hscript.Parser();
                interp.variables.set("bots",bots);
                while (true)
                {
                    try {
                        interp.execute(parser.parseString(Sys.stdin().readLine()));
                    }catch(e:Dynamic)
                    {
                        trace('e $e');
                    }
                }
                #else
                while (true)
                {
                    Sys.sleep(1/10);
                }
                #end
                #else
                throw "not threaded can not run multiple bots";
                #end
                return;
            }
        }
        var bot = new Bot(data.email + ":" + data.key,data.ip + ":" + data.port,data.legacy,relay,seed);
        bot.connect(false,relay);
        #if (hscript && target.threaded)
        interp = new hscript.Interp();
        var parser = new hscript.Parser();
        interp.variables.set("bot",bot);
        sys.thread.Thread.create(function()
        {
            while (true)
            {
                try {
                    interp.execute(parser.parseString(Sys.stdin().readLine()));
                }catch(e:Dynamic)
                {
                    trace('e $e');
                }
            }
        });
        #end
        while (true)
        {
            if(bot.resetFlag==true){
                bot = new Bot(data.email + ":" + data.key,data.ip + ":" + data.port,data.legacy,relay,seed);
                bot.connect(false,relay);
                interp.variables.set("bot",bot);
            }
            bot.update();
            Sys.sleep(1/40);
        }
    }
    public static function getFields(Object:Dynamic):Array<String>
    {
        var fields = [];
        if ((Object is Class)) // passed a class -> get static fields
            fields = Type.getClassFields(Object);
        else if ((Object is Enum))
            fields = Type.getEnumConstructs(Object);
        else if (Reflect.isObject(Object)) // get instance fields
            fields = Type.getInstanceFields(Type.getClass(Object));

        // on Flash, enums are classes, so Std.is(_, Enum) fails
        fields.remove("__constructs__");

        var filteredFields = [];
        for (field in fields)
        {
            // don't add property getters / setters
            if (field.startsWith("get_") || field.startsWith("set_"))
            {
                var name = field.substr(4);
                // property without a backing field, needs to be added
                if (!fields.contains(name) && !filteredFields.contains(name))
                    filteredFields.push(name);
            }
            else
                filteredFields.push(field);
        }

        return filteredFields;
    }
}