package settings;
import haxe.io.Path;
import openfl.Lib;
import openfl.net.SharedObject;
import lime.system.System;
import lime.ui.FileDialogType;
import lime.ui.FileDialog;
#if sys
import sys.io.File;
import sys.FileSystem;
#end
class Settings
{
    public var data:Dynamic;
    public var fail:Bool = true;
    public function new()
    {
        var path:String = Static.dir + "/settings/";
        if (FileSystem.isDirectory(path))
        {
            fail = false;
            for (name in FileSystem.readDirectory(path))
            {
                Reflect.setField(data,Path.withoutExtension(name),File.getContent(path + name));
            }
        }
    }
}