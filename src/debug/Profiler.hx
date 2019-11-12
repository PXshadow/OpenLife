package debug;
#if openfl
import openfl.system.System;
import openfl.display.DisplayObjectContainer;
import openfl._internal.renderer.context3D.stats.Context3DStats;
#end
class Profiler #if openfl extends DisplayObjectContainer #end
{
    private var currentTime:Float;
	private var times:Array<Float> = [];
    private var cacheCount:Int = 0;
    public var fps:Float = 0;
    public function new()
    {
        #if openfl super(); #end
        UnitTest.inital();
    }
    #if (openfl && cpp)
    public function update()
    {
        //frameRate
        currentTime += UnitTest.stamp();
        times.push(currentTime);
        while (times[0] < currentTime - 1000) times.shift();
        fps = (times.length + cacheCount)/2;
        cacheCount = times.length;
    }
    public function drawCalls():Int
    {
        return Context3DStats.totalDrawCalls();
    }
    public function memory():Int
    {
        return System.totalMemory;
    }
    #end
    public static function start(string:String) {}
    public static function stop() {}
}