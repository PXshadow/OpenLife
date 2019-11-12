package data;
#if openfl
class SoundData
{
    public var i:Int = 0;
    public var multi:Float = 0;
    public function new(string:String)
    {
        var array = string.split(":");
        var i:Int = Std.parseInt(array[0]);
        multi = Std.parseFloat(array[1]);
    }
}
enum SoundType
{
    creation;
    //using conflicts with haxe
    use;
    eating;
    decay;
}
#end