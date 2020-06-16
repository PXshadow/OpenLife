package;
import sys.FileSystem;
using haxe.io.Path;
class OpenObject
{
    static function main()
    {
        new OpenObject();
    }
    var path:String = "./OneLifeData7/";
    public function new()
    {
        if (!FileSystem.exists(path))
        {
            trace("OneLifeData7 folder not found");
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