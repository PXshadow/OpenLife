package data.object;
import haxe.io.Input;
import sys.FileSystem;
import sys.io.File;
import haxe.ds.Vector;
import data.animation.AnimationData;
import data.sound.SoundData;
class ObjectData extends LineReader
{
    /**
     * nextObjectNumber int to add generated objects in
     */
    public static var nextObjectNumberInt:Int = 0;
    /**
     * Max clothing pieces
     */
    public static inline var CLOTHING_PIECES:Int = 6;
    /**
     * Id of object
     */
    public var id:Int=0;
    /**
     * Description of object
     */
    public var description:String = "";
    /**
     * Whether an object is containable
     */
    public var containable:Bool=false;
    /**
     * N/A
     */
    public var containSize:Int = 0;
    /**
     * N/A
     */
    public var noFlip:Bool = false;
    /**
     * N/A
     */
    public var sideAcess:Bool = false;
    /**
     * rotation of slots
     */
    public var vertSlotRot:Float = 0.000000;
    /**
     * N/A
     */
    public var permanent:Int = 0;
    /**
     * Min age to be able to pick up object
     */
    public var minPickupAge:Int = 0;
    /**
     * Is held in hand boolean
     */
    public var heldInHand:Bool = false;
    /**
     * Rideable boolean
     */
    public var rideable:Bool = false;
    /**
     * Boolean if object blocks walking
     */
    public var blocksWalking:Bool = false;
    /**
     * N/A
     */
    public var leftBlockingRadius:Int = 0;
    /**
     * N/A
     */
    public var rightBlockingRadius:Int = 0;
    /**
     * If Object is drawn 
     */
    public var drawBehindPlayer:Bool = false;
    /**
     * Chance of object to appear
     */
    public var mapChance:Float = 0.000000;//#biomes_0
    /**
     * Heat value of object
     */
    public var heatValue:Int =0;
    /**
     * N/A
     */
    public var rValue:Float =0.000000;
    /**
     * If person
     */
    public var person:Bool = false;
    /**
     * N/A
     */
    public var noSpawn:Bool = false;
    /**
     * If person object is male
     */
    public var male:Bool=false;//=0
    /**
     * N/A
     */
    public var deathMarker:Int =0;
    /**
     * N/A
     */
    public var homeMarker:Int = 0;
    /**
     * If object is floor
     */
    public var floor:Bool = false;
    /**
     * N/A
     */
    public var floorHugging:Bool = false;
    /**
     * Increase value to player's food if object is eaten
     */
    public var foodValue:Int =0;
    /**
     * Multiply speed of player if held
     */
    public var speedMult:Float =1.000000;
    /**
     * Position offset of object when held
     */
    public var heldOffset:Point;//=0.000000,0.000000
    /**
     * N/A
     */
    public var clothing:String="n";
    /**
     * weapon can not be dropped
     */
     public var neverDrop:Bool = false;
    /**
     * Offset of object when worn
     */
    public var clothingOffset:Point;//=0.000000,0.000000
    /**
     * Deadly distance in tiles
     */
    public var deadlyDistance:Int=0;
    /**
     * Use distance in tiles
     */
    public var useDistance:Int=1;
    /**
     * N/A
     */
    public var creationSoundInitialOnly:Bool = false;
    /**
     * N/A
     */
    public var creationSoundForce:Bool = false;
    /**
     * Num of slots in object
     */
    public var numSlots:Int=0;
    /**
     * N/A
     */
    public var timeStretch:Float=1.000000;
    /**
     * N/A
     */
    public var slotSize:Int=1;
    /**
     * N/A
     */
    public var slotsLocked:Int = 1;
    /**
     * postion of slots
     */
    public var slotPos:Vector<Point>;
    /**
     * N/A
     */
    public var slotVert:Vector<Bool>;
    /**
     * N/A
     */
    public var slotParent:Vector<Int>;
    /**
     * Number of sprites in object
     */
    public var numSprites:Int=0;
    /**
     * Array of sprite data
     */
    public var spriteArray:Vector<SpriteData>;


    /**
     * Index of head for object
     */
    public var headIndex:Int = -1;
    /**
     * Index of body for object
     */
    public var bodyIndex:Int = -1;
    /**
     * Index of back foot for object
     */
    public var backFootIndex:Int = -1;
    /**
     * Index of front foot for object
     */
    public var frontFootIndex:Int = -1;
    // derrived automatically for person objects from sprite name
    // tags (if they contain Eyes or Mouth)
    // only filled in if sprite bank has been loaded before object bank
    /**
     * Generated index from sprite name of eye index
     */
    public var eyesIndex:Int = -1;
    /**
     * Generated index from sprite of mouth index
     */
    public var mouthIndex:Int = -1;
    //eyes offset
    //derrived automatically from whatever eyes are visible at age 30
    //(old eyes may have wrinkles around them, so they end up
    //getting centered differently)
    /**
     * Generated index from sprites of eye index
     */
    public var eyesOffset:Point = null;

    /**
     * Number of uses for object
     */
    public var numUses:Int = 0;
    /**
     * Number of chances for object
     */
    public var useChance:Int = 0;
    /**
     * Vanish index array
     */
    public var useVanishIndex:Array<Int>;
    /**
     * Appear index array
     */
    public var useAppearIndex:Array<Int>;
    // -1 if not set
    // used to avoid recomputing height repeatedly at client/server runtime

    /**
     * Cached height of object
     */
    public var cacheHeight:Int = -1;


    /**
     * N/A
     */
    public var apocalypseTrigger:Bool = false;
    /**
     * N/A
     */
    public var monumentStep:Bool = false;
    /**
     * N/A
     */
    public var monumentDone:Bool = false;
    /**
     * N/A
     */
    public var monumentCall:Bool = false;

    /**
     * N/A
     */
    public var toolSetIndex:Int = 0;
    /**
     * N/A
     */
    public var toolLearned:Bool = false;

    /**
     * Whether the object failed to load
     */
    public var fail:Bool = false;
    //animation
    #if openfl
    /**
     * Animation arrays of sprites
     */
    public var animation:AnimationData;
    /**
     * Sound array for objects
     */
    public var sounds:Vector<SoundData>;//=-1:0.250000,-1:0.250000,-1:0.250000,-1:1.000000
    #end
    /**
     * dummy bool for generated object
     */
    public var dummy:Bool = false;
    /**
     * dummy parent id
     */
    public var dummyParent:Int = 0;
    /**
     * dummyIndex, the amount of uses
     */
    public var dummyIndex:Int = 0;
    /**
     * New Object Data
     * @param i id
     */
    public function new(i:Int=0)
    {
        super();
        if (i <= 0) return;
        try {
            readLines(File.getContent(Static.dir + "objects/" + i + ".txt"));
        }catch(e:Dynamic)
        {
            trace("object txt e " + e);
            return;
        }
        //setup animation
        #if openfl
        animation = new AnimationData(i);
        if (animation.fail) animation = null;
        #end
        read();
    }
    /**
     * Read data to set
     */
    public inline function read()
    {
        id = getInt();
        description = getString();
        //tool setup
        var toolPos = description.indexOf("+tool");
        if (toolPos > -1)
        {
            var setTag = description.substring(toolPos + 5,description.length);
            var set:Bool = false;
            for (record in toolsetRecord)
            {
                if (record.setTag == setTag)
                {
                    //already exists
                    if (record.setMembership.indexOf(id) == -1)
                    {
                        record.setMembership.push(id);
                        set = true;
                        break;
                    }
                }
            }
            //new
            if (!set) toolsetRecord.push({setTag: setTag, setMembership: [id]});
        }
        containable = getBool();

        var i = getArrayInt();
        containSize = i[0];
        vertSlotRot = i[1];

        i = getArrayInt();
        permanent = i[0];
        minPickupAge = i[0];

        if(readName("noFlip"))
        {
            noFlip = getBool();
        }
        if(readName("sideAccess"))
        {
            sideAcess = getBool();
        }

        var string = getString();
        if (string == "1") heldInHand = true;
        if (string == "2") rideable = true;

        i = getArrayInt();
        blocksWalking = i[0] == 1 ? true : false;
        leftBlockingRadius = i[1];
        rightBlockingRadius = i[2];
        drawBehindPlayer = i[3] == 1 ? true : false;

        //skipping map chance
        getString();
        //values
        heatValue = getInt();
        rValue = getInt();

        i = getArrayInt();
        //person is the race of the person
        person = i[0] == 1 ? true : false;
        noSpawn = i[1] == 1 ? true : false;

        male = getBool();

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
            floor = getBool();
        }
        if(readName("floorHugging"))
        {
            floorHugging = getBool();
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
            #if openfl
            var array = getStringArray();
            sounds = new Vector<SoundData>(array.length);
            for (i in 0...array.length) sounds[i] = new SoundData(array[i]);
            #else
            getString();
            #end
        }

        if(readName("creationSoundInitialOnly")) creationSoundInitialOnly = getBool();
        if(readName("creationSoundForce")) creationSoundForce = getBool();

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
            var array = getArrayInt();
            spriteArray[j].invisHolding = array[0];
            spriteArray[j].invisWorn = array[1];
            spriteArray[j].behindSlots = array[2] == 1 ? true : false;
            if (readName("invisCont"))  spriteArray[j].invisCont = getInt();
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
        backFootIndex = getInt();
        frontFootIndex = getInt();
        #end
        if(next < line.length)
        {
            numUses = getInt();
            if (next < line.length) useVanishIndex = getIntArray();
            if (next < line.length) useAppearIndex = getIntArray();
            if (next < line.length) cacheHeight = getInt();
            //set num uses everything else should be set for cloning
            if (numUses > 1)
            {
                var dummyObject:ObjectData;
                for (j in 0...numUses - 1) 
                {
                    dummyObject = clone();
                    dummyObject.numUses = 0;
                    dummyObject.dummy = true;
                    dummyObject.dummyParent = id;
                    dummyObject.dummyIndex = j + 1;
                    Main.data.objectMap.set(++nextObjectNumberInt,dummyObject);
                }
            }
        }
        Main.data.objectMap.set(id,this);
    }
    public function getSpriteData()
    {
        //get sprite data
        for(i in 0...spriteArray.length)
        {
            if (spriteArray[i].spriteID <= 0) continue;
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
    /**
     * clone data
     */
    public function clone():ObjectData
    {
        return Reflect.copy(this);
    }
    /**
     * Toolset record set
     */
    public static var toolsetRecord:Array<ToolSetRecord> = [];
}