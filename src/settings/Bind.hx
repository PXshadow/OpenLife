package settings;
#if openfl 
import openfl.events.KeyboardEvent;
import openfl.ui.Keyboard;

//deals with mouse and keyboard control setup
class Bind
{

    public static var playerUp:Action = new Action([Keyboard.W,Keyboard.UP]);
    public static var playerDown:Action = new Action([Keyboard.S,Keyboard.DOWN]);
    public static var playerLeft:Action = new Action([Keyboard.A,Keyboard.LEFT]);
    public static var playerRight:Action = new Action([Keyboard.D,Keyboard.RIGHT]);

    public static var playerUse:Action = new Action([Keyboard.G]);
    public static var playerDrop:Action = new Action([Keyboard.Q]);

    public static var playerSelf:Action = new Action([Keyboard.SPACE]);
    public static var playerKill:Action = new Action([Keyboard.SHIFT]);
    public static var playerMove:Action = new Action([Keyboard.CONTROL]);

    public static var zoomIn:Action = new Action([Keyboard.I]);
    public static var zoomOut:Action = new Action([Keyboard.O]);

    public static var search:Action = new Action([Keyboard.F],true);
    public static var chat:Action = new Action([Keyboard.ENTER]);
    //show commands
    public static var help:Action = new Action([Keyboard.H]);

    public static var settings:Action = new Action([Keyboard.ESCAPE]);

    public static var start:Action = new Action([Keyboard.ENTER]);
    

    public static function keys(e:KeyboardEvent,bool:Bool)
    {
        zoomIn.set(e,bool);
        zoomOut.set(e,bool);

        playerUp.set(e,bool);
        playerDown.set(e,bool);
        playerLeft.set(e,bool);
        playerRight.set(e,bool);

        playerUse.set(e,bool);
        playerDrop.set(e,bool);

        playerSelf.set(e,bool);
        playerKill.set(e,bool);

        playerMove.set(e,bool);

        search.set(e,bool);
        help.set(e,bool);
        chat.set(e,bool);
        settings.set(e,bool);

        start.set(e,bool);
    }
}
class Action
{
    public var bool:Bool = false;
    public var array:Array<Int> = [];
    public var control:Bool = false;
    public var shift:Bool = false;
    public var alt:Bool = false;
    public function new(array:Array<Int>,control:Bool=false,shift:Bool=false,alt:Bool=false)
    {
        this.array = array;
        this.control = control;
        this.shift = shift;
        this.alt = alt;
    } 
    public function set(e:KeyboardEvent,bool:Bool)
    {
        if (array.indexOf(e.keyCode) >= 0)
        {
            this.bool = bool;
            if (this.bool)
            {
                var controlKey:Bool = #if mac e.commandKey #else e.controlKey #end;
                if (control && !controlKey || shift && !e.shiftKey || alt && !e.altKey)
                {
                    this.bool = false;
                }
            }
        }
        return false;
    }
}
#end