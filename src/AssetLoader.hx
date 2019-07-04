import sys.FileSystem;
import haxe.Http;
import openfl.events.ProgressEvent;
import haxe.io.BytesInput;
import openfl.events.IOErrorEvent;
import openfl.events.Event;
import openfl.net.URLLoaderDataFormat;
import openfl.net.URLLoader;
import openfl.net.URLRequestMethod;
import openfl.net.URLRequest;
import haxe.io.Bytes;
class AssetLoader
{
    var github:String = "https://github.com/";
    var progress:(loaded:Float,total:Float)->Void;
    public function new(username:String,project:String,dir:String="")
    {
        Static.request(github + username + "/" + project + "/releases/latest",function(data:String)
        {
            var href = 'href="';
            var int = data.indexOf(href);
            if(int >= 0)
            {
                //has link
                int += href.length;
                var link = data.substring(int,data.indexOf('"',int));
                Static.request(link,function(data:String)
                {
                    var block = data.indexOf('<div class="d-block py-1 py-md-2 Box-body px-2">');
                    int = data.indexOf(href,block) + href.length;
                    link = data.substring(int,data.indexOf('"',int));
                    trace("link " + link);
                    unCompress(link);
                });
            }else{
                trace("failed");
            }
        });
    }
    public function unCompress(url:String)
    {
        var request = new URLRequest("https://github.com/PXshadow/OneLifeData7/archive/master.zip");
        request.contentType = "application/octet-stream";
        request.method = GET;
        var loader = new URLLoader();
        loader.dataFormat = BINARY;
        loader.addEventListener(Event.COMPLETE,function(_)
        {
            var zip = new haxe.zip.Reader(new BytesInput(loader.data));
            var list = zip.read();
            for (items in list)
            {
                trace("name " + items.fileName);
            }
        });
        loader.addEventListener(IOErrorEvent.IO_ERROR,function(_)
        {
            trace("failed");
        });
        loader.addEventListener(ProgressEvent.PROGRESS,function(e:ProgressEvent)
        {
            if (progress != null) progress(e.bytesLoaded,e.bytesTotal);
        });
        trace("start zip loader");
        //loader.load(request);
    }
}