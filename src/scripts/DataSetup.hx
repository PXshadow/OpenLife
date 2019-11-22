package scripts;
import sys.FileSystem;
import sys.io.File;
class DataSetup
{
    public static function main()
    {
        new DataSetup();
    }
    public function new()
    {
        trace("Start");
        //move onelifedata7 files to windows or mac folder depending on whitch is up
        var path:String = "";
        if (FileSystem.exists("bin/windows")) path = "bin/windows/bin/";
        if (FileSystem.exists("bin/macOS")) path = "bin/macOS/bin/";
        //setup linux later
        trace("path: " + path);
        //check if path is set
        if (path == "")
        {
            trace("No built directory");
            Sys.sleep(1);
            return;
        }
        if (!FileSystem.exists("OneLifeData7"))
        {
            trace("OneLifeData7 does not exist, clone");
            Sys.command("git clone https://github.com/pxshadow/onelifedata7");
            Sys.sleep(1);
            if (!FileSystem.exists("OneLifeData7"))
            {
                trace("Not able to generate OneLifeData7");
                Sys.sleep(1);
                return;
            }
        }
        trace("begin copy");
        copyDir("OneLifeData7/",path,true);
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