package states.update;

import haxe.Json;
import haxe.Http;
class Update extends State
{
    //check for patch to client
    public function new()
    {
        super();
        patch(function(bool:Bool)
        {
            
        });
    }
    private function patch(finish:Bool->Void)
    {
        var channel:String = "";
        #if windows
        channel = win32-beta
        #elseif linux
        channel = "linux-universal";
        #elseif mac
        channel = "mac-os";
        #else
        finish(false);
        return;
        #end
        var http = new Http("https://itch.io/api/1/x/wharf/latest?target=pxshadow/openlife&channel_name=" + channel);
        http.onData = function(string:String)
        {
            var data = Json.parse(string);
            var errors:Array<String> = data.errors;
            if (errors.length > 0)
            {
                trace("error " + errors);
                finish(false);
            }else{
                //no errors
            }
        }
        http.onError = function(error:String)
        {
            trace("patch get error " + error);
            finish(false);
        }
        http.request(false);
    }
}