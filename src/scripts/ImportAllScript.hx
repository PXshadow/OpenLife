package scripts;

import sys.io.File;
using haxe.io.Path;
using StringTools;
import sys.FileSystem;

class ImportAllScript
{
    var importContent:String = "";
    var importAllPath:String = "./src/ImportAll.hx";
    var source:String = "src/";
    public static function main() 
    {
        new ImportAllScript();
    }
    public function new()
    {
        trace("start importing");
        var content = File.getContent(importAllPath);
        var index = content.indexOf("#if nativeGen");
        dir(source);
        File.saveContent(importAllPath,importContent + content.substring(index,content.length));
    }
    private function dir(path:String)
    {
        if (FileSystem.exists(path) && FileSystem.isDirectory(path))
        {
            for (name in FileSystem.readDirectory(path))
            {
                //skip git
                if (name.substring(0,1) == ".") continue;
                if (["ImportAll.hx","Test.hx","Secret.hx","scripts","editor","graphics"].indexOf(name) != -1) continue;
                if (FileSystem.isDirectory(path + name))
                {
                    dir(path + name + "/");
                }else{
                    //script name
                    importContent += "import " + path.replace("/",".").substr(source.length) + name.withoutExtension() + ";\n";
                }
            }
        }
    }
}