package;

import sys.io.File;
import sys.FileSystem;

class HLDebug
{
    public static function main()
    {
        for (lib in ["vshaxe","vscode","vscode-debugadapter"])
        {
            Sys.command('haxelib install $lib');
        }
        if (!FileSystem.exists("hashlink-debugger"))
        {
            Sys.command("git clone https://github.com/vshaxe/hashlink-debugger");
        }
        final path = "hashlink-debugger/debugger/";
        final init = Sys.getCwd();
        trace("init " + init);
        Sys.setCwd(path);
        trace("set");
        Sys.command('haxe debugger.hxml');
        Sys.setCwd(init);
        File.copy(path + "debug.hl","debug.hl");
    }
}
