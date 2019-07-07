package states.game;

import openfl.display.Tile;
import data.ObjectData;

class Objects extends TileDisplay
{
    var game:Game;
    //old pos
    var ox:Int = 0;
    var oy:Int = 0;
    var tile:Tile;
    var clean:Bool = false;
    public function new(game:Game)
    {
        this.game = game;
        super(Main.setWidth,Main.setHeight);
    }
    //when map has changed
    public function update()
    {
        if (ox != game.tileX || oy != game.tileY)
        {
            ox = game.tileX;
            oy = game.tileY;
            clean = true;
        }
        for(i in 0...numTiles)
        {
            tile = getTileAt(i);
            tile.x += x;
            tile.y += y;
            if (clean)
            {
                if (tile.x > width || tile.x < 0 || tile.y > height || tile.y < 0)
                {
                    removeTile(tile);
                }
            }
        }
        clean = false;
        //reset pos
        x = 0;
        y = 0;
    }
    public function addFloor(id:Int)
    {
        add(id);
    }
    public function addObject(string:String)
    {
        var id:Null<Int> = Std.parseInt(string);
        if (id != null)
        {
            //single object
            add(id);
        }else{
            //group
        }
    }
    private function add(id:Int)
    {
        addTile(new Object(id));
    }
}