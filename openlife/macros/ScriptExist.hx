package openlife.macros;

import haxe.macro.Compiler;
import sys.FileSystem;

class ScriptExist
{
    public static macro function run()
    {
        if (FileSystem.exists("Script.hx")) Compiler.define("script");
        return null;
    }
}