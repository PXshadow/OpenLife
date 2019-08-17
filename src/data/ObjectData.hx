package data;
import haxe.io.Input;
import sys.FileSystem;
import sys.io.File;
import haxe.ds.Vector;
enum ObjectType {
    OBJECT;
    FLOOR;
    PLAYER;
    GROUND;
}
class ObjectData extends LineReader
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
    public var numSprites:Int=0;
    public var spriteArray:Vector<SpriteData>;

    public var headIndex:Int = -1;
    public var bodyIndex:Int = -1;
    public var backFootIndex:Array<Int> = [];
    public var frontFootIndex:Array<Int> = [];

    public var numUses:Int = 0;
    public var useVanishIndex:Int = 0;
    public var useAppearIndex:Int = 0;
    public var pixHeight:Int = 0;

    public var fail:Bool = false;
    //animation
    public var animation:AnimationData;
    public function new(i:Int=0)
    {
        super();
        if (i <= 0) throw("objectData id less than 1: " + i);
        try {
            line = readLines(File.getContent(Static.dir + "objects/" + i + ".txt"));
        }catch(e:Dynamic)
        {
            trace("object txt e " + e);
            return;
        }
        //setup animation
        animation = new AnimationData(i);
        if (animation.fail) animation = null;
        read();
    }
    public function read()
    {
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
        #if openfl
        //visual
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
            //throw("sprite drawn behind " + line[next]);
            next++;
        }
        if(readName("spritesAdditiveBlend"))
        {
            //throw("sprite additive blend " + line[next]);
            next++;
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
        #end
    }
    public function getSpriteData()
    {
        //get sprite data
        for(i in 0...spriteArray.length)
        {
            var s:String;
            try { 
                s = File.getContent(Static.dir + "sprites/" + spriteArray[i].spriteID + ".txt");
            }catch(e:Dynamic)
            {
                trace("sprite text e " + e);
                //continue;
                return;
            }
            var j:Int = 0;
            var a = s.split(" ");
            for(string in a)
            {
                switch(j++)
                {
                    case 0:
                    //name
                    spriteArray[i].name = string;
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
}