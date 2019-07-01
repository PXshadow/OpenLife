package states.game;

class Objects extends TileDisplay
{
    var game:Game;
    public function new(game:Game)
    {
        this.game = game;
        super(Main.setWidth,Main.setHeight);
    }
}