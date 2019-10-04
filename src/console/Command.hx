package console;
#if openfl
import openfl.Lib;
#end

class Command
{
    #if !openfl
    var Lib:Dynamic = {};
    #end
    public function new()
    {

    }
    public function run(string:String):Bool
    {
        string = string.toLowerCase();
        switch(string)
        {
            case "exit":
            #if sys
            Sys.exit(0);
            #end
            case "reload":
            //hotreload

            //states
            case "menu":
            //go to menu

            case "game":
            //go to game
            case "clear":
            //clear display 

            case "disconnect" | "dc":
            Main.client.close();
            case "connect" | "c":
            Main.client.connect();
            #if openfl
            //window
            case "fullscreen":
            Lib.current.stage.window.fullscreen = !Lib.current.stage.window.fullscreen;
            case "controls":
            //toggle controls
            case "borderless":
            Lib.current.stage.window.borderless = !Lib.current.stage.window.borderless;
            #end
            case "date":
            //console.print("date",Date.now().toString());
            //urls
            case "github" | "code" | "source":
            url("https://github.com/pxshadow/openlife");
            case "techtree" | "tech" | "tree":
            url("https://onetech.info/");
            case "forums" | "forms" | "fourms" | "forum" | "form" | "fourm":
            url("https://onehouronelife.com/forums/");
            case "wiki":
            url("https://onehouronelife.gamepedia.com/One_Hour_One_Life_Wiki");
            /*case "debug":
            Main.console.debug = !Main.console.debug;
            case "release":
            Main.console.debug = false;*/
            default:
            return false;
        }
        return true;
    }
    public function url(string:String)
    {
        #if openfl
        openfl.Lib.navigateToURL(new openfl.net.URLRequest(string));
        #else
        execUrl(string);
        #end
    }
    #if !openfl
    public function execUrl (url:String) : Void 
    {
        switch (Sys.systemName()) 
        {
            case "Linux", "BSD": Sys.command("xdg-open", [url]);
            case "Mac": Sys.command("open", [url]);
            case "Windows": Sys.command("start", [url]);
            default:
        }
    }
    #end
}