package openlife.client;

import haxe.io.Eof;
import haxe.io.Error;
import haxe.Exception;
import sys.thread.Thread;
import sys.net.Host;
import sys.net.Socket;

class Relay
{
    public static function run(listen:Int):Client
    {
        var relay:Socket = new Socket();
        relay.bind(new Host("localhost"),listen);
        relay.listen(1);
        Sys.println('waiting for connection on port $listen');
        var client:Client = new Client();
        var relayIn = relay.accept();
        //here we are connected
        relayIn.setFastSend(true);
        relayIn.setBlocking(false);
        Sys.println("begin threading relay");
        var input:String;
        Thread.create(function()
        {
            while (true)
            {   
                try {
                    input = relayIn.input.readUntil("#".code);
                    //trace("input " + input);
                    client.send(input);
                }catch(e:Dynamic)
                {   
                    if(e != haxe.io.Error.Blocked)
                    {
                        var msg:String = '$e';
                        if(msg.indexOf('Eof') > -1){
                            //this is where we need to reset the connection
                            relay.close();
                            client.resetFlag=true;
                            client.close();
                            //reset();
                            return;
                        } else {
                            trace('e: $e');
                            return;
                        }
                    }
                }   
                Sys.sleep(1/40);
            }
        });
        //relay out
        client.relay = relayIn;
        return client;
    }
}
private class Client extends openlife.client.Client
{
    public function new()
    {
        super();
    }
    override function process(wasCompressed:Bool) 
    {
        if(!connected)
            return;
        super.process(wasCompressed);
        if (wasCompressed) return;
        //server -> router -> client
        //relay.close();
        trace('output $data');
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