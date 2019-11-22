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
        //move onelifedata7 files to windows or mac folder depending on whitch is up
        var path:String = "";
        if (FileSystem.exists("bin/windows")) path = "bin/windows/bin";
        if (FileSystem.exists("bin/macOS")) path = "bin/macOS/bin";
        //setup linux later

        //check if path is set
        if (path == "")
        {
            trace("No built directory");
            Sys.sleep(1);
            return;
        }
        if (!FileSystem.exists("OneLifeData7"))
        {
            Sys.command("git clone https://github.com/pxshadow/onelifedata7");
            Sys.sleep(1);
            if (!FileSystem.exists("OneLifeData7"))
            {
                trace("Not able to generate OneLifeData7");
                Sys.sleep(1);
                return;
            }
        }
        copyDir("OneLifeData7",path);
    }
    /**
     * Code from the launcher I made a while ago :)
     * @param path 
     * @param newpath 
     */
    private function copyDir(path:String,newpath:String)
    {
        if (FileSystem.exists(path) && FileSystem.isDirectory(path))
        {
            for (name in FileSystem.readDirectory(path))
            {
                if (FileSystem.isDirectory(path + name))
                {
                    copyDir(path + name + "/", newpath + name + "/");
                    FileSystem.createDirectory(newpath + name);
                }else{
                    File.copy(path + name,newpath + name);
                }
            }
        }
    }
}