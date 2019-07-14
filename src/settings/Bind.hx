package settings;

import openfl.events.KeyboardEvent;
import openfl.ui.Keyboard;

//deals with mouse and keyboard control setup
class Bind
{
    public static var cameraUp:Action = new Action([Keyboard.UP]);
    public static var cameraDown:Action = new Action([Keyboard.DOWN]);
    public static var cameraLeft:Action = new Action([Keyboard.LEFT]);
    public static var cameraRight:Action = new Action([Keyboard.RIGHT]);

    public static var playerUp:Action = new Action([Keyboard.W]);
    public static var playerDown:Action = new Action([Keyboard.S]);
    public static var playerLeft:Action = new Action([Keyboard.A]);
    public static var playerRight:Action = new Action([Keyboard.D]);

    public static var playerPick:Action = new Action([Keyboard.SPACE]);
    public static var playerDrop:Action = new Action([Keyboard.SPACE]);
    

    public static function keys(e:KeyboardEvent,bool:Bool)
    {
        cameraUp.set(e,bool);
        cameraDown.set(e,bool);
        cameraLeft.set(e,bool);
        cameraRight.set(e,bool);
        
        playerUp.set(e,bool);
        playerDown.set(e,bool);
        playerLeft.set(e,bool);
        playerRight.set(e,bool);

        playerPick.set(e,bool);
        playerDrop.set(e,bool);
    }
}
class Action
{
    public var bool:Bool = false;
    public var array:Array<Int> = [];
    public var control:Bool = false;
    public var shift:Bool = false;
    public var alt:Bool = false;
    public function new(array:Array<Int>)
    {
        this.array = array;
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