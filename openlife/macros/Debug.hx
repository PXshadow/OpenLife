package openlife.macros;

import openlife.settings.OpenLifeData;
import sys.FileSystem;

class Debug
{
    public static function run()
    {
        if (!FileSystem.exists("data.json"))
            return;
        var data = OpenLifeData.getData();
        trace("data " + data);
        if (data.debug) 
        {
            haxe.macro.Compiler.define("debug","");
            haxe.macro.Compiler.addNativeArg("--no-inline");
            haxe.macro.Compiler.addNativeArg("-v");
        }
    }
}