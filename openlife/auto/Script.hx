package openlife.auto;

import sys.FileSystem;
import sys.io.File;

class Script
{
    public static function create()
    {
        if (!FileSystem.exists("Script.hx")) File.saveContent("Script.hx",File.getContent("Script.txt"));
    }
    #if hscript
    private static var last:Float = 0;
    public static function execute():String
    {
        create();
        var data = File.getContent("Script.hx");
        final exec = "//execute ";
        var index = data.indexOf(exec);
        if (last == lastMod()) return "";
        last = lastMod();
        final firstLine = "var bot = new Bot(new openlife.client.Client());";
        data = data.substring(data.indexOf(firstLine) + firstLine.length,data.lastIndexOf("}"));
        trace(data);
        return data;
    }
    private static function lastMod():Float
    {
        return FileSystem.stat("Script.hx").mtime.getTime();
    }
    #end
}