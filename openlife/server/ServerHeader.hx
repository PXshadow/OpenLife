package openlife.server;

import openlife.data.Pos;

// TODO currently logic is in process of shifting from Connection to GlobalPlayerInstance
interface ServerHeader 
{
    public function keepAlive():Void;
    public function login():Void;
    public function rlogin():Void;
    //public function move(x:Int,y:Int,seq:Int,moves:Array<Pos>):Void;
    public function die():Void;
    public function emote(id:Int):Void;
    //public function use(x:Int,y:Int):Void;
    //public function drop(x:Int,y:Int):Void;
    public function say(text:String):Void;
    public function flip():Void;

    public var player:GlobalPlayerInstance;

    //public function get_player():GlobalPlayerInstance;
}