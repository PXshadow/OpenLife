package debug;

import openfl.display.Tile;
import data.object.ObjectData;
import game.Objects;

class ObjectSpriteViewer
{
    public function new(id:Int,objects:Objects)
    {
        //objects.add()
        var sy:Float = 0;
        var sx:Float = 0;
        var objectData = objects.get(id);
        for (sprite in objectData.spriteArray)
        {
            id = @:privateAccess objects.cacheSprite(sprite.spriteID);
            var tile = new Tile(id);
            tile.x = sx;
            tile.y = sy;
            trace("height " + objects.tileset.getRect(id).height);
            sy += objects.tileset.getRect(id).height + 8;
            objects.addTile(tile);
        }
    }
}