package states.game;
import data.PlayerData.PlayerInstance;
import motion.MotionPath;
import openfl.geom.Point;
import motion.Actuate;
import haxe.Timer;
import data.SpriteData;
import data.ObjectData;
class Player extends Object
{
    public var tileX:Int = 0;
    public var tileY:Int = 0;
    public function new(id:Int)
    {
        super();
    }
}