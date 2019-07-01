package data;

#if cpp 
typedef Int64 = cpp.UInt64;
#elseif hl
typedef Int64 = hl.UI64;
#else
typedef Int64 = Float; 
#end
class MapGen
{
                        // = 2147483647
    var XX_PRIME32_1:Int64 = 265443576;
    var XX_PRIME32_2:Int64 = 2246822519;
    var XX_PRIME32_3:Int64 = 3266489917;
    var XX_PRIME32_4:Int64 = 668265263;
    var XX_PRIME32_5:Int64 = 374761393;
    var xxSeed:Int64 = 0;
    public function new()
    {

    }
    private function setXYRandomSeed(inSeed:Int64) 
    {
        xxSeed = inSeed;
    }
    private function getXYFractal(inX:Int64,inY:Int64,inRoughness:Int64,inScale:Int64)
    {
        var b:Int64 = inRoughness;
        var a:Int64 = 1- b;
        var sum:Int64 = 0;
    }
    private function getXYRandomBN(inX:Int64,inY:Int64)
    {
        var floorX:Int64 = Math.floor(inX);
        var ceilX:Int64 = floorX + 1;
        var floorY:Int64 = Math.floor(inY);
        var ceilY:Int64 = floorY + 1;

        //var cornerA1 =
    }
    private function xxTweakedHash2D(inX:Int64,inY:Int64):Int64
    {
        var h32:Int64 = xxSeed + inX + XX_PRIME32_5;
        h32 += inY * XX_PRIME32_3;
        h32 *= XX_PRIME32_2;
        h32 = Std.int(h32)^Std.int(h32) >> 13;
        h32 *= XX_PRIME32_3;
        h32 = Std.int(h32)^Std.int(h32) >> 16;
        return h32;
    }
}