package ui;
#if openfl
class InputText extends Text
{
    public function new()
    {
        super();
        color = 0xFFFFFF;
        border = true;
        size = 24;
        height = 30;
        width = 400;
        borderColor = 0xFFFFFF;
        cacheAsBitmap = false;
        type = INPUT;
        border = true;
        selectable = true;
        mouseEnabled = true;
    }
}
#end