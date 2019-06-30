package client;
#if cpp 
import discord_rpc.DiscordRpc;
#end
class Discord
{
    public function new()
    {
        //discord
        #if cpp 
        trace("discord presence init");
        function onReady()
        {
            trace("discord ready");
            DiscordRpc.presence({
            details : 'Survival',
            state   : 'Playing Solo',
            largeImageKey  : 'icon',
            largeImageText : 'Open Life '
            });
        }
        function onError(_code:Int,message:String)
        {
            trace('Error! ' + _code + " " + message);
        }
        function onDisconnected(_code : Int, _message : String)
        {
            trace('Disconnected! $_code : $_message');
        }
        DiscordRpc.start({
            clientID : "589413060582572052",
            onReady  : onReady,
            onError  : onError,
            onDisconnected : onDisconnected
        });
        #end
    }
}