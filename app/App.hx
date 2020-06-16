package;

import openlife.data.map.MapInstance;
import openlife.engine.Engine;

class App extends Engine
{
    public function new()
    {
        super();
        Sys.println("Legacy authentication (y)es (n)o");
        client.legacy = Sys.stdin().readLine() == "n" ? true : false;
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

            settings.data.set("email",client.email);
            settings.data.set("accountKey",client.key);
            settings.data.set("useCustomServer","1");
            settings.data.set("customServerAddress",client.ip);
            settings.data.set("customServerPort",client.port);
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