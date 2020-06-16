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
    var users:Array<String> = ["jasonrohrer","twohoursonelife"];
    var index:Null<Int>;
    public function new()
    {
        Sys.println('Repository index $users:');
        index = Std.parseInt(Sys.stdin().readLine());
        if (index == null || index < 0 || index > users.length - 1) index = 0;
        Sys.println('Downloading ${users[index]}');
        //linux is folder name case senetive
        if (!FileSystem.exists("OneLifeData7"))
        {
            trace("clone-");
            Sys.command('git clone https://github.com/${users[index]}/OneLifeData7.git');
        }
        Sys.setCwd("OneLifeData7");
        trace("pull-");
        Sys.command('git pull https://github.com/${users[index]}/OneLifeData7.git --force');
        Sys.command("git fetch --tags");
        var proc = new Process("git for-each-ref --sort=-creatordate --format '%(refname:short)' --count=1");

        var tag = proc.stdout.readLine();
        tag = StringTools.trim(tag);
        tag = StringTools.replace(tag,"'","");
        trace("tag = |" + tag + "|");
        if (tag.indexOf("v") > -1)
        {
            Sys.command('git checkout -q $tag');
            trace("checkout!");
        }else{
            trace("tag format wrong: " + tag);
        }
    }
}