package scripts;

import sys.FileSystem;
import haxe.io.Path;
class Generate
{
    public static function main()
    {
        trace("start generation");
        //docs
        var exclude = ["debugger","shaders","ui","scripts"];
        var string:String = '"(';
        for (dir in FileSystem.readDirectory("src"))
        {
            if (FileSystem.isDirectory("src/" + dir) && exclude.indexOf(dir) == -1)
            {
                for (name in FileSystem.readDirectory("src/" + dir))
                {
                    string += dir + "." + Path.withoutExtension(name) + "|";
                }
            }
        }
        string += ')"';
        trace("generate html api");
        Sys.command('haxelib run dox -i xml -o docs -D version "0.0.1 alpha" -D logo "https://raw.githubusercontent.com/PXshadow/OpenLife/master/logo.png" -D title "API Reference" -D source-path "https://github.com/PXshadow/openlife/tree/master/src/" --include ' + string);
    }
}