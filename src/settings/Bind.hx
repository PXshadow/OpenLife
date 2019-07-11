package settings;

import openfl.ui.Keyboard;

//deals with mouse and keyboard control setup
class Bind
{
    public static var cameraUp:Bool = false;
    public static var cameraDown:Bool = false;
    public static var cameraLeft:Bool = false;
    public static var cameraRight:Bool = false;
    public static var playerUp:Bool = false;
    public static var playerDown:Bool = false;
    public static var playerLeft:Bool = false;
    public static var playerRight:Bool = false;

    public static var cameraUpArray:Array<Int> = [Keyboard.UP];
    public static var cameraDownArray:Array<Int> = [Keyboard.DOWN];
    public static var cameraLeftArray:Array<Int> = [Keyboard.LEFT];
    public static var cameraRightArray:Array<Int> = [Keyboard.RIGHT];

    public static var playerUpArray:Array<Int> = [Keyboard.W];
    public static var playerDownArray:Array<Int> = [Keyboard.S];
    public static var playerLeftArray:Array<Int> = [Keyboard.A];
    public static var playerRightArray:Array<Int> = [Keyboard.D];

    public static function keys(code:Int,bool:Bool)
    {
        if (cameraUpArray.indexOf(code) >= 0) cameraUp = bool;
        if (cameraDownArray.indexOf(code) >= 0) cameraDown = bool;
        if (cameraLeftArray.indexOf(code) >= 0) cameraLeft = bool;
        if (cameraRightArray.indexOf(code) >= 0) cameraRight = bool;

        if (playerUpArray.indexOf(code) >= 0) playerUp = bool;
        if (playerDownArray.indexOf(code) >= 0) playerDown = bool;
        if (playerLeftArray.indexOf(code) >= 0) playerLeft = bool;
        if (playerRightArray.indexOf(code) >= 0) playerRight = bool;

    }
}