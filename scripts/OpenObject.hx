package;
import openlife.engine.Utility;
import sys.FileSystem;
using haxe.io.Path;
class OpenObject
{
    static function main()
    {
        new OpenObject();
    }
    var path:String = Utility.dir();
    public function new()
    {
        if (!FileSystem.exists('${path}objects/nextObjectNumber.txt'))
        {
            trace("directory not found");
            return;
        }
        Execute.run(path + "objects/" + getId() + ".txt");
    }
    private function getId()
    {
        trace("input id:");
        return Std.parseInt(Sys.stdin().readLine());
    }
}