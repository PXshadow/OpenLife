package states.launcher;
import ui.Button;
import haxe.io.Path;
class Launcher extends states.State
{
    var string:String;
    public function new()
    {
        super();
        stage.color = 0;
        //directory
        #if windows
        Static.dir = "";
        #else
        Static.dir = Path.normalize(lime.system.System.applicationDirectory);
        Static.dir = Path.removeTrailingSlashes(Static.dir) + "/";
        #end
        #if mac
        Static.dir = Static.dir.substring(0,Static.dir.indexOf("/Contents/Resources/"));
        Static.dir = Static.dir.substring(0,Static.dir.lastIndexOf("/") + 1);
        #end
        trace("dir " + Static.dir);
        set();
        //ui
        var login = new Button();
        login.text = "LOGIN";
        style(login);
        login.Click = function(_)
        {
            start();
        }
        addChild(login);
        //center
        login.x = (Main.setWidth - login.width)/2;
        login.y = (Main.setHeight - login.height)/2;

        
    }
    public function set()
    {
        //account default
        //Main.client.login.email = "test@test.co.uk";
        //Main.client.login.key = "WC2TM-KZ2FP-LW5A5-LKGLP";
        Main.client.login.email = "test@test.com";
        Main.client.login.key = "9UYQ3-PQKCT-NGQXH-YB93E";
        //server default (thanks so much Kryptic <3)
        Main.client.ip = "game.krypticmedia.co.uk";
        Main.client.port = 8007;

        //settings to grab infomation
        Main.settings = new settings.Settings();
        if (!Main.settings.fail)
        {
            //account
            if (valid(Main.settings.data.email)) Main.client.login.email = string;
            if (valid(Main.settings.data.accountKey)) Main.client.login.key = string;
            if (valid(Main.settings.data.useCustomServer) && string == "1")
            {
                if (valid(Main.settings.data.customServerAddress)) Main.client.ip = string;
                if (valid(Main.settings.data.customServerPort)) Main.client.port = Std.parseInt(string);
            }
            //window
            if (valid(Main.settings.data.borderless)) stage.window.borderless = Std.parseInt(string) == 1 ? true : false;
            if (valid(Main.settings.data.fullscreen)) stage.window.fullscreen = Std.parseInt(string) == 1 ? true : false;
            if (valid(Main.settings.data.screenWidth)) stage.window.width = Std.parseInt(string);
            if (valid(Main.settings.data.screenHeight)) stage.window.height = Std.parseInt(string);
            if (valid(Main.settings.data.targetFrameRate)) stage.frameRate = Std.parseInt(string);
        }
        //by pass settings and force email and key if secret account
        #if secret
        trace("set secret");
        Main.client.login.email = Secret.email;
        Main.client.login.key = Secret.key;
        Main.client.ip = Secret.ip;
        Main.client.port = Secret.port;
        #end
    }
    public function valid(obj:Dynamic):Bool
    {
        if (obj == null || obj == "") return false;
        string = cast obj;
        return true;
    }
    private function style(button:Button)
    {
        button.graphics.endFill();
        button.graphics.lineStyle(3,0x808080);
        //lines
        button.graphics.moveTo(6,-4);
        button.graphics.lineTo(-6,-4);
        button.graphics.lineTo(-6,34);
        button.graphics.lineTo(6,34);

        button.graphics.moveTo(button.textfield.textWidth - 6 + 6,-4);
        button.graphics.lineTo(button.textfield.textWidth + 6 + 6,-4);
        button.graphics.lineTo(button.textfield.textWidth + 6 + 6,34);
        button.graphics.lineTo(button.textfield.textWidth - 6 + 6,34);

        button.graphics.endFill();
        button.graphics.beginFill(0,0);
        button.graphics.drawRect(-6,-4,button.width,34);
    }
    private function start()
    {
        Main.state.remove();
        Main.state = new states.game.Game();
    }
    override function update() 
    {
        if (settings.Bind.start.bool)
        {
            start();
        }
    }
}