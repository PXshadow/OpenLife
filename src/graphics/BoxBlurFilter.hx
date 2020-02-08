package graphics;

import haxe.io.Bytes;

/**
 * https://github.com/jasonrohrer/minorGems/blob/master/graphics/filters/BoxBlurFilter.h
 */
class BoxBlurFilter
{
    public function new(radius:Int=12,channel:Bytes,width:Int,height:Int)
    {
        var total = width * height;
        var accumulate = total;
        var pointer = channel;
        for (y in 0...height)
        {
            for (x in 0...width)
            {
                
            }
        }
    }
}