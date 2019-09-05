import haxe.Timer;

class UnitTest
{
    private static var time:Float = 0;
    private static var time2:Float = 0;
    public static function inital()
    {
        time = Timer.stamp();
    }
    public static function stamp():Float
    {
        time2 = Timer.stamp() - time;
        time += time2;
        return time2;
    }
}