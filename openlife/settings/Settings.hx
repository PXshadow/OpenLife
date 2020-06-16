package openlife.settings;
import openlife.resources.Resource;
import haxe.DynamicAccess;
import haxe.io.Path;
import openlife.engine.Engine;
#if (sys || nodejs)
import sys.io.FileOutput;
import sys.io.File;
import sys.FileSystem;
#end

class Settings
{
    @:isVar public var data(default,set):Data = {};
    function set_data(value:Data):Data
    {
        var a = value.keys();
        var b = data.keys();
        trace('a ${a.length} b ${b.length}');
        if (a.length > b.length)
        {
            var name = a[a.length - 1] + ".ini";
            var obj = value.get(name);
            #if sys
            //set settings
            var file = File.write(Engine.dir + "settings/" + name,false);
            file.writeString(obj);
            file.close();
            #end

            #if (js || html)
            
            #end
        }
        return data = value;
    }
    public function new()
    {
        var path:String = Engine.dir + "settings/";
        #if sys
        if (!FileSystem.exists(path))
        {
            FileSystem.createDirectory(Engine.dir + "settings");
        }
        for (name in FileSystem.readDirectory(path))
        {
            Reflect.setField(data,Path.withoutExtension(name),File.getContent(path + name));
        }
        #end
    }
}
typedef Data = DynamicAccess<Dynamic> 