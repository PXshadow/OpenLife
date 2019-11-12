package ui;
class InputText extends Text
{
    public function new()
    {
        super();
        cacheAsBitmap = false;
        type = INPUT;
        border = true;
        selectable = true;
        mouseEnabled = true;
    }
}