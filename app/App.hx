package;

import openlife.data.map.MapInstance;
import openlife.engine.Engine;

class App extends Engine
{
    public function new()
    {
        super();
        cred();
        connect(false);
        var string = "";
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