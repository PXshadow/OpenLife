package editor;

import data.object.player.PlayerInstance;
import data.object.ObjectData;
import ui.Text;
import openfl.display.Tile;
import game.Game;
import openfl.events.TextEvent;
import ui.InputText;
import game.Objects;
import openfl.display.Sprite;

class Inspector
{
    var objects:Objects;
    var sprite:Sprite;
    var idInput:InputText;
    public function new(objects:Objects,sprite:Sprite)
    {
        sprite.stage.color = 0x00FF00;
        this.objects = objects;
        this.sprite = sprite;

        idInput = new InputText();
        sprite.addChild(idInput);
        var num = sprite.numChildren + 1;
        idInput.addEventListener(TextEvent.TEXT_INPUT,function(e:TextEvent)
        {
            var id = Std.parseInt(idInput.text + e.text);
            if (id > Game.data.nextObjectNumber || id <= 0)
            {
                trace("id out of range");
                return;
            }
            objects.group.removeTiles();
            sprite.removeChildren(num,sprite.numChildren);
            var data = objects.get(id);
            if (data.person)
            {
                var instance = new PlayerInstance([]);
                instance.age = 20;
                instance.po_id = data.id;
                instance.x = 2;
                instance.y = Static.tileHeight - 2;
                objects.addPlayer(instance);
            }else{
                objects.add([id],2,Static.tileHeight - 2);
            }
            sprites(data);
            trace("t " + id);
        });
        sprite.addChild(idInput);
    }
    private function sprites(data:ObjectData)
    {
        var sy:Float = 40;
        var sx:Float = 330;
        if (data == null || data.spriteArray == null) return;
        for (sprite in data.spriteArray)
        {
            var id = @:privateAccess objects.cacheSprite(sprite.spriteID);
            var tile = new Tile(id);
            tile.x = sx;
            tile.y = sy;
            var text = new Text();
            text.text = Std.string(sprite.spriteID);
            text.y = sy - 24;
            text.x = sx;
            this.sprite.addChild(text);
            trace("height " + objects.tileset.getRect(id).height);
            sy += objects.tileset.getRect(id).height + 20;
            if (sy > objects.stage.height - Static.GRID)
            {
                sy = 0;
                sx += Static.GRID;
            }
            objects.group.addTile(tile);
        }
    }
}