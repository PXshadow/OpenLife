package openlife.engine;
import sys.FileSystem;
import sys.io.File;
@:expose
class Utility
{
    public static function dir():String
    {
        if (!FileSystem.exists("dir")) File.saveContent("dir","OneLifeData7/");
        return File.getContent("dir");
    }
}