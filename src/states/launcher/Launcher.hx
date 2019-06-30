package states.launcher;
import openfl.display.Bitmap;
import openfl.display.Shape;
import lime.ui.FileDialog;
import openfl.net.URLRequest;
import openfl.display.Sprite;
import format.SVG;
import openfl.Assets;
import openfl.events.MouseEvent;
class Launcher extends states.State
{
    var text:ui.Text;
    public function new()
    {
        super();
        for(j in 0...3)
        {
            for(i in 0...3)
            {
                var tile = new Tile();
                tile.x = 58 + (55 + 250) * i;
                tile.y = 60 - 20 + 352 * j;
                addChild(tile);
            }
        }

        text = new ui.Text();
        text.color = 0xFFFFFF;
        addChild(text);
    }
    override function update() {
        super.update();
    }
}