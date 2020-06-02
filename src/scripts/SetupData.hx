package scripts;
import haxe.io.Path;
import sys.io.Process;
import sys.FileSystem;
import sys.io.File;

class SetupData
{
    var outputPaths:Array<String> = [];
    var dir:String;
    var dataDir:String;
    public static function main()
    {
        new SetupData();
    }
    public function new()
    {
        dir = Sys.getCwd();
        trace("Start " + dir);
        //move onelifedata7 files to windows or mac folder depending on whitch is up
        if (FileSystem.exists("bin/windows")) outputPaths.push("bin/windows/bin/");
        if (FileSystem.exists("bin/macOS")) outputPaths.push("bin/macOS/bin/");
        if (FileSystem.exists("bin/hl")) outputPaths.push("bin/hl/bin/");
        if (FileSystem.exists("bin/neko")) outputPaths.push("bin/neko/bin/");
        if (FileSystem.exists("bin/server")) outputPaths.push("bin/server/");
        //setup linux later
        trace("paths: " + outputPaths);
        //check if path is set
        if (outputPaths.length == 0)
        {
            trace("No built directory");
            Sys.sleep(1);
            return;
        }
        if (true)
        {
            dataDir = Path.join([dir,"/onelifedata7"]);
            //trace("dir " + FileSystem.readDirectory("."));
            if (!FileSystem.exists("onelifedata7"))
            {
                trace("clone-");
                Sys.command("git clone https://github.com/jasonrohrer/OneLifeData7.git");
                Sys.setCwd(dataDir);
            }else{
                trace("pull-");
                Sys.setCwd(dataDir);
                Sys.command("git pull https://github.com/jasonrohrer/OneLifeData7.git --force");
            }
            Sys.command("git fetch --tags");
            var proc = new Process("git for-each-ref --sort=-creatordate --format '%(refname:short) --count=1");
            var tag = proc.stdout.readAll().toString();
            tag = tag.substring(1,tag.length - 1);
            trace("tag|" + tag + "|");
            if (tag.indexOf("OneLife_v") == 0)
            {
                Sys.command('git checkout -q $tag');
                trace("checkout!");
            }else{
                trace("tag format wrong: " + tag);
            }
        }
        Sys.setCwd(dir);
        //copy
        trace("begin copy " + outputPaths + " dir " + FileSystem.readDirectory("."));
        for (path in outputPaths) 
        {
            //remove bake.res file
            if (FileSystem.exists("bake.res")) FileSystem.deleteFile("bake.res");
            //copy directory over
            copyDir(Path.addTrailingSlash(dataDir),path,true);
        }
        trace("Finished data setup");
        Sys.sleep(2);
    }
    /**
     * Code from the launcher I made a while ago :)
     * @param path 
     * @param newpath 
     */
    private function copyDir(path:String,newpath:String,main:Bool=false)
    {
        if (FileSystem.exists(path) && FileSystem.isDirectory(path))
        {
            var dir = FileSystem.readDirectory(path);
            var i:Int = 0;
            for (name in dir)
            {
                if (main) trace("process " + i++/dir.length);
                //skip git
                if (name.substring(0,1) == ".") continue;

                if (FileSystem.isDirectory(path + name))
                {
                    FileSystem.createDirectory(newpath + name);
                    copyDir(path + name + "/", newpath + name + "/");
                }else{
                    File.copy(path + name,newpath + name);
                }
            }
        }
    }
}