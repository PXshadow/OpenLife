package;

import openlife.data.map.MapInstance;
import openlife.engine.Engine;

class App extends Engine
{
    public function new()
    {
        super();
        cred();
        client.ip = "thinqbator.app";
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