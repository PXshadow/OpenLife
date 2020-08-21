package openlife.server;

import openlife.data.Pos;

interface ServerHeader 
{
    public function keepAlive():Void;
    public function login():Void;
    public function rlogin():Void;
    public function move(x:Int,y:Int,seq:Int,moves:Array<Pos>):Void;
    public function die():Void;
}