package states.launcher;
import sys.io.File;
import haxe.io.Path;
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
    public var progress:(loaded:Float,total:Float)->Void;
    public var complete:Bool->Void;
    public function new()
    {
    }
    public function loader(url:String,path:String)
    {
        Static.request(url,function(data:String)
        {
            var href = 'href="';
            var int = data.indexOf(href);
            if(int >= 0)
            {
                //redirect
                int += href.length;
                var link = data.substring(int,data.indexOf('"',int));
                Static.request(link,function(data:String)
                {
                    var block = data.indexOf('<div class="d-block py-1 py-md-2 Box-body px-2">');
                    int = data.indexOf(href,block) + href.length;
                    link = data.substring(int,data.indexOf('"',int));
                    trace("link " + link);
                    unCompress(link,path);
                });
            }else{
                //non redirect
                trace("failed");
                unCompress(url,path);
            }
        });
    }
    private function unCompress(url:String,path:String)
    {
        var request = new URLRequest("https://github.com/PXshadow/OneLifeData7/archive/master.zip");
        request.contentType = "application/octet-stream";
        request.method = GET;
        var loader = new URLLoader();
        loader.dataFormat = BINARY;
        loader.addEventListener(Event.COMPLETE,function(_)
        {
            var zip = new haxe.zip.Reader(new BytesInput(loader.data));
            //unziper like a pro
            unzip(zip.read(),path);
        });
        loader.addEventListener(IOErrorEvent.IO_ERROR,function(_)
        {
            trace("failed");
            if (complete != null) complete(false);
        });
        loader.addEventListener(ProgressEvent.PROGRESS,function(e:ProgressEvent)
        {
            if (progress != null) progress(e.bytesLoaded,e.bytesTotal);
        });
        trace("start zip loader");
        loader.load(request);
    }
    private function unzip(list:List<haxe.zip.Entry>,path:String)
    {
        var ext:String = "";
        path += "/";
        for (items in list)
        {
            items.fileName = items.fileName.substring(items.fileName.indexOf("/") + 1,items.fileName.length);
            ext = Path.extension(items.fileName);
            if(ext == "")
            {
                //folder
                FileSystem.createDirectory(path + items.fileName);
            }else{
                if (FileSystem.isDirectory(path + Path.directory(items.fileName)))
                {
                    File.write(path + items.fileName).writeBytes(items.data,0,items.data.length);
                }else{
                    trace("Can not find directory " + Path.directory(items.fileName));
                }
            }

        }
        if (complete != null) complete(true);
    }
}