package states.game;

import openfl.display.Tilemap;

class Ground extends Tilemap
{
    var game:Game;
    public function new(game:Game)
    {
        this.game = game;
        super(Main.setWidth,Main.setHeight);
        cacheAsBitmap = true;
    }
}