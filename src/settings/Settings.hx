package settings;
import haxe.io.Path;
#if sys
import sys.io.File;
import sys.FileSystem;
#end
class Settings
{
    public var data:Dynamic = {};
    public var fail:Bool = true;
    public function new()
    {
        var path:String = Static.dir + "settings/";
        if (FileSystem.exists(path) && FileSystem.isDirectory(path))
        {
            fail = false;
            for (name in FileSystem.readDirectory(path))
            {
                Reflect.setField(data,Path.withoutExtension(name),File.getContent(path + name));
            }
        }else{
            fail = true;
            trace("fail");
        }
    }
}