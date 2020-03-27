package debug;

import openfl.display.Shape;
class Grid extends Shape
{
    public function new()
    {
        super();
        //cacheAsBitmap = true;
        render();
    }
    private function render()
    {
        graphics.lineStyle(2,0);
        var shift = -16.5;
        for (x in 0...32)
        {
            for (y in 0...32)
            {
                graphics.moveTo((x + 0 + shift) * Static.GRID, (y + 1 + shift) * Static.GRID);
                graphics.lineTo((x + 0 + shift) * Static.GRID, (y + 0 + shift) * Static.GRID);
                graphics.lineTo((x + 1 + shift) * Static.GRID, (y + 0 + shift) * Static.GRID);
            }
        }
    }
}