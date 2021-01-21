package openlife.auto;
import openlife.data.Pos;

interface MessageHandler
{
    public function say(text:String):Void;
    public function self(x:Int,y:Int,clothingSlot:Int):Void;
    public function move(x:Int,y:Int,seq:Int,moves:Array<Pos>):Void;
}