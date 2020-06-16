package;

import sys.io.File;
import haxe.Json;
import openlife.data.map.MapInstance;
import openlife.engine.Engine;
import sys.FileSystem;

class App extends Engine
{
    public function new()
    {
        super();
        var bool:Bool = false;
        var config:Cred;
        if (FileSystem.exists("cred"))
        {
            Sys.println("Use existing cred config (y)es (n)o");
            bool = Sys.stdin().readLine() == "n" ? false : true;
            if (bool)
            {
                config = cast Json.parse(File.getContent("cred"));
                client.ip = config.ip;
                client.port = config.port;
                client.legacy = config.legacy;
                client.email = config.email;
                client.key = config.key;
            }
        }
        if (!bool)
        {
            Sys.println("Legacy authentication (y)es (n)o");
            client.legacy = Sys.stdin().readLine() == "y" ? true : false;
            //set credentioals
            if (!cred())
            {
                Sys.println("ip:");
                string = Sys.stdin().readLine();
                client.ip = string;
                Sys.println("port:");
                string = Sys.stdin().readLine();
                client.port = Std.parseInt(string);
                Sys.println("email:");
                string = Sys.stdin().readLine();
                client.email = string;
                Sys.println("key");
                string = Sys.stdin().readLine();
                client.key = string;
                //set config
                config = {email: client.email, key: client.key,ip: client.ip,port: client.port,legacy: client.legacy};
                File.saveContent("cred",Json.stringify(config));
            }
        }
        connect(false);
        while (true)
        {
            client.update();
            Sys.sleep(1/30);
        }
    }
    override function mapChunk(instance:MapInstance) {
        super.mapChunk(instance);
        trace("instance " + instance.toString());
    }
}
typedef Cred = {legacy:Bool,email:String,key:String,ip:String,port:Int}