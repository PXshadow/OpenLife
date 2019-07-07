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
import haxe.io.BytesInput;
import haxe.io.Bytes;
class AssetLoader
{
    var github:String = "https://github.com/";
    public var progress:(loaded:Float,total:Float)->Void;
    public var complete:Bool->Void;
    //update version
    public var update:String->Void;
    public var current:Void->Void;
    public function new()
    {
    }
    public function loader(url:String,path:String,version:String)
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
                    //version
                    versionFunction(link,version);
                    //redirect
                    link = link.substring(0,link.indexOf("/",8)) + data.substring(int,data.indexOf('"',int));
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
    private function versionFunction(url:String,version:String)
    {
        if (url.indexOf("github") >= 0)
        {
            var v = url.substring(url.lastIndexOf("/") + 1,url.length);
            if (version == v)
            {
                //same version
                if (complete != null) complete(true);
                return;
            }else{
                if (update != null) update(v);
            }
        }
    }
    private function unCompress(url:String,path:String)
    {
        var request = new URLRequest(url);
        request.contentType = "application/octet-stream";
        request.method = GET;
        var loader = new URLLoader();
        loader.dataFormat = BINARY;
        loader.addEventListener(Event.COMPLETE,function(_)
        {
            trace("unzip");
            //unziper like a pro
            unzip(haxe.zip.Reader.readZip(new BytesInput(loader.data)),path);
        });
        loader.addEventListener(IOErrorEvent.IO_ERROR,function(e:IOErrorEvent)
        {
            trace("io error " + e);
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
        var i:Int = 0;
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
                    File.write(path + items.fileName).write(haxe.zip.Reader.unzip(items));
                }else{
                    trace("Can not find directory " + Path.directory(items.fileName));
                }
            }
            i++;
            if (i > 20) 
            {
                Sys.sleep(0.016);
                trace("sleep");
            }
        }
        if (complete != null) complete(true);
    }
}