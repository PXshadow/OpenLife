package states.game;

import openfl.display.Tilemap;

class Objects extends Tilemap
{
    var game:Game;
    public function new(game:Game)
    {
        this.game = game;
        super(Main.setWidth,Main.setHeight);
    }
}