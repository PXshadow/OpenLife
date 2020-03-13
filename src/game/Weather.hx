package game;

import openfl.display.Tile;

class Weather
{
    var objects:Objects;
    var list:Array<ObjectWeather> = [];
    public function new(objects:Objects)
    {
        this.objects = objects;
    }
    public function wind(index:Int=0,count:Int=20,init:Bool=true)
    {
        var obj:ObjectWeather;
        for (i in index...count)
        {
            if (init)
            {
                obj = new ObjectWeather(objects.create(objects.get(62),0,0)[0]);
            }else{
                obj = list[i];
            }
            obj.tile.y = Std.random(600) + 40;
            obj.tile.x = 40;
            obj.tile.originX = Std.random(80) - 40;
            obj.tile.originY = Std.random(80) - 40;
            obj.vx = Std.random(10);
            objects.addTile(obj.tile);
            list.push(obj);
        }
    }
    public function snow()
    {

    }
    public function rain()
    {

    }
    public function update()
    {
        for (obj in list)
        {
            if (obj.tile.x > 2400 || obj.tile.y > 2000) 
            obj.update();
        }
    }
}
class ObjectWeather
{
    //velocity
    public var vx:Float = 0;
    public var vy:Float = 0;
    public var vr:Float = 0;
    //acceleration
    public var ax:Float = 0;
    public var ay:Float = 0;
    public var ar:Float = 0;
    //damper
    public var dx:Float = 0;
    public var dy:Float = 0;
    public var dr:Float = 0;
    public var tile:Tile;
    public function new(tile:Tile)
    {
        this.tile = tile;
    }
    public function update()
    {

    }
}