package;
import sys.FileSystem;
import haxe.Json;
import sys.io.File;
import openlife.settings.Settings.ConfigData;
class Config
{
    public static function run(cred:Bool):ConfigData
    {
        var bool = false;
        var string:String;
        var config:ConfigData = {ip: "",port: 0,email: "",key: "",legacy: false};
        if (FileSystem.exists("cred"))
        {
            Sys.println("Use existing cred config (y)es (n)o");
            bool = Sys.stdin().readLine() == "n" ? false : true;
            if (bool)
            {
                config = cast Json.parse(File.getContent("cred"));
                config.ip = config.ip;
                config.port = config.port;
                config.legacy = config.legacy;
                config.email = config.email;
                config.key = config.key;
            }
        }
        if (!bool)
        {
            Sys.println("Legacy authentication (y)es (n)o");
            config.legacy = Sys.stdin().readLine() == "y" ? true : false;
            //set credentioals
            if (!cred)
            {
                Sys.println("ip:");
                string = Sys.stdin().readLine();
                config.ip = string;
                Sys.println("port:");
                string = Sys.stdin().readLine();
                config.port = Std.parseInt(string);
                Sys.println("email:");
                string = Sys.stdin().readLine();
                config.email = string;
                Sys.println("key");
                string = Sys.stdin().readLine();
                config.key = string;
                //set config
                File.saveContent("cred",Json.stringify(config));
            }
        }
        return config;
    }
}