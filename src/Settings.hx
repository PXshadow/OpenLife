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
    public static var assetPath:String = "assets/";
    private var email:String;
    private var key:String;
    public var connected:Bool = false;
    private function new()
    {
        if(FileSystem.isDirectory("assets"))
        {
            return;
        }
        if(FileSystem.isDirectory("Settings"))
        {
            //assetPath = "Settings/";
            assetPath = "";
            return;
        }
        Lib.application.window.alert("Place OneHourOneLife in Directory","Not found Game Folders");
    }
    public static function getLocal():Settings
    {
        return new Settings();
    }
}