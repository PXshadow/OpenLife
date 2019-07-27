package states.launcher;
import ui.Button;
import haxe.io.Path;
class Launcher extends states.State
{

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
        //ui
        var login = new Button();
        login.text = "LOGIN";
        style(login);
        login.Click = function(_)
        {
            Main.state.remove();
            Main.state = new states.game.Game();
        }
        addChild(login);
        //center
        login.x = (Main.setWidth - login.width)/2;
        login.y = (Main.setHeight - login.height)/2;

        var test = new Button();
        test.graphics.beginFill(0xFFFFFF);
        test.graphics.drawRect(0,0,100,100);
        addChild(test);
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
}