package console;

import states.game.Player;
import states.game.Game;
class Program
{
    var game:Game;
    var goal:Pos = new Pos();
    public function new(game:Game)
    {
        this.game = game;
        
    }
    public function drop():Program
    {
        return this;
    }
    public function find(name:String):Program
    {
        id(name);
        return this;
    }
    public function pickup():Program
    {
        return this;
    }
    public function self(index:Int=-1):Program
    {
        return this;
    }
    public function emote(index:Int,time:Int=1):Program
    {
        return this;
    }
    public function craft(name:String):Program
    {
        return this;
    }
    //move player
    public function move(x:Int,y:Int):Program
    {
        Player.main.move(x,y);
        return this;
    }
    private function id(name:String):Array<Int>
    {
        return switch(name)
        {
            case "berry bush":
            [30];
            case "berry" | "berries": 
            [31];
            default: 
            [-1];
        }
    }
    private function cat()
    {

    }
}
class Pos
{
    var x:Int;
    var y:Int;
    public function new()
    {

    }
}