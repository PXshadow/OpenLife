package;
import sys.FileSystem;
using haxe.io.Path;
class OpenObject
{
    static function main()
    {
        new OpenObject();
    }
    var path:String;
    public function new()
    {
        while (!getPath()) trace("Not a valid path, needs to be where the main game directory is ./objects");
        while (true)
        {
            Execute.run(path + "objects/" + getId() + ".txt");
        }
    }
    private function getPath():Bool
    {
        trace("input path:");
        path = Sys.stdin().readLine().normalize().addTrailingSlash();
        return (FileSystem.exists(path + "objects") && FileSystem.isDirectory(path + "objects"));
    }
    private function getId()
    {
        trace("input id:");
        return Std.parseInt(Sys.stdin().readLine());
    }
}