package scripts; //used globally
import haxe.io.Path;
import sys.io.Process;
import sys.FileSystem;
import sys.io.File;

class SetupData
{
    public static function main()
    {
        new SetupData();
    }
    public function new()
    {
        trace("exists 1 " + FileSystem.exists("onelifedata7") + " 2 " + FileSystem.exists("OneLifeData7"));
        //linux 1 is fals but 2 is true, whitch means case sensitive
        if (FileSystem.exists("onelifedata7") || FileSystem.exists("OneLifeData7"))
        {
            trace("clone-");
            Sys.command("git clone https://github.com/jasonrohrer/OneLifeData7.git");
        }
        Sys.setCwd("onelifedata7");
        trace("pull-");
        Sys.command("git pull https://github.com/jasonrohrer/OneLifeData7.git --force");
        Sys.command("git fetch --tags");
        var proc = new Process("git for-each-ref --sort=-creatordate --format '%(refname:short)' --count=1 refs/tags/OneLife_v*");

        var tag = proc.stdout.readLine();
        //var tag = proc.stdout.readAll().toString();
        tag = StringTools.trim(tag);
        tag = StringTools.replace(tag,"'","");
        trace("tag = |" + tag + "|");
        if (tag.indexOf("OneLife_v") == 0)
        {
            Sys.command('git checkout -q $tag');
            trace("checkout!");
        }else{
            trace("tag format wrong: " + tag);
        }
    }
}