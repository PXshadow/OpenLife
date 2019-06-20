import openfl.display.Sprite;
import format.SVG;
import openfl.Assets;
import openfl.events.MouseEvent;
class Launcher extends Sprite
{
    public function new()
    {
        super();
        var item = new Item();
        item.x = 58 + 55 * 0;
        item.y = 94 - 20 + 352 * 0;

        addChild(item);
    }
}
class Item extends Sprite
{
    public function new()
    {
        super();
        //rect
        graphics.beginFill(0x292929);
        graphics.drawRect(0,0,250,100);
        graphics.beginFill(0x121212);
        graphics.drawRect(0,100,250,200);

        new SVG(Assets.getText("assets/bookmark.svg")).render(graphics,0 + 4,0 + 4);
        new SVG(Assets.getText("assets/settings.svg")).render(graphics,216 - 4,4);

        new SVG(Assets.getText("assets/link.svg")).render(graphics,15,268);
        new SVG(Assets.getText("assets/code.svg")).render(graphics,55,272);
        new SVG(Assets.getText("assets/play.svg")).render(graphics,115,263);
        new SVG(Assets.getText("assets/note.svg")).render(graphics,163,272);
        new SVG(Assets.getText("assets/folder.svg")).render(graphics,217,272);

        addEventListener(MouseEvent.MOUSE_MOVE,function(_)
        {
            if(mouseY > 260)
            {
                buttonMode = true;
            }else{
                buttonMode = false;
            }
        });
    }
}