package openlife.server;
import haxe.Serializer;
import sys.FileSystem;
import haxe.macro.Expr.Catch;
import haxe.Exception;
import sys.io.File;
import openlife.settings.ServerSettings;
import openlife.data.transition.TransitionData;
import sys.thread.Mutex;
import openlife.data.object.ObjectHelper;
import format.png.Reader;
#if (target.threaded)
import openlife.data.map.MapData;
import haxe.ds.Vector;
import openlife.data.FractalNoise;
import openlife.data.object.ObjectData;

import haxe.io.Bytes;

@:enum abstract BiomeTag(Int) from Int to Int
{
    public var GREEN = 0;
    public var SWAMP = 1;
    public var YELLOW = 2;
    public var GREY = 3;
    public var SNOW = 4;
    public var DESERT= 5;
    public var JUNGLE = 6; //6 
    public var BORDERJUNGLE = 15; // 8 or 15  

    public var SNOWINGREY = 21; //7 // its snow on top of mountains which should not be walkable
    public var OCEAN = 9;  //deep ocean
    public var PASSABLERIVER = 13;
    public var RIVER = 17;  // TODO deep river which is not walkable 
}

@:enum abstract BiomeMapColor(String) from String to String
{
    public var CGREEN = "FFB5E61D";  
    public var CSWAMP = "FF008080";  
    public var CYELLOW = "FFFECC36"; //savannah
    public var CGREY = "FF808080"; //badlands // bevor it was: FF404040
    public var CSNOW = "FFFFFFFF";
    public var CDESERT= "FFDBAC4D"; 
    public var CJUNGLE = "FF007F0E";
    public var CBORDERJUNGLE = "FF007F00";  
    
    public var CSAND = "FFefe4b0";

    public var CSNOWINGREY = "FF404040"; // its snow on top of mountains which should not be walkable
    public var COCEAN = "FF004080"; //deep ocean 
    public var CRIVER = "FF0080FF"; //shallow water
    public var CPASSABLERIVER = "FF00E8FF"; // TODO use also for passable ocean? otherwise use biomeID: 22??? 
}

@:enum abstract BiomeSpeed(Float) from Float to Float
{
    // var truncMovementSpeedDiff = 0.1;
    // considered as bad biome for horses if speed < 0.999
    // TODO make fast for specialists 
    public var SGREEN = 1;  
    public var SSWAMP = 0.6;  
    public var SYELLOW = 1;
    public var SGREY = 0.98; 
    public var SSNOW = 0.98; 
    public var SDESERT= 0.98; 
    public var SJUNGLE = 0.98;  
    public var SCBORDERJUNGLE = 0.98; 

    public var SSNOWINGREY = 0.01;
    public var SOCEAN = 0.01;  
    public var SRIVER = 0.01;
    public var SPASSABLERIVER = 0.6;   
}

class WorldMap
{
    public var mutex = new Mutex();

    var objects:Vector<Array<Int>>;
    var originalObjects:Vector<Array<Int>>;
    var objectHelpers:Vector<ObjectHelper>; 
    var floors:Vector<Int>;
    var biomes:Vector<Int>;
    var originalBiomes:Vector<Int>;

    public var originalObjectsCount:Map<Int,Int>;
    public var currentObjectsCount:Map<Int,Int>;


    public var width:Int;
    public var height:Int;
    private var length:Int;

    // stuff for random generator
    private var seed:Int = 38383834;
    static inline final MULTIPLIER:Float = 48271.0;
    static inline final MAX_NUM:Int = 2147483647;
    static inline final MODULUS:Int = MAX_NUM;

    var saveDataNumber = 0;
    var backupDataNumber = 0;

    public function new()
    {

    }

    public static var world(get, set):WorldMap;

    public static function get_world() {
        return Server.server.map;
    }

    public static function set_world(world) {
        return Server.server.map = world;
    }

    public function generateExtraDebugStuff(tx:Int, ty:Int)
    {
        setFloorId(tx - 3, ty - 2, 1596); // stone road
        setFloorId(tx - 3, ty - 3, 1596); // stone road
        setFloorId(tx - 3, ty - 4, 1596); // stone road
        setFloorId(tx - 3, ty - 5, 1596); // stone road
        setFloorId(tx - 3, ty - 6, 1596); // stone road
        setFloorId(tx - 3, ty - 7, 1596); // stone road
        setFloorId(tx - 4, ty - 7, 1596); // stone road
        setFloorId(tx - 5, ty - 7, 1596); // stone road
        setFloorId(tx - 6, ty - 7, 1596); // stone road

        setObjectId(tx - 3, ty - 2, [0]); //  clear road
        setObjectId(tx - 3, ty - 3, [0]); //  clear road
        setObjectId(tx - 3, ty - 4, [0]); //  clear road
        setObjectId(tx - 3, ty - 5, [0]); //  clear road
        setObjectId(tx - 3, ty - 6, [0]); //  clear road
        setObjectId(tx - 3, ty - 7, [0]); //  clear road
        setObjectId(tx - 4, ty - 7, [0]); //  clear road
        setObjectId(tx - 5, ty - 7, [0]); //  clear road
        setObjectId(tx - 6, ty - 7, [0]); //  clear road

        setObjectId(tx - 8,ty-3,[3159]); // Hitched Horse-Drawn Tire Cart
        setObjectId(tx - 8,ty-2,[774]); // Hitched Riding Horse
        setObjectId(tx - 8,ty-2,[779]); // Hitched Horse-Drawn Cart
        setObjectId(tx - 8,ty-1,[779]); // Hitched Horse-Drawn Cart
        setObjectId(tx - 7,ty-1,[331]); // Hot Steel Axe Head
        setObjectId(tx - 6,ty-1,[334]); // Axe
        setObjectId(tx - 5,ty-2,[767]); //Lasso
        setObjectId(tx - 5,ty-1,[769]); // Wild Horse
        setObjectId(tx - 4,ty-1,[391]); // Domestic Gooseberry Bush
        setObjectId(tx - 4,ty-2,[391]); // Domestic Gooseberry Bush
        setObjectId(tx - 3,ty-1,[1121]); // popcorn
        setObjectId(tx - 3,ty,[3900]); // onion pile
        setObjectId(tx - 2,ty-1,[2742]); // carrot pile
        setObjectId(tx - 2,ty,[2742]); // carrot pile
        setObjectId(tx - 1,ty,[3371,1251,1251,245]); // table with stew
        setObjectId(tx - 1,ty-1,[3371,291,807,107]); // table flat stone / burdock / stakes 
        setObjectId(tx - 1,ty-1,[3371,441,309,309]); // table smithing hammer / Hot Iron Bloom on Flat Rock / Hot Iron Bloom on Flat Rock 
        
        
        setObjectId(tx - 3, ty-3, [461]); // saw
        setObjectId(tx - 2, ty-3, [336]);
        setObjectId(tx - 1, ty-3, [211]);
        
        setObjectId(tx, ty, [33]);
        setObjectId(tx+1, ty, [32]);
        setObjectId(tx+2, ty, [486]);
        setObjectId(tx+3, ty, [486]);
        setObjectId(tx+4, ty, [677]);
        setObjectId(tx+5, ty, [684]);
        setObjectId(tx+6, ty, [677]);

        // sheares with pink rose
        setObjectId(tx+6, ty, [3842]);
        

        // add some clothing for testing
        setObjectId(tx, ty+1, [2916]);
        setObjectId(tx+1, ty+1, [2456]);
        setObjectId(tx+2, ty+1, [766]);
        setObjectId(tx-1, ty+1, [2919]);
        setObjectId(tx-2, ty+1, [198]);
        setObjectId(tx-3, ty+1, [2886]);
        setObjectId(tx-4, ty+1, [586]);
        setObjectId(tx-5, ty+1, [2951]);


        // pond
        setObjectId(tx - 4,ty + 3,[511]);
        setObjectId(tx - 5,ty + 3,[235]);
        setObjectId(tx - 6,ty + 3,[659]);
        setObjectId(tx - 7,ty + 3,[659]);
        setObjectId(tx - 8,ty + 3,[659]);
        setObjectId(tx - 9,ty + 3,[659]);
        setObjectId(tx - 10,ty + 3,[659]);

        // horses and carts
        setObjectId(tx - 11,ty + 3,[659]);
        setObjectId(tx - 12,ty + 3,[774]); // riding horse 
        setObjectId(tx - 13,ty + 3,[484]); // cart
        setObjectId(tx - 16,ty + 3,[1422]); // escaped horse cart

        // test movement restriction
        setObjectId(tx,ty - 3,[775]); // escaped riding horse 
        setObjectId(tx - 0,ty - 2,[887]); // wall
        setObjectId(tx - 2,ty - 2,[887]); // wall
        setObjectId(tx + 1,ty - 2,[887]); // wall
        setObjectId(tx - 0,ty - 4,[887]);
        setObjectId(tx - 1,ty - 4,[887]); // wall
        setObjectId(tx + 1,ty - 4,[887]); // wall
        setObjectId(tx - 0,ty - 5,[0]);
        setObjectId(tx - 1,ty - 3,[887]);
        setObjectId(tx + 1,ty - 3,[887]);
        

        // spring / tool use
        setObjectId(tx - 2,ty + 4,[1096]); // well site
        setObjectId(tx - 3,ty + 4,[1096]); // well site
        setObjectId(tx - 3,ty + 3,[0]); // 0
        setObjectId(tx - 4,ty + 4,[3030]); // natural spring
        setObjectId(tx - 5,ty + 4,[661]);
        setObjectId(tx - 6,ty + 4,[661]);
        setObjectId(tx - 7,ty + 4,[661]);
        setObjectId(tx - 8,ty + 4,[334]);
        setObjectId(tx - 9,ty + 4,[502]);

        // test time / decay transitions
        setObjectId(tx - 4,ty + 5,[248]);
        setObjectId(tx - 5,ty + 5,[82]);
        setObjectId(tx - 6,ty + 5,[418]);

        //test transitions of numUses + decay
        setObjectId(tx,ty + 10,[238]);
        setObjectId(tx,ty + 11,[1599]);

        //containers testing SREMV
        setObjectId(tx - 4,ty + 7,[434]);
        setObjectId(tx - 5,ty + 7,[292,2143,2143,2143]);
        setObjectId(tx - 6,ty + 7,[292,2143,2143,2143]);
        setObjectId(tx - 7,ty + 7,[292,33,2143,33]);
        setObjectId(tx - 8,ty + 7,[2143,2143,2143]);
        // table
        setObjectId(tx - 9,ty + 7,[3371,33,2143,33]);
        setObjectId(tx - 10,ty + 7,[3371,2873,2873,245]);
        setObjectId(tx - 11,ty + 7,[3371,1251,1251,245]);
    }
    
    
    private function generateSeed():Int
    {
        return seed = Std.int((seed * MULTIPLIER) % MODULUS);
    }

    public static function calculateRandomInt(maxInt:Int)
    {
        return Server.server.map.randomInt(maxInt);
    }

    public function randomInt(x:Int=MAX_NUM):Int
    {
        return Math.floor(generateSeed() / MODULUS * (x + 1));
    }

    public static function calculateRandomFloat():Float
    {
        return Server.server.map.randomFloat();
    }

    public function randomFloat():Float
    {
        return generateSeed() / MODULUS;
    }

    private function createVectors(length:Int)
    {
        this.length = length;

        objects = new Vector<Array<Int>>(length);
        objectHelpers = new Vector<ObjectHelper>(length);
        floors = new Vector<Int>(length);
        biomes = new Vector<Int>(length);
        
        //timeObjectHelpers = [];

        originalObjectsCount = new Map<Int,Int>();
        currentObjectsCount = new Map<Int,Int>();
    }

    public function getBiomeSpeed(x:Int, y:Int):Float 
    {
        var biomeType = biomes[index(x, y)];

        //trace('${ x },${ y }:BI ${ biomeType }');

        return switch biomeType {
            case GREEN: SGREEN;
            case SWAMP: SSWAMP;
            case YELLOW: SYELLOW;
            case GREY: SGREY;
            case SNOW: SSNOW;
            case DESERT: SDESERT;
            case JUNGLE: SJUNGLE;
            case BORDERJUNGLE: SCBORDERJUNGLE;
            case SNOWINGREY: SSNOWINGREY;
            case OCEAN: SOCEAN;
            case RIVER: SRIVER;
            case PASSABLERIVER: SPASSABLERIVER;
            default: 1;
        }
    } 

    public static function isBiomeBlocking(x:Int, y:Int) : Bool
    {
        var biomeSpeed = Server.server.map.getBiomeSpeed(x,y);

        return biomeSpeed < 0.1;
    }

    public static function worldGetBiomeId(x:Int, y:Int):Int
    {
        return Server.server.map.getBiomeId(x,y);
    }

    public function getBiomeId(x:Int, y:Int):Int
    {
        return biomes[index(x,y)];
    }

    public function setBiomeId(x:Int, y:Int, biomeId:Int)
    {
        return biomes[index(x,y)] = biomeId;
    }
    
    public static function worldGetObjectId(x:Int, y:Int) : Array<Int>
        {
            return Server.server.map.getObjectId(x,y);
        }

    public function getObjectId(x:Int, y:Int):Array<Int>
    {
        return objects[index(x,y)];
    }

    public function getOriginalObjectId(x:Int, y:Int) : Array<Int>
    {
        return originalObjects[index(x,y)];
    }

    public function setObjectId(x:Int, y:Int, ids:Array<Int>)
    {
        objects[index(x,y)] = ids;

        if(ids.length > 1)
        {
            // set object Helper, otherwiese stuff in containers will not be saved
            setObjectHelper(x,y, ObjectHelper.readObjectHelper(null, ids));
        }
        else
        {
            // TODO create time transition
            setObjectHelperNull(x, y);
        }
    }

    public static function worldGetObjectHelper(x:Int, y:Int, allowNull:Bool = false):ObjectHelper
    {
        return Server.server.map.getObjectHelper(x, y, allowNull);
    }

    public function getObjectHelper(x:Int, y:Int, allowNull:Bool = false):ObjectHelper
    {
        //trace('objectHelper: $x,$y');
        var helper = objectHelpers[index(x,y)];   

        if(helper != null || allowNull) return helper;

        helper = ObjectHelper.readObjectHelper(null, getObjectId(x , y));
        helper.tx = x;
        helper.ty = y;

        return helper;
    }

    public function setObjectHelperNull(x:Int, y:Int)
    {
        objectHelpers[index(x,y)] = null;
    }

    // sets objectHelper and also Object Ids on same Tile
    public function setObjectHelper(x:Int, y:Int, helper:ObjectHelper)
    {
        if(helper != null) helper.TransformToDummy();

        //trace('objectHelper: $x,$y');
        objectHelpers[index(x,y)] = helper;

        if(helper == null)
        {
            objects[index(x,y)] = [0]; 
            return; 
        }

        var ids = helper.toArray();
        objects[index(x,y)] = ids; 

        helper.tx = x;
        helper.ty = y;

        // TODO set time to chage if it has time transition

        if(deleteObjectHelperIfUseless(helper)) return;
    }

    // to save space keep ObjectHelper only if used to store number of uses, or has time transition...
    // ... or has owner or is a container or has a groundObject (used if a animal walks on an object)
    // TODO dont delete stuff with owners like a gate 
    public function deleteObjectHelperIfUseless(helper:ObjectHelper) : Bool
    {
        var obj = getObjectId(helper.tx, helper.ty);

        if(obj[0] != helper.dummyId())
        {
            WorldMap.world.mutex.acquire();

            try
            {
                // test again after receiving mutex
                var obj = getObjectId(helper.tx, helper.ty);

                if(obj[0] != helper.dummyId())
                {
                    // TODO look who sets the dummy wrong
                    var objData = ObjectData.getObjectData(obj[0]);

                    trace('WARNING: ${helper.tx},${helper.ty} object Id: ${obj[0]} ${objData.description} did not fit to object.dummyId: ${helper.dummyId()} helper.id: ${helper.id} ${helper.description}');

                    objectHelpers[index(helper.tx, helper.ty)] = null;

                    setObjectId(helper.tx, helper.ty, helper.toArray());
                }
            }
            catch(ex) trace(ex);

            WorldMap.world.mutex.release();
        }

        if(isHelperToBeDeleted(helper))
        {
            WorldMap.world.mutex.acquire();

            try
            {
                helper = getObjectHelper(helper.tx, helper.ty);

                if(isHelperToBeDeleted(helper))
                {
                    // test again after receiving mutex
                    //if(x != helper.tx || y != helper.ty) trace('REMOVE ObjectHelper $x,$y h${helper.tx},h${helper.ty} USES < 1 && timeToChange == 0 && containedObjects.length == 0 && groundObject == null');
                    objectHelpers[index(helper.tx, helper.ty)] = null;
                }
            }
            catch(ex) trace(ex);

            WorldMap.world.mutex.release();

            return true;
        }
        
        return false;
    }

    public static function isHelperToBeDeleted(helper:ObjectHelper) : Bool
    {
        return ((helper.numberOfUses == helper.objectData.numUses || helper.numberOfUses < 1) && helper.timeToChange == 0 && helper.containedObjects.length == 0 && helper.groundObject == null);
    }

    public function getFloorId(x:Int, y:Int):Int
    {
        return floors[index(x,y)];
    }

    public function setFloorId(x:Int, y:Int, floor:Int)
    {
        floors[index(x,y)] = floor;
    }

    private inline function index(x:Int,y:Int):Int
    {
        // Dont know why yet, but y seems to be right if -1
        y -= 1;
        // make map round x wise
        x = x % this.width;
        if(x < 0) x += this.width; 
        //else if(x >= this.width) x -= this.width;

        // make map round y wise
        y = y % this.height;
        if(y < 0) y += this.height; 
        //else if(y >= this.height) y -= this.height;

        var i = x + y * width;

        return i;
    }

    // The Server and Client map is saved in an array with y starting from bottom, 
    // The Map is saved with y starting from top. Therefore the map is y inversed during generation from picture
    public function generate()
    {
        this.mutex.acquire();

        var pngDir = './${ServerSettings.MapFileName}';// "./map.png";
        var pngmap = readPixels(pngDir);

        width = pngmap.width;
        height = pngmap.height;
        length = width * height;       
        
        trace('map width: ' + width);
        trace('map height: ' + height);

        createVectors(length);

        for (y in 0...height){
            for (x in 0...width) {
                if(y % 100 == 0 && x == 0){
                    trace('generating map up to y: ' + y);
                }
                
                //var p = pngmap.data.getInt32(4*(x+xOffset+(y+yOffset)*pngmap.width));
                var p = pngmap.data.getInt32(4*(x + ((height - 1) - y) * pngmap.width));

                // ARGB, each 0-255
                //var a:Int = p>>>24;
                //var r:Int = (p>>>16)&0xff;
                //var g:Int = (p>>>8)&0xff;
                //var b:Int = (p)&0xff;
                // Or, AARRGGBB in hex:
                var hex:String = StringTools.hex(p,8);
                var biomeInt;

                switch hex {
                    case CYELLOW: biomeInt = YELLOW;
                    case CGREY: biomeInt = GREY;
                    case CSNOW: biomeInt = SNOW;
                    case CDESERT: biomeInt = DESERT;
                    case CSAND: biomeInt = DESERT;
                    case CJUNGLE: biomeInt = JUNGLE;
                    case CBORDERJUNGLE: biomeInt = BORDERJUNGLE;
                    case CSWAMP: biomeInt = SWAMP;
                    case CSNOWINGREY: biomeInt = SNOWINGREY;
                    case COCEAN: biomeInt = OCEAN;
                    case CRIVER: biomeInt = RIVER;
                    case CPASSABLERIVER: biomeInt = PASSABLERIVER;
                    default: biomeInt = GREEN;
                }
                if(biomeInt == YELLOW){
                    //trace('${ x },${ y }:BI ${ biomeInt },${ r },${ g },${ b } - ${ StringTools.hex(p,8) }');
                }

                //biomeInt = x % 30;

                biomes[x+y*width] = biomeInt;           
            }
        } 

        addExtraBiomes();

        generateObjects();

        generateExtraStuff();
        
        this.originalBiomes = biomes.copy();

        this.originalObjects = objects.copy();

        if(ServerSettings.debug) generateExtraDebugStuff(ServerSettings.startingGx, ServerSettings.startingGy);

        this.mutex.release();      
    }

    public function writeBackup()
    {
        var tmpBackupDataNumber =  (backupDataNumber % ServerSettings.MaxNumberOfBackups) + 1;

        var dir = './${ServerSettings.SaveDirectory}/$tmpBackupDataNumber/';

        writeToDisk(false, dir);

        trace('Wrote backup: backupDataNumber: $tmpBackupDataNumber');
        backupDataNumber++;
    }

    public function writeToDisk(saveOriginals:Bool = true, dir:String = null)
    {        
        this.mutex.acquire();        

        var time = Sys.time();
        if(dir == null) dir = './${ServerSettings.SaveDirectory}/';
        
        if(FileSystem.exists(dir) == false) FileSystem.createDirectory(dir);
        
        var tmpDataNumber = (saveDataNumber % 10) + 1;
        
        if(saveOriginals) writeMapBiomes(dir + ServerSettings.OriginalBiomesFileName + ".bin", originalBiomes);

        if(saveOriginals) writeMapObjects(dir + ServerSettings.OriginalObjectsFileName + ".bin", originalObjects);

        writeMapBiomes(dir + ServerSettings.CurrentBiomesFileName  + tmpDataNumber + ".bin", biomes);

        writeMapFloors(dir + ServerSettings.CurrentFloorsFileName  + tmpDataNumber + ".bin", floors);

        writeMapObjects(dir + ServerSettings.CurrentObjectsFileName + tmpDataNumber + ".bin", objects);

        writeMapObjHelpers(dir + ServerSettings.CurrentObjHelpersFileName + tmpDataNumber + ".bin", objectHelpers);
        
        var path = dir + "lastDataNumber.txt";
        var writer = File.write(path, false);
        writer.writeString('$tmpDataNumber\n');
        writer.writeString('$backupDataNumber\n');
        writer.writeString('${TimeHelper.tick}\n');
        writer.close();

        saveDataNumber++;
         
        this.mutex.release();

        var time = Math.round((Sys.time() - time) * 100) / 100;

        if(ServerSettings.DebugWrite) trace('Write to disk: saveDataNumber: $tmpDataNumber Time: $time backupDataNumber: $backupDataNumber tick: ${TimeHelper.tick}');
    } 

    public function readFromDisk() : Bool
    {
        this.mutex.acquire();

        try
        {
            var dir = './${ServerSettings.SaveDirectory}/';
            var path = dir + "lastDataNumber.txt";
            var reader = File.read(path, false);
            this.saveDataNumber = Std.parseInt(reader.readLine());
            this.backupDataNumber = Std.parseInt(reader.readLine());
            TimeHelper.tick = Std.parseFloat(reader.readLine());
            reader.close();    

            trace('saveDataNumber: $saveDataNumber backupDataNumber: $backupDataNumber tick: ${TimeHelper.tick}');        

            this.originalBiomes = readMapBiomes(dir + ServerSettings.OriginalBiomesFileName + ".bin");

            this.originalObjects = readMapObjects(dir + ServerSettings.OriginalObjectsFileName + ".bin");

            this.biomes = readMapBiomes(dir + ServerSettings.CurrentBiomesFileName + saveDataNumber + ".bin");

            this.floors = readMapBiomes(dir + ServerSettings.CurrentFloorsFileName + saveDataNumber + ".bin");

            this.objects = readMapObjects(dir + ServerSettings.CurrentObjectsFileName + saveDataNumber + ".bin");

            this.objectHelpers = readMapObjHelpers(dir + ServerSettings.CurrentObjHelpersFileName + saveDataNumber + ".bin");

            this.originalObjectsCount = countObjects(this.originalObjects);

            this.currentObjectsCount = countObjects(this.objects);
        }
        catch(ex)
        {
            trace(ex);
            return false;
        }

        this.mutex.release();

        return true;
    }

    public function writeMapBiomes(path:String, biomesToWrite:Vector<Int>)
    {
        //trace('Wrtie to file: $path width: $width height: $height length: $length');

        if(width * height != length) throw new Exception('width * height != length');
        if(biomesToWrite.length != length) throw new Exception('biomesToWrite.length != length');

        var writer = File.write(path, true);
        var dataVersion = 1;

        writer.writeInt32(dataVersion);
        writer.writeInt32(width);
        writer.writeInt32(height);

        for(biome in biomesToWrite)
        {
            writer.writeInt8(biome);
        }

        writer.close();
    }

    public function readMapBiomes(path:String) : Vector<Int>
    {
        var reader = File.read(path, true);
        var dataVersion = reader.readInt32();
        this.width = reader.readInt32();
        this.height = reader.readInt32();
        this.length = width * height;
        var newBiomes = new Vector<Int>(length);

        trace('Read from file: $path width: $width height: $height length: $length');

        if(width * height != length) throw new Exception('width * height != length');
        
        for(i in 0...newBiomes.length)
        {
            newBiomes[i] = reader.readInt8();
        }

        reader.close();

        return newBiomes;
    }

    public function writeMapFloors(path:String, floorsToWrite:Vector<Int>)
    {
        //trace('Wrtie to file: $path width: $width height: $height length: $length');

        if(width * height != length) throw new Exception('width * height != length');
        if(floorsToWrite.length != length) throw new Exception('floorsToWrite.length != length');

        var writer = File.write(path, true);
        var dataVersion = 1;

        writer.writeInt32(dataVersion);
        writer.writeInt32(width);
        writer.writeInt32(height);

        for(floor in floorsToWrite)
        {
            writer.writeInt32(floor);
        }

        writer.close();
    }

    public function readMapFloors(path:String) : Vector<Int>
    {
        var reader = File.read(path, true);
        var dataVersion = reader.readInt32();
        var width = reader.readInt32();
        var height = reader.readInt32();
        var length = width * height;
        var newFloors = new Vector<Int>(length);

        trace('Read from file: $path width: $width height: $height length: $length');

        if(width != this.width) throw new Exception('width != this.width');
        if(height != this.height) throw new Exception('height != this.height');
        if(length != this.length) throw new Exception('length != this.length');

        for(i in 0...newFloors.length)
        {
            newFloors[i] = reader.readInt32();
        }

        reader.close();

        return newFloors;
    }

    public function writeMapObjects(path:String, objectsToWrite:Vector<Array<Int>>)
    {
        //trace('Wrtie to file: $path width: $width height: $height length: $length');
        if(objectsToWrite.length != length) throw new Exception('objectsToWrite.length != length');

        var writer = File.write(path, true);
        var dataVersion = 1;

        writer.writeInt32(dataVersion);        
        writer.writeInt32(width);
        writer.writeInt32(height);

        for(obj in objectsToWrite)
        {
            writer.writeInt32(obj[0]);
        }

        writer.close();
    }

    public function readMapObjects(path:String) : Vector<Array<Int>>
    {
        var reader = File.read(path, true);
        var dataVersion = reader.readInt32();
        var width = reader.readInt32();
        var height = reader.readInt32();
        var length = width * height;
        var newObjects = new Vector<Array<Int>>(length);
        
        trace('Read from file: $path width: $width height: $height length: $length');

        if(width != this.width) throw new Exception('width != this.width');
        if(height != this.height) throw new Exception('height != this.height');
        if(length != this.length) throw new Exception('length != this.length');
        
        for(i in 0...newObjects.length)
        {
            newObjects[i] = [reader.readInt32()];
        }

        reader.close();

        return newObjects;
    }

    public function writeMapObjHelpers(path:String, objHelpersToWrite:Vector<ObjectHelper>)
    {
        //trace('Wrtie to file: $path width: $width height: $height length: $length');

        if(width * height != length) throw new Exception('width * height != length');
        if(objHelpersToWrite.length != length) throw new Exception('objHelpersToWrite.length != length');

        var count = 0;
        var writer = File.write(path, true);
        var dataVersion = 2;

        writer.writeInt32(dataVersion);        
        writer.writeInt32(width);
        writer.writeInt32(height);        

        for(obj in objHelpersToWrite)
        {
            if(obj == null) continue;

            count++;

            var objArray = obj.toArray();

            writer.writeInt8(objArray.length);

            for(i in objArray)
            {
                writer.writeInt32(i);
            }

            writer.writeInt32(obj.tx);
            writer.writeInt32(obj.ty);
            writer.writeInt32(obj.numberOfUses);
            writer.writeDouble(obj.creationTimeInTicks);
            writer.writeInt32(obj.timeToChange);
        }

        writer.writeInt8(100); // end sign

        writer.close();

        if(ServerSettings.DebugWrite) trace('wrote $count ObjectHelpers...');
    }

    public function readMapObjHelpers(path:String) : Vector<ObjectHelper>
    {
        var reader = File.read(path, true);
        var expectedDataVersion = 2;
        var dataVersion = reader.readInt32();
        var width = reader.readInt32();
        var height = reader.readInt32();
        var length = width * height;
        var newObjects = new Vector<ObjectHelper>(length);
        var count = 0;
        

        if(dataVersion != 2) throw new Exception('Data version is: $dataVersion expected data version is: $expectedDataVersion');
        if(width != this.width) throw new Exception('width != this.width');
        if(height != this.height) throw new Exception('height != this.height');
        if(length != this.length) throw new Exception('length != this.length');

        trace('Read from file: $path width: $width height: $height length: $length');

        try{
            while(reader.eof() == false)
            {
                var arrayLength = reader.readInt8();
                if(arrayLength == 100) break; // reached the end
                if(arrayLength > 100) throw new Exception('array length is: $arrayLength > 100');
                count++;

                var newObjArray = new Array<Int>();

                for(i in 0...arrayLength)
                {
                    newObjArray.push(reader.readInt32());
                }

                var newObject = ObjectHelper.readObjectHelper(null, newObjArray);
                newObject.tx = reader.readInt32();
                newObject.ty = reader.readInt32();
                newObject.numberOfUses = reader.readInt32();
                newObject.creationTimeInTicks = reader.readDouble();
                newObject.timeToChange = reader.readInt32();

                if(newObject.creationTimeInTicks > TimeHelper.tick) newObject.creationTimeInTicks = TimeHelper.tick;

                if(newObject.numberOfUses > 1 || newObject.containedObjects.length > 0)
                {
                    // 1435 = bison // 1261 = Canada Goose Pond with Egg // 30 = Gooseberry Bush // 2142 = Banana Plant // 1323 = Wild Boar
                    if(newObject.id != 1435 && newObject.id != 1261  && newObject.id != 30 && newObject.id != 2142 && newObject.id != 1323)
                    {
                        // trace('${newObject.description()} numberOfUses: ${newObject.numberOfUses} from  ${newObject.objectData.numUses} ' + newObjArray);
                    }
                }

                newObjects[index(newObject.tx, newObject.ty)] = newObject;
                objects[index(newObject.tx, newObject.ty)] = newObjArray;
            }
        }
        catch(ex)
        {
            reader.close();
            throw ex;
        }

        reader.close();

        trace('read $count ObjectHelpers...');

        return newObjects;
    }

    public function updateObjectCounts() 
    {
        //var time = Sys.time();

        this.currentObjectsCount = countObjects(objects, objectHelpers);

        //trace('count objects time: ${ Sys.time() - time}');

        if(ServerSettings.TraceCountObjects)
        {
            for(key in currentObjectsCount.keys()){
                var objData = ObjectData.getObjectData(key);
                trace('Count object: [${key}] ${objData.description}: ${currentObjectsCount[key]} original: ${originalObjectsCount[key]}');
            }
        }
    }

    public function countObjects(objectsToCount:Vector<Array<Int>>, objHelpersToCount:Vector<ObjectHelper> = null) :  Map<Int, Int>
    {
        var objList = new Map<Int, Int>();
        
        for(obj in objectsToCount)
        {
            if(obj[0] == 0) continue;

            var objData = ObjectData.getObjectData(obj[0]);

            objList[objData.parentId]++;
        }

        if(objHelpersToCount == null) return objList;

        for(obj in objHelpersToCount)
        {
            if(obj == null) continue;

            for(containedObj in obj.containedObjects)
            {
                objList[containedObj.parentId]++;

                for(subContainedObj in containedObj.containedObjects)
                {
                    objList[subContainedObj.parentId]++;
                }
            }
        }

        return objList;
    }

    function addExtraBiomes()
    {
        var dist = ServerSettings.CreateGreenBiomeDistance;
        var tmpIsPlaced = new Vector<Bool>(length);

        for (y in 0...height)
        {
            for (x in 0...width)
            {
                if(tmpIsPlaced[index(x,y)]) continue;

                var biome = getBiomeId(x,y);

                if(biome == BiomeTag.RIVER || biome == BiomeTag.PASSABLERIVER || biome == BiomeTag.JUNGLE || biome == BiomeTag.SWAMP)
                {
                    for(ix in -dist...dist+1)
                    {
                        for(iy in -dist...dist+1)
                        {
                            var tmpX = x + ix;
                            var tmpY = y + iy;

                            if(tmpIsPlaced[index(tmpX,tmpY)]) continue;
                            var nextBiome = getBiomeId(tmpX, tmpY);
                            
                            if((biome == BiomeTag.RIVER || biome == BiomeTag.PASSABLERIVER) && ix * ix < 2 && iy * iy < 2)
                            {
                                if(nextBiome == BiomeTag.GREEN || nextBiome == BiomeTag.YELLOW || nextBiome == BiomeTag.DESERT || nextBiome == BiomeTag.RIVER)
                                {
                                    //trace('$ix,$iy biome: $biome nextBiome: $nextBiome ');
                                    if(biome == BiomeTag.PASSABLERIVER || nextBiome != BiomeTag.RIVER)
                                    {
                                        //trace('SET!!! $ix,$iy biome: $biome nextBiome: $nextBiome ');
                                        setBiomeId(tmpX, tmpY, BiomeTag.PASSABLERIVER); 
                                        tmpIsPlaced[index(tmpX, tmpY)] = true;
                                    }
                                }
                            }
                            else
                            {
                                if(nextBiome == BiomeTag.YELLOW || nextBiome == BiomeTag.DESERT)
                                {
                                     setBiomeId(tmpX, tmpY, BiomeTag.GREEN);                                 
                                     //tmpIsPlaced[index(tmpX, tmpY)] = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    function generateObjects()
    {
        var generatedObjects = 0;
        originalObjectsCount = new Map<Int, Int>();
        currentObjectsCount = new Map<Int, Int>();

        for (y in 0...height)
        {
            for (x in 0...width)
            {
                var biomeInt = biomes[x+y*width];

                objects[x+y*width] = [0];
                //if(x+y*width < 10000) objects[x+y*width] = [4746];
                 
                //if(x < 200 || x > 600) continue;

                // if there is a object below allready continue
                if(y > 0 && objects[x+(y-1)*width][0] != 0) continue;

                if (randomFloat() > 0.4) continue;
                
                
                var set:Bool = false;

                var biomeData = ObjectData.biomeObjectData[biomeInt];

                if(biomeData == null) continue;

                var random = randomFloat() * ObjectData.biomeTotalChance[biomeInt]; 
                var sumChance = 0.0;
                
                for (obj in biomeData) {
                    if (set) continue;
                    var chance = obj.mapChance;
                    sumChance += chance;

                    if (random <= sumChance)
                    {
                        objects[x+y*width] = [obj.id];

                        originalObjectsCount[obj.id] += 1;
                        currentObjectsCount[obj.id] += 1; 
                                            
                        //trace('generate: bi: $biomeInt id: ${obj.id} rand: $random sc: $sumChance');
                        set = true;
                        generatedObjects++;                      
                    }
                }
            }
        }

        trace('generatedObjects: $generatedObjects');      

        if(ServerSettings.TraceCountObjects)
        {
            for(key in originalObjectsCount.keys()){
                var objData = ObjectData.getObjectData(key);
                trace('Generated obj[${key}] ${objData.description}: ${originalObjectsCount[key]}');
            }
        }
    }

    function generateExtraStuff()
    {
        var tmpIsPlaced = new Vector<Bool>(length);


        for (y in 0...height)
        {
            for (x in 0...width)
            {
                var obj = objects[x+y*width];

                // change muddy iron vein to loose muddy iron vein // TODO better patch the data
                if(obj[0] == 942) // iron vein
                {
                    objects[x+y*width] = [3962]; // loose muddy iron vein

                    // generate also some random stones and some more mines nearby

                    var random = randomInt(6) + 3;

                    for(i in 0...100)
                    {
                        var dist = 8;
                        var tx = x + randomInt(dist * 2) - dist;
                        var ty = y + randomInt(dist * 2) - dist; 


                        if(((tx -x) * (tx - x)) + ((ty - y) * (ty - y)) > dist * dist) continue;

                        if(biomes[tx+ty*width] != BiomeTag.GREY && biomes[tx+ty*width] != BiomeTag.YELLOW) continue; 
                        if(objects[tx+ty*width][0] != 0) continue;

                        objects[tx+ty*width] = [503];
                        //tmpIsPlaced[index(tx,ty)] = true;

                        random -= 1;
                        if(random <= 0) break;
                    }

                    /*
                    var random = randomInt(4);
                    if(random == 1 || random == 3) random += 1;
                    for(i in 0...50)
                    {
                        var dist = 5;
                        var tx = x + randomInt(dist * 2) - dist;
                        var ty = y + randomInt(dist * 2) - dist; 

                        if(((tx - x) * (tx - x)) + ((ty - y) * (ty - y)) > dist * dist) continue;

                        if(biomes[tx+ty*width] != BiomeTag.GREY) continue; 

                        objects[tx+ty*width] = [3962];
                        //tmpIsPlaced[index(tx,ty)] = true;

                        random -= 1;
                        if(random <= 0) break;
                    }*/
                } 

                var tmpObj = getObjectId(x,y);

                if(tmpObj[0] == 0) continue;

                if(tmpIsPlaced[index(x,y)]) continue; 

                // if obj is no iron, no tary spot and no spring there is a chance for winning lottery
                if(ServerSettings.CanObjectRespawn(tmpObj[0]) == false) continue;

                if(randomFloat() < ServerSettings.ChanceForLuckySpot)
                {
                    //var objData = ObjectData.getObjectData(tmpObj[0]);
                    var timeTransition = Server.transitionImporter.getTransition(-1, tmpObj[0], false, false);
                    var random = 2 + randomInt(timeTransition != null ? 3 : 8);

                    var tmpRandom = random;               

                    for(i in 0...100)
                    {
                        var dist = 10;
                        var tx = x + randomInt(dist * 2) - dist;
                        var ty = y + randomInt(dist * 2) - dist; 

                        if(((tx - x) * (tx - x)) + ((ty - y) * (ty - y)) > dist * dist) continue;
                        
                        var biomeId = getBiomeId(x,y);

                        if(biomeId != getBiomeId(tx,ty)) continue; 

                        if(getObjectId(tx,ty)[0] != 0) continue;
                        if(getObjectId(tx,ty-1)[0] != 0) continue;

                        tmpIsPlaced[index(tx,ty)] = true;
                        setObjectId(tx,ty, tmpObj);


                        random -= 1;
                        if(random <= 0) break;
                    }
                    
                    //trace('lucky: ${objData.description} placed: ${tmpRandom-random} from $tmpRandom');
                }
            }
        }
    }    
    
    function readPixels(file:String):{data:Bytes, width:Int, height:Int} {
        var handle = sys.io.File.read(file, true);
        var d = new Reader(handle).read();
        var hdr = format.png.Tools.getHeader(d);
        var ret = {
            data:format.png.Tools.extract32(d),
            width:hdr.width,
            height:hdr.height
        };
        handle.close();
        return ret;
    }

    public function getChunk(x:Int,y:Int,width:Int,height:Int):WorldMap
    {
        var map = new WorldMap();
        var length = width * height;
        map.createVectors(length);
        for (px in 0...width)
        {
            for (py in 0...height)
            {
                var localIndex = px + py * width;
                var index = index(x + px, y + py); 
                
                map.biomes[localIndex] = biomes[index];
                map.floors[localIndex] = floors[index];
                map.objects[localIndex] = objects[index];
            }
        }
        return map;
    }  

    private inline function sigmoid(input:Float,knee:Float):Float
    {
        var shifted = input * 2 -1;
        var sign = input < 0 ? -1 : 1;
        var k = -1 - knee;
        var abs = Math.abs(shifted);
        var out = sign * abs * k / (1 + k - abs);
        return (out + 1) * 0.5;
    }

    public function toString():String
    {
        var string = "";
        //trace("length: ");
        //trace(length);
        for (i in 0...length)
        {
            var obj = MapData.stringID(objects[i]);
            string += ' ${biomes[i]}:${floors[i]}:$obj';
        }
        return string.substr(1);
    }

    public function findClosest(){
        
    }
}
#end