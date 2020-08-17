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
        Engine.dir = "OneLifeData7/";
        vector = Bake.run();
        trace("baked chisel: " + ObjectBake.dummies.get(455));
        //start program
        Sys.println("(y)es (n)o relay system to use a client");
        var relay:Bool = Sys.stdin().readLine() == "y";
        var bool:Bool = false;
        if (!relay && FileSystem.exists("combo.txt"))
        {
            var lines = File.getContent("combo.txt").split("\n");
            Sys.println("(y)es (n)o deploy " + lines.length + " bots");
            var deploy:Bool = Sys.stdin().readLine() == "y";
            if (deploy)
            {
                Sys.println("spawning bots");
                return;
            }
        }
        var data = Config.run(true);
        if (data.email == "") data = new Settings().cred();
        var bot = new Bot(data.email + ":" + data.key,data.ip + ":" + data.port,relay);
        bot.connect(false,relay);
        #if (hscript && target.threaded)
        interp = new hscript.Interp();
        var parser = new hscript.Parser();
        interp.variables.set("program",bot.program);
        interp.variables.set("map",bot.map);
        interp.variables.set("app",this);
        interp.variables.set("client",bot.client);
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
            bot.update();
            Sys.sleep(1/30);
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