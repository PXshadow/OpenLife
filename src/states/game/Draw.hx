package states.game;

import openfl.display.Shape;

class Draw extends Shape
{
    var game:Game;
    var dist:Bool = true;
    var px:Float = 0;
    var py:Float = 0;
    public function new(game:Game)
    {
        this.game = game;
        super();
    }
    public function update()
    {
        if (dist = !dist) return;
        graphics.clear();
        if (game.program.setupGoal) path();
    }
    public function render()
    {

    }
    private function path()
    {
        if (Player.main != null)
        {
            //red 2 pixel line
            graphics.lineStyle(2,0x00FF00);
            //start at player
            px = Player.main.x - game.objects.x + game.x;
            py = Player.main.y - game.objects.y + game.y;
            px = Main.setWidth/2;
            py = Main.setHeight/2;
            graphics.moveTo(px,py);
            //end at goal
            px += (game.program.goal.x - Player.main.instance.x) * Static.GRID;
            py += (Player.main.instance.y - game.program.goal.y) * Static.GRID;
            graphics.lineTo(px,py);
        }
    }
}