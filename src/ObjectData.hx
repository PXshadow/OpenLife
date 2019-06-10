import openfl.geom.Point;
import sys.io.File;
import haxe.ds.Vector;
class ObjectData
{
    public var id:Int=0;
    public var description:String = "";
    public var containable:Int=0;
    public var containSize:Int = 0;
    public var noFlip:Bool = false;
    public var sideAcess:Bool = false;
    public var vertSlotRot:Float = 0.000000;
    public var permanent:Int = 0;
    public var minPickupAge:Int = 0;
    public var heldInHand:Bool = false;
    public var rideable:Bool = false;
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
    public var homeMarker:Int = 0;
    public var floor:Int = 0;
    public var floorHugging:Bool = false;
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
    public var slotsLocked:Int = 1;
    public var slotPos:Vector<Point>;
    public var slotVert:Vector<Bool>;
    public var slotParent:Vector<Int>;
    public var numSprites:Int=6;
    public var spriteArray:Vector<SpriteData>;

    public var headIndex:Int = -1;
    public var bodyIndex:Int = -1;
    public var backFootIndex:Array<Int> = [];
    public var frontFootIndex:Array<Int> = [];

    public var numUses:Int = 0;
    public var useVanishIndex:Int = 0;
    public var useAppearIndex:Int = 0;
    public var pixHeight:Int = 0;

    //vars for reading
    var line:Vector<String>;
    var next:Int = 0;
    public function new(i:Int)
    {
        line = Static.readLines(File.read(Settings.assetPath + "objects/" + i + ".txt"));
        id = getInt();
        description = getString();
        containable = getInt();

        var i = getArrayInt();
        containSize = i[0];
        vertSlotRot = i[1];

        i = getArrayInt();
        permanent = i[0];
        minPickupAge = i[0];

        if(readName("noFlip"))
        {
            noFlip = getString() == "1" ? true : false;
        }
        if(readName("sideAccess"))
        {
            sideAcess = getString() == "1" ? true : false;
        }

        var string = getString();
        if (string == "1") heldInHand = true;
        if (string == "2") rideable = true;

        i = getArrayInt();
        blocksWalking = i[0];
        leftBlockingRadius = i[1];
        rightBlockingRadius = i[2];
        drawBehindPlayer = i[3];

        //skipping map chance
        getString();
        //values
        heatValue = getInt();
        rValue = getInt();

        i = getArrayInt();
        //person is the race of the person
        person = i[0];
        noSpawn = i[1];

        male = getString() == "1" ? true : false;

        deathMarker = getInt();

        //from death (I don't know what this does)
        if(readName("fromDeath"))
        {
            trace("from death " + line[next]);
        }
        if(readName("homeMarker"))
        {
            homeMarker = getInt();
        }
        if(readName("floor"))
        {
            floor = getInt();
        }
        if(readName("floorHugging"))
        {
            floorHugging = getString() == "1" ? true : false;
        }

        foodValue = getInt();
        speedMult = getFloat();

        heldOffset = getPoint();

        clothing = getString();
        clothingOffset = getPoint();

        deadlyDistance = getInt();

        if(readName("useDistance"))
        {
            useDistance = getInt();
        }
        if(readName("sounds"))
        {
            //not setup
            getString();
        }
        if(readName("creationSoundInitialOnly"))
        {
            //not setup
            getString();
        }
        if(readName("creationSoundForce"))
        {
            //not setup
            getString();
        }

        //num slots and time stretch
        string = getString();
        string = string.substring(0,string.indexOf("#"));
        numSlots = Std.parseInt(string);

        slotSize = getInt();
        if(readName("slotsLocked"))
        {
            //trace("slot lock");
            slotsLocked = getInt();
        }
        slotPos = new Vector<Point>(numSlots);
        slotVert = new Vector<Bool>(numSlots);
        slotParent = new Vector<Int>(numSlots);
        var set:Int = 0;
        for(j in 0...numSlots)
        {
            string = getString();
            set = string.indexOf(",");
            slotPos[j] = new Point(
                Std.parseInt(string.substring(0,set)),
                Std.parseInt(string.substring(set + 1,set = string.indexOf(",",set)))
            );
            set = string.indexOf("=",set) + 1;
            slotVert[j] = string.substring(set,set = string.indexOf(",",set)) == "1" ? true : false;
            set = string.indexOf("=",set) + 1;
            slotParent[j] = Std.parseInt(string.substring(set,string.length));
        }
        numSprites = getInt();
        spriteArray = new Vector<SpriteData>(numSprites);
        for(j in 0...numSprites)
        {
            spriteArray[j] = new SpriteData();
            spriteArray[j].spriteID = getInt();
            spriteArray[j].pos = getPoint();
            spriteArray[j].rot = getFloat();
            spriteArray[j].hFlip = getInt();
            spriteArray[j].color = getFloatArray();
            spriteArray[j].ageRange = getFloatArray();
            spriteArray[j].parent = getInt();
            //invis holding, invisWorn, behind slots
            getString();
        }
        //get offset center
        getSpriteData();

        //extra
        if(readName("spritesDrawnBehind"))
        {
            throw("sprite drawn behind " + line[next]);
        }
        if(readName("spritesAdditiveBlend"))
        {
            throw("sprite additive blend " + line[next]);
        }
        
        headIndex = getInt();
        bodyIndex = getInt();
        //arrays
        backFootIndex = getIntArray();
        frontFootIndex = getIntArray();
        
        if(next < line.length)
        {
            numUses = getInt();
            if (next < line.length) useVanishIndex = getInt();
            if (next < line.length) useAppearIndex = getInt();
            if (next < line.length) pixHeight = getInt();

        }
    }
    public function getSpriteData()
    {
        //get sprite data
        for(i in 0...spriteArray.length)
        {
            var input = File.read(Settings.assetPath + "sprites/" + spriteArray[i].spriteID + ".txt",false);
            var j:Int = 0;
            var a = input.readLine().split(" ");
            for(string in a)
            {
                switch(j++)
                {
                    case 0:
                    //name

                    case 1:
                    //multitag

                    case 2:
                    //centerX
                    spriteArray[i].inCenterXOffset = Std.parseInt(string);
                    case 3:
                    //centerY
                    spriteArray[i].inCenterYOffset = Std.parseInt(string);
                }              
            }
        }
    }
    public function getFloatArray():Array<Float>
    {
        var array:Array<Float> = [];
        for(o in getStringArray())
        {
            array.push(Std.parseFloat(o));
        }
        return array;
    }
    public function getIntArray():Array<Int>
    {
        var array:Array<Int> = [];
        for(o in getStringArray())
        {
            array.push(Std.parseInt(o));
        }
        return array;
    }
    public function getStringArray():Array<String>
    {
        return getString().split(",");
    }
    public function getPoint():Point
    {
        var string = getString();
        var comma:Int = string.indexOf(",");
        return new Point(Std.parseInt(string.substring(0,comma)),Std.parseInt(string.substring(comma + 1,string.length)));
    }
    public function getInt():Int
    {
        return Std.parseInt(getString());
    }
    public function getArrayInt():Array<Int>
    {
        var array:Array<Int> = [];
        var string = line[next++];
        var i:Int = 0;
        var j:Int = 0;
        var bool:Bool = true;
        while(bool)
        {
            i = string.indexOf("=",i + 1);
            j = string.indexOf(",",i);
            j = j < 0 ? string.length : j;
            array.push(Std.parseInt(string.substring(i,j)));
            if(j == string.length) bool = false;
        }
        return array;
    }
    public function getFloat():Float
    {
        return Std.parseFloat(getString());
    }
    public function getString():String
    {
        var string = line[next++];
        var equals = string.indexOf("=");
        return string.substring(equals + 1,line.length);
    }
    public function readName(name:String):Bool
    {
        var string = line[next];
        if(name == string.substring(0,name.length)) return true;
        return false;
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
    //added
    public var inCenterXOffset:Int = 0;
    public var inCenterYOffset:Int = 0;
    public function new()
    {

    }
}