import openfl.geom.Point;
import sys.io.File;
class ObjectData
{
    public var id:Int=0;
    public var name:String = "";
    public var containable:Int=0;
    public var containSize:Int = 0;
    public var vertSlotRot:Float = 0.000000;
    public var permanent:Int = 0;
    public var minPickupAge:Int = 0;
    public var heldInHand:Int = 0;
    public var blocksWalking:Int = 0;
    public var leftBlockingRadius:Int = 0;
    public var rightBlockingRadius:Int = 0;
    public var drawBehindPlayer:Int = 0;
    public var mapChance:Float = 0.000000;//#biomes_0
    public var heatValue:Int =0;
    public var rValue:Float =0.000000;
    public var person:Int =0;
    public var noSpawn:Int =0;
    //int -> bool
    public var male:Bool=false;//=0
    public var deathMarker:Int =0;
    public var foodValue:Int =0;
    public var speedMult:Float =1.000000;
    public var heldOffset:Point;//=0.000000,0.000000
    public var clothing:String="n";
    public var clothingOffset:Point;//=0.000000,0.000000
    public var deadlyDistance:Int=0;
    public var useDistance:Int=1;
    public var sounds:Array<String> = [];//=-1:0.250000,-1:0.250000,-1:0.250000,-1:1.000000
    public var creationSoundInitialOnly:Int=0;
    public var numSlots:Int=0;
    public var timeStretch:Float=1.000000;
    public var slotSize:Int=1;
    public var numSprites:Int=6;
    public var spriteArray:Array<SpriteData> = [];
    public function new(i:Int)
    {
        var input = File.read(Settings.assetPath + "objects/" + i + ".txt");
        var line:String = "";
        var index:Int = 0;
        var spriteIndex:Int = 0;
        while(true)
        {
            try {
            line = input.readLine();
            }catch(e:Dynamic)
            {
                trace("e " + e);
                break;
            }
            switch(index++)
            {
                case 0:
                id = getInt(line);
                case 1:
                name = line;
                case 2:
                containable = getInt(line);
                case 3:
                containSize = getInt(line);
                case 4:
                vertSlotRot = getFloat(line);
                case 5:
                permanent = getInt(line);
                case 6:
                minPickupAge = getInt(line);
                case 7:
                heldInHand = getInt(line);
                case 8:
                blocksWalking = getInt(line);
                case 9:
                leftBlockingRadius = getInt(line);
                case 10:
                rightBlockingRadius = getInt(line);
                case 11:
                drawBehindPlayer = getInt(line);
                case 12:
                mapChance = getFloat(line);
                case 13:
                heatValue = getInt(line);
                case 14:
                rValue = getFloat(line);
                case 15:
                person = getInt(line);
                case 16:
                noSpawn = getInt(line);
                case 17:
                male = getInt(line) == 1 ? true : false;
                case 18:
                deathMarker = getInt(line);
                case 19:
                foodValue = getInt(line);
                case 20:
                speedMult = getFloat(line);
                case 21:
                heldOffset = getPoint(line);
                case 22:
                clothing = getString(line);
                case 23:
                clothingOffset = getPoint(line);
                case 24:
                deadlyDistance = getInt(line);
                case 25:
                useDistance = getInt(line);
                case 26:
                sounds = getStringArray(line);
                case 27:
                creationSoundInitialOnly = getInt(line);
                case 28:
                numSlots = getInt(line);
                default:
                if(index > 28)
                {
                    //sprite id
                    if(line.indexOf("spriteID") == 0)
                    {
                        spriteIndex = 0;
                        trace("new sprite");
                        spriteArray.push(new SpriteData());
                    }else{
                        var i = spriteArray.length - 1;
                        switch(spriteIndex++)
                        {
                            case 0:
                            //spriteArray[i].pos = getPoint(line);
                            case 2:
                            //spriteArray[i].rot = getFloat(line);
                            case 3:
                            //spriteArray[i].color = getFloatArray(line);
                            case 4:
                            //spriteArray[i].ageRange = getFloatArray(line);
                            case 5:
                            //spriteArray[i].parent = getInt(line);
                        }
                    }
                }
            }
        }
    }
    public function getFloatArray(line:String):Array<Float>
    {
        var array:Array<Float> = [];
        for(o in getStringArray(line))
        {
            array.push(Std.parseFloat(o));
        }
        return array;
    }
    public function getStringArray(line:String):Array<String>
    {
        return getString(line).split(",");
    }
    public function getPoint(line:String):Point
    {
        var comma:Int = line.indexOf(",");
        return new Point(Std.parseInt(line.substring(0,comma)),Std.parseInt(line.substring(comma + 1,line.length)));
    }
    public function getInt(line:String):Int
    {
        return Std.parseInt(getString(line));
    }
    public function getFloat(line:String):Float
    {
        return Std.parseFloat(getString(line));
    }
    public function getString(line:String):String
    {
        var equals = line.indexOf("=");
        return line.substring(equals + 1,line.length);
    }
}
class SpriteData
{
    public var spriteID:Int=54;
    public var pos:Point = new Point();//=166.000000,107.000000
    public var rot:Float=0.000000;
    public var hFlip:Int=0;
    public var color:Array<Float> = [];//=0.952941,0.796078,0.756863
    public var ageRange:Array<Float> = [];//=-1.000000,-1.000000
    public var parent:Int = 0;//=-1
    public function new()
    {

    }
}