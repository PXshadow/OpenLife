package;

import sys.io.File;
import sys.FileSystem;
using haxe.io.Path;
using StringTools;
class RemoveCarriage
{
    public static function main()
    {
        new RemoveCarriage();
    }
    public function new()
    {
        recursion("OneLifeData7");
    }
    private function recursion(dir:String)
    {
        for (path in FileSystem.readDirectory(dir))
        {
            if (path.substring(0,1) == ".") continue;
            if (FileSystem.isDirectory(dir.addTrailingSlash() + path))
            {
                recursion(dir.addTrailingSlash() + path);
            }else{
                if (path.extension() == "txt")
                {
                    File.saveContent(dir.addTrailingSlash() + path,File.getContent(dir.addTrailingSlash() + path).replace(String.fromCharCode(13),""));
                }
            }
        }
    }
}