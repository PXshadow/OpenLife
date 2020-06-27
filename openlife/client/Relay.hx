package openlife.client;

import sys.thread.Thread;
import sys.net.Host;
import sys.net.Socket;

class Relay
{
    public static function run(listen:Int):Client
    {
        var relay:Socket = new Socket();
        relay.bind(new Host("0.0.0.0"),listen);
        relay.listen(1);
        Sys.println('waiting for connection on port $listen');
        var client = new Client();
        var relayIn = relay.accept();
        //relayIn.setBlocking(false);
        Sys.println("begin threading relay");
        var input:String;
        Thread.create(function()
        {
            while (true)
            {
                //try {
                input = relayIn.input.readUntil("#".code);
                trace("input " + input);
                client.send(input);
                /*}catch(e:Dynamic)
                {
                    if(e != haxe.io.Error.Blocked)
                    {
                        trace('e: $e');
                        return;
                    }
                }
                Sys.sleep(1/15);*/
            }
        });
        //relay out
        client.relay = relayIn;
        return client;
    }
}
private class Client extends openlife.client.Client
{
    public var relay:Socket;
    public function new()
    {
        super();
    }
    override function process() 
    {
        super.process();
        //server -> router -> client
        //relay.close();
        relay.output.writeString('$data#');
    }
    override function compressProcess() {
        super.compressProcess();
        relay.output.write(dataCompressed);
    }
    override function close() {
        super.close();
        relay.close();
    }
}