package openlife.server;

import haxe.io.Float32Array;
import format.png.Reader;
import haxe.Exception;
import haxe.Int32;
import openlife.auto.AiBase;
import openlife.auto.PlayerInterface;
import openlife.data.object.ObjectHelper;
import openlife.data.transition.TransitionImporter;
import openlife.macros.Macro;
import openlife.settings.ServerSettings;
import sys.FileSystem;
import sys.io.File;
import sys.io.FileInput;
import sys.io.FileOutput;
import sys.thread.Mutex;
#if (target.threaded)
import haxe.ds.Vector;
import haxe.io.Bytes;
import openlife.data.map.MapData;
import openlife.data.object.ObjectData;
import openlife.server.Biome;

class WorldMap {
	public var mutex = new Mutex();

	var objects:Vector<Array<Int>>;
	var originalObjects:Vector<Array<Int>>;
	var hiddenObjects:Vector<Array<Int>>;

	public var objectHelpers:Vector<ObjectHelper>;

	var floors:Vector<Int>;
	var biomes:Vector<Int>;
	var originalBiomes:Vector<Int>;

	public var originalObjectsCount:Map<Int, Int>;
	public var currentObjectsCount:Map<Int, Int>;

	public var width:Int;
	public var height:Int;
	public var length:Int;

	// stuff for random generator
	private var seed:Int = 38383834;

	static inline final MULTIPLIER:Float = 48271.0;
	static inline final MAX_NUM:Int = 2147483647;
	static inline final MODULUS:Int = MAX_NUM;

	var saveDataNumber = 0;
	var backupDataNumber = 0;

	// possible spawn locations
	public var bananaPlants = new Map<Int, ObjectHelper>();
	public var berryBushes = new Map<Int, ObjectHelper>();
	public var wildCarrots = new Map<Int, ObjectHelper>();
	public var cactuses = new Map<Int, ObjectHelper>();
	public var wildGarlics = new Map<Int, ObjectHelper>();

	// possible teleport locations
	public var roads = new Map<Int, ObjectHelper>();
	public var ovens = new Map<Int, ObjectHelper>();
	public var cursedGraves = new Map<Int, ObjectHelper>();

	public var eatenFoodValues = new Map<Int, Float>();
	//public var eatenFoodCravings = new Map<Int, Float>();
	public var eatenFoodsYum = new Map<Int, Float>();
	public var eatenFoodsYumBoni = new Map<Int, Float>();
	public var eatenFoodsMeh = new Map<Int, Float>();
	public var eatenFoodsMehMali = new Map<Int, Float>();
	//public var eatenFoodsSuperMeh = new Map<Int, Float>();

	public function new() {}

	public static var world(get, set):WorldMap;

	public static function get_world() {
		return Server.server.map;
	}

	public static function set_world(world) {
		return Server.server.map = world;
	}

	public function generateExtraDebugStuff(tx:Int, ty:Int) {
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

		setObjectId(tx - 8, ty - 3, [3159]); // Hitched Horse-Drawn Tire Cart
		setObjectId(tx - 8, ty - 2, [774]); // Hitched Riding Horse
		setObjectId(tx - 8, ty - 2, [779]); // Hitched Horse-Drawn Cart
		setObjectId(tx - 8, ty - 1, [779]); // Hitched Horse-Drawn Cart
		setObjectId(tx - 7, ty - 1, [331]); // Hot Steel Axe Head
		setObjectId(tx - 6, ty - 1, [334]); // Axe
		setObjectId(tx - 5, ty - 2, [767]); // Lasso
		setObjectId(tx - 5, ty - 1, [769]); // Wild Horse
		setObjectId(tx - 4, ty - 1, [391]); // Domestic Gooseberry Bush
		setObjectId(tx - 4, ty - 2, [391]); // Domestic Gooseberry Bush
		setObjectId(tx - 3, ty - 1, [1121]); // popcorn
		setObjectId(tx - 3, ty, [3900]); // onion pile
		setObjectId(tx - 2, ty - 1, [2742]); // carrot pile
		setObjectId(tx - 2, ty, [2742]); // carrot pile
		setObjectId(tx - 1, ty, [3371, 1251, 1251, 245]); // table with stew
		setObjectId(tx - 1, ty - 1, [3371, 291, 807, 107]); // table flat stone / burdock / stakes
		setObjectId(tx - 1, ty - 1, [3371, 441, 309, 309]); // table smithing hammer / Hot Iron Bloom on Flat Rock / Hot Iron Bloom on Flat Rock

		setObjectId(tx - 3, ty - 3, [461]); // saw
		setObjectId(tx - 2, ty - 3, [336]);
		setObjectId(tx - 1, ty - 3, [211]);

		setObjectId(tx, ty, [33]);
		setObjectId(tx + 1, ty, [32]);
		setObjectId(tx + 2, ty, [486]);
		setObjectId(tx + 3, ty, [486]);
		setObjectId(tx + 4, ty, [677]);
		setObjectId(tx + 5, ty, [684]);
		setObjectId(tx + 6, ty, [677]);

		// sheares with pink rose
		setObjectId(tx + 6, ty, [3842]);

		// add some clothing for testing
		setObjectId(tx, ty + 1, [2916]);
		setObjectId(tx + 1, ty + 1, [2456]);
		setObjectId(tx + 2, ty + 1, [766]);
		setObjectId(tx - 1, ty + 1, [2919]);
		setObjectId(tx - 2, ty + 1, [198]);
		setObjectId(tx - 3, ty + 1, [2886]);
		setObjectId(tx - 4, ty + 1, [586]);
		setObjectId(tx - 5, ty + 1, [2951]);

		// pond
		setObjectId(tx - 4, ty + 3, [511]);
		setObjectId(tx - 5, ty + 3, [235]);
		setObjectId(tx - 6, ty + 3, [659]);
		setObjectId(tx - 7, ty + 3, [659]);
		setObjectId(tx - 8, ty + 3, [659]);
		setObjectId(tx - 9, ty + 3, [659]);
		setObjectId(tx - 10, ty + 3, [659]);

		// horses and carts
		setObjectId(tx - 11, ty + 3, [659]);
		setObjectId(tx - 12, ty + 3, [774]); // riding horse
		setObjectId(tx - 13, ty + 3, [484]); // cart
		setObjectId(tx - 16, ty + 3, [1422]); // escaped horse cart

		// test movement restriction
		setObjectId(tx, ty - 3, [775]); // escaped riding horse
		setObjectId(tx - 0, ty - 2, [887]); // wall
		setObjectId(tx - 2, ty - 2, [887]); // wall
		setObjectId(tx + 1, ty - 2, [887]); // wall
		setObjectId(tx - 0, ty - 4, [887]);
		setObjectId(tx - 1, ty - 4, [887]); // wall
		setObjectId(tx + 1, ty - 4, [887]); // wall
		setObjectId(tx - 0, ty - 5, [0]);
		setObjectId(tx - 1, ty - 3, [887]);
		setObjectId(tx + 1, ty - 3, [887]);

		// spring / tool use
		setObjectId(tx - 2, ty + 4, [1096]); // well site
		setObjectId(tx - 3, ty + 4, [1096]); // well site
		setObjectId(tx - 3, ty + 3, [0]); // 0
		setObjectId(tx - 4, ty + 4, [3030]); // natural spring
		setObjectId(tx - 5, ty + 4, [661]);
		setObjectId(tx - 6, ty + 4, [661]);
		setObjectId(tx - 7, ty + 4, [661]);
		setObjectId(tx - 8, ty + 4, [334]);
		setObjectId(tx - 9, ty + 4, [502]);

		// test time / decay transitions
		setObjectId(tx - 4, ty + 5, [248]);
		setObjectId(tx - 5, ty + 5, [82]);
		setObjectId(tx - 6, ty + 5, [418]);

		// test transitions of numUses + decay
		setObjectId(tx, ty + 10, [238]);
		setObjectId(tx, ty + 11, [1599]);

		// containers testing SREMV
		setObjectId(tx - 4, ty + 7, [434]);
		setObjectId(tx - 5, ty + 7, [292, 2143, 2143, 2143]);
		setObjectId(tx - 6, ty + 7, [292, 2143, 2143, 2143]);
		setObjectId(tx - 7, ty + 7, [292, 33, 2143, 33]);
		setObjectId(tx - 8, ty + 7, [292, 2143, 2143]);
		// table
		setObjectId(tx - 9, ty + 7, [3371, 33, 2143, 33]);
		setObjectId(tx - 10, ty + 7, [3371, 2873, 2873, 245]);
		setObjectId(tx - 11, ty + 7, [3371, 1251, 1251, 245]);
	}

	private function generateSeed():Int {
		return seed = Std.int((seed * MULTIPLIER) % MODULUS);
	}

	public static function calculateRandomInt(maxInt:Int) {
		return Server.server.map.randomInt(maxInt);
	}

	public function randomInt(x:Int = MAX_NUM):Int {
		return Math.floor(generateSeed() / MODULUS * (x + 1));
	}

	public static function calculateRandomFloat():Float {
		return Server.server.map.randomFloat();
	}

	public function randomFloat():Float {
		return generateSeed() / MODULUS;
	}

	private function createVectors(length:Int) {
		this.length = length;

		objects = new Vector<Array<Int>>(length);
		hiddenObjects = new Vector<Array<Int>>(length);
		objectHelpers = new Vector<ObjectHelper>(length);
		floors = new Vector<Int>(length);
		biomes = new Vector<Int>(length);

		// timeObjectHelpers = [];

		originalObjectsCount = new Map<Int, Int>();
		currentObjectsCount = new Map<Int, Int>();
	}

	public function isWater(x:Int, y:Int):Bool {
		var biome = getBiomeId(x, y);
		return Biome.IsWater(biome);
	}

	public function getBiomeSpeed(x:Int, y:Int):Float {
		var floor = WorldMap.world.getFloorId(x, y);
		var biome = biomes[index(x, y)];

		// trace('${ x },${ y }:BI ${ biomeType }');

		if(floor == 485 || floor ==  884 || floor ==  898)
		{
			if(biome == BiomeTag.OCEAN || biome == BiomeTag.PASSABLERIVER || biome == BiomeTag.RIVER) return 1;
		}

		return switch biome {
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

	public static function isBiomeBlocking(x:Int, y:Int):Bool {
		var floorId = WorldMap.world.getFloorId(x, y);
		var biome = WorldMap.world.getBiomeId(x, y);		
		
		// 485 Wooden Floor / 884 Stone Floor / 898 Ancient Stone Floor / 1596 Stone Road
		//if(floor == 485 || floor ==  884 || floor ==  898 ||  floor == 1596)
		// 3290 Pine Floor
		if(floorId > 0 && floorId != 3290)
		{
			if(biome == BiomeTag.SNOWINGREY || biome == BiomeTag.OCEAN || biome == BiomeTag.PASSABLERIVER || biome == BiomeTag.RIVER) return false;
		}

		var biomeSpeed = Server.server.map.getBiomeSpeed(x, y);

		return biomeSpeed < 0.1;
	}

	/**
		var targetBiome = getBiomeId(x,y);

		if(targetBiome == BiomeTag.SNOWINGREY) return true;
		if(targetBiome == BiomeTag.OCEAN) return true;
		return false;
	} **/
	public static function worldGetBiomeId(x:Int, y:Int):BiomeTag {
		return Server.server.map.getBiomeId(x, y);
	}

	public function getBiomeId(x:Int, y:Int):BiomeTag {
		return biomes[index(x, y)];
	}
	
	public function getOriginalBiomeId(x:Int, y:Int):Int {
		return originalBiomes[index(x, y)];
	}

	public function setBiomeId(x:Int, y:Int, biomeId:Int) {
		return biomes[index(x, y)] = biomeId;
	}

	public static function worldGetObjectId(x:Int, y:Int):Array<Int> {
		return Server.server.map.getObjectId(x, y);
	}

	public function getObjectId(x:Int, y:Int):Array<Int> {
		return objects[index(x, y)];
	}

	public function getHiddenObjectId(x:Int, y:Int):Array<Int> {
		return hiddenObjects[index(x, y)];
	}

	public function getOriginalObjectId(x:Int, y:Int):Array<Int> {
		return originalObjects[index(x, y)];
	}

	/** Does not set timeToChange for object. If you want to set use setObjectHelper instead **/
	public function setObjectId(x:Int, y:Int, ids:Array<Int>) {
		objects[index(x, y)] = ids;

		if (ids.length > 1) {
			// set object Helper, otherwiese stuff in containers will not be saved
			setObjectHelper(x, y, ObjectHelper.readObjectHelper(null, ids));
		} else {
			// TODO create time transition
			setObjectHelperNull(x, y);
		}
	}

	public function setHiddenObjectId(x:Int, y:Int, ids:Array<Int>) {
		hiddenObjects[index(x, y)] = ids; // TODO ObjectHelper also hidden???
	}

	public function getObjectDataAtPosition(x:Int, y:Int):ObjectData {
		var helper = objectHelpers[index(x, y)];

		if (helper != null) return helper.objectData;

		var objId = getObjectId(x, y);

		return ObjectData.getObjectData(objId[0]);
	}

	public static function worldGetObjectHelper(x:Int, y:Int, allowNull:Bool = false):ObjectHelper {
		return Server.server.map.getObjectHelper(x, y, allowNull);
	}

	public function getObjectHelper(tx:Int, ty:Int, allowNull:Bool = false):ObjectHelper {
		// trace('objectHelper: $x,$y');
		var position = index(tx, ty);
		var helper = objectHelpers[position];

		if (helper == null && allowNull) return helper;
		var helperPosition = helper == null ? 0 : index(helper.tx, helper.ty);

		if (helper != null && helperPosition != position) {
			trace('WARNING: Object ${helper.description} moved meanwhile! ${helper.tx} ${helper.ty} --> ${tx} ${ty} $helperPosition --> $position');
			//helper.tx = tx;
			var samePos = helper.tx == tx && helper.ty == ty;

			if(samePos == false){
				if (ServerSettings.debug && helper.id != 0) throw new Exception('WARNING: Object ${helper.name} moved meanwhile!');
				objectHelpers[index(tx, ty)] = null;
				helper = null;
				//throw new Exception('WARNING: Object ${helper.name} moved meanwhile!');
			}
		}

		if (helper != null) return helper;

		helper = ObjectHelper.readObjectHelper(null, getObjectId(tx, ty));
		helper.tx = tx;
		helper.ty = ty;

		if (helper.containedObjects.length > helper.objectData.numSlots) {
			var message = 'WARNING: world getObjectHelper: ${helper.name} ${helper.toArray()} slots: containedObjects.length > player.heldObject.objectData.numSlots: ${helper.objectData.numSlots}';
			trace(message);
			throw new Exception(message);
		}

		return helper;
	}

	public function setObjectHelperNull(x:Int, y:Int) {
		objectHelpers[index(x, y)] = null;
	}

	// sets objectHelper and also Object Ids on same Tile
	public function setObjectHelper(x:Int, y:Int, helper:ObjectHelper) {
		if (helper != null) helper.TransformToDummy();

		// trace('objectHelper: $x,$y');
		objectHelpers[index(x, y)] = helper;

		if (helper == null) {
			objects[index(x, y)] = [0];
			return;
		}

		var ids = helper.toArray();
		objects[index(x, y)] = ids;

		helper.tx = x;
		helper.ty = y;

		helper.timeToChange = ObjectHelper.CalculateTimeToChangeForObj(helper);

		if (helper.containedObjects.length > helper.objectData.numSlots) {
			var objData = helper.containedObjects[0];
			var message = 'WARNING: world setObjectHelper: ${helper.name} ${helper.toArray()} first: ${objData.name} slots: containedObjects.length > player.heldObject.objectData.numSlots: ${helper.objectData.numSlots}';
			trace(message);
			// helper.containedObjects = [];

			throw new Exception(message);
		}

		if (deleteObjectHelperIfUseless(helper)) return;
	}

	// to save space keep ObjectHelper only if used to store number of uses, or has time transition...
	// ... or has owner or is a container or has a groundObject (used if a animal walks on an object)
	// TODO dont delete stuff with owners like a gate
	public function deleteObjectHelperIfUseless(helper:ObjectHelper):Bool {
		var obj = getObjectId(helper.tx, helper.ty);

		if (obj[0] != helper.dummyId()) {
			WorldMap.world.mutex.acquire();

			try {
				// test again after receiving mutex
				var obj = getObjectId(helper.tx, helper.ty);

				if (obj[0] != helper.dummyId()) {
					// TODO look who sets the dummy wrong
					var objData = ObjectData.getObjectData(obj[0]);

					trace('WARNING: ${helper.tx},${helper.ty} object Id: ${obj[0]} ${objData.description} did not fit to object.dummyId: ${helper.dummyId()} helper.id: ${helper.id} ${helper.description} NumberUses: ${helper.numberOfUses}');

					objectHelpers[index(helper.tx, helper.ty)] = null;

					setObjectHelper(helper.tx, helper.ty, helper);
				}
			} catch (ex)
				trace(ex);

			WorldMap.world.mutex.release();
		}

		if (helper.isHelperToBeDeleted()) {
			WorldMap.world.mutex.acquire();

			try {
				helper = getObjectHelper(helper.tx, helper.ty);

				if (helper.isHelperToBeDeleted()) {
					// test again after receiving mutex
					// if(x != helper.tx || y != helper.ty) trace('REMOVE ObjectHelper $x,$y h${helper.tx},h${helper.ty} USES < 1 && timeToChange == 0 && containedObjects.length == 0 && groundObject == null');
					objectHelpers[index(helper.tx, helper.ty)] = null;
				}
			} catch (ex)
				trace(ex);

			WorldMap.world.mutex.release();

			return true;
		}

		return false;
	}

	public function getFloorId(x:Int, y:Int):Int {
		return floors[index(x, y)];
	}

	public function setFloorId(x:Int, y:Int, floor:Int) {
		floors[index(x, y)] = floor;
	}

	public function index(x:Int, y:Int):Int {
		// Dont know why yet, but y seems to be right if -1
		y -= 1;
		// make map round x wise
		x = x % this.width;
		if (x < 0) x += this.width;
		// else if(x >= this.width) x -= this.width;

		// make map round y wise
		y = y % this.height;
		if (y < 0) y += this.height;
		// else if(y >= this.height) y -= this.height;

		var i = x + y * width;

		return i;
	}

	// The Server and Client map is saved in an array with y starting from bottom,
	// The Map is saved with y starting from top. Therefore the map is y inversed during generation from picture
	public function generate() {
		this.mutex.acquire();

		Macro.exception(generateHelper());

		this.mutex.release();
	}

	public function generateHelper() {
		var pngDir = './${ServerSettings.MapFileName}'; // "./map.png";
		var pngmap = readPixels(pngDir);

		width = pngmap.width;
		height = pngmap.height;
		length = width * height;

		trace('map width: ' + width);
		trace('map height: ' + height);

		createVectors(length);

		for (y in 0...height) {
			for (x in 0...width) {
				if (y % 100 == 0 && x == 0) {
					trace('generating map up to y: ' + y);
				}

				// var p = pngmap.data.getInt32(4*(x+xOffset+(y+yOffset)*pngmap.width));
				var p = pngmap.data.getInt32(4 * (x + ((height - 1) - y) * pngmap.width));

				// ARGB, each 0-255
				// var a:Int = p>>>24;
				// var r:Int = (p>>>16)&0xff;
				// var g:Int = (p>>>8)&0xff;
				// var b:Int = (p)&0xff;
				// Or, AARRGGBB in hex:
				var hex:String = StringTools.hex(p, 8);
				var biomeInt;

				switch hex {
					case CYELLOW:
						biomeInt = YELLOW;
					case CGREY:
						biomeInt = GREY;
					case CSNOW:
						biomeInt = SNOW;
					case CDESERT:
						biomeInt = DESERT;
					case CSAND:
						biomeInt = DESERT;
					case CJUNGLE:
						biomeInt = JUNGLE;
					case CBORDERJUNGLE:
						biomeInt = BORDERJUNGLE;
					case CSWAMP:
						biomeInt = SWAMP;
					case CSNOWINGREY:
						biomeInt = SNOWINGREY;
					case COCEAN:
						biomeInt = OCEAN;
					case CRIVER:
						biomeInt = RIVER;
					case CPASSABLERIVER:
						biomeInt = PASSABLERIVER;
					default:
						biomeInt = GREEN;
				}
				if (biomeInt == YELLOW) {
					// trace('${ x },${ y }:BI ${ biomeInt },${ r },${ g },${ b } - ${ StringTools.hex(p,8) }');
				}

				// biomeInt = x % 30;

				biomes[x + y * width] = biomeInt;
			}
		}

		addExtraBiomes();

		generateObjects();

		generateExtraStuff();

		this.originalBiomes = biomes.copy();

		this.originalObjects = objects.copy();

		if (ServerSettings.AllowDebugObjectCreation) generateExtraDebugStuff(ServerSettings.startingGx, ServerSettings.startingGy);
	}

	public function writeBackup() {
		var tmpBackupDataNumber = (backupDataNumber % ServerSettings.MaxNumberOfBackups) + 1;

		var dir = './${ServerSettings.SaveDirectory}/$tmpBackupDataNumber/';

		writeToDisk(false, dir);

		trace('Wrote backup: backupDataNumber: $tmpBackupDataNumber');
		backupDataNumber++;
	}

	public function writeToDisk(saveOriginals:Bool = true, dir:String = null) {
		this.mutex.acquire();

		Macro.exception(writeToDiskHelper(saveOriginals, dir));

		this.mutex.release();
	}

	public function writeToDiskHelper(saveOriginals:Bool = true, dir:String = null) {
		var time = Sys.time();
		var sleepTime = 0.1;

		fixObjectIds('writing');

		if (dir == null) dir = './${ServerSettings.SaveDirectory}/';

		if (FileSystem.exists(dir) == false) FileSystem.createDirectory(dir);

		var tmpDataNumber = (saveDataNumber % 10) + 1;

		if (saveOriginals) writeMapBiomes(dir + ServerSettings.OriginalBiomesFileName + ".bin", originalBiomes);

		if (saveOriginals) writeMapObjects(dir + ServerSettings.OriginalObjectsFileName + ".bin", originalObjects);

		writeMapBiomes(dir + ServerSettings.CurrentBiomesFileName + tmpDataNumber + ".bin", biomes);

		this.mutex.release();
		Sys.sleep(sleepTime);
		this.mutex.acquire();

		writeMapFloors(dir + ServerSettings.CurrentFloorsFileName + tmpDataNumber + ".bin", floors);

		this.mutex.release();
		Sys.sleep(sleepTime);
		this.mutex.acquire();

		writeMapObjects(dir + ServerSettings.CurrentObjectsFileName + "Hidden" + tmpDataNumber + ".bin", hiddenObjects);

		this.mutex.release();
		Sys.sleep(sleepTime);
		this.mutex.acquire();

		writeMapObjects(dir + ServerSettings.CurrentObjectsFileName + tmpDataNumber + ".bin", objects);

		ObjectHelper.WriteMapObjHelpers(dir + ServerSettings.CurrentObjHelpersFileName + tmpDataNumber + ".bin", objectHelpers);

		PlayerAccount.WritePlayerAccounts(dir + "PlayerAccounts" + tmpDataNumber + ".bin");

		this.mutex.release();
		Sys.sleep(sleepTime);
		this.mutex.acquire();

		//Lineage.WriteAllLineages(dir + "Lineages" + tmpDataNumber + ".bin");
		Lineage.WriteNewLineages(dir + "Lineages" + tmpDataNumber + ".bin");
		if (ServerSettings.SavePlayers) GlobalPlayerInstance.WriteAllPlayers(dir + "Players" + tmpDataNumber + ".bin");

		writeIndexFile(dir + "lastDataNumber" + tmpDataNumber + ".txt", tmpDataNumber);
		writeIndexFile(dir + "lastDataNumber.txt", tmpDataNumber);

		writeFoodStatistics(dir + "FoodStats" + tmpDataNumber + ".txt");

		saveDataNumber++;

		var time = Math.round((Sys.time() - time) * 1000);

		if (ServerSettings.DebugWrite)
			trace('Write to disk: saveDataNumber: $tmpDataNumber Time: $time backupDataNumber: $backupDataNumber tick: ${TimeHelper.tick}');

		if (ServerSettings.TraceCountObjectsToDisk) {
			//trace('count objects time: ${Sys.time() - time}');
			var path = dir + 'ObjectCounts${tmpDataNumber}.txt';
			var writer = File.write(path, false);

			for (key in currentObjectsCount.keys()) {
				var objData = ObjectData.getObjectData(key);
				writer.writeString('Count object: [${key}] ${objData.description}: ${currentObjectsCount[key]} original: ${originalObjectsCount[key]}\n');
			}
			
			writer.close();
		}
	}

	private function writeFoodStatistics(path:String) {
		var writer = File.write(path, false);
		var total = 0.0;

		for(foodId in eatenFoodValues.keys()){
			total += eatenFoodValues[foodId];
		}

		for(foodId in eatenFoodValues.keys()){
			var foodData = ObjectData.getObjectData(foodId);
			var foodValue = Math.round(eatenFoodValues[foodId] * 1) / 1;
			var foodValueYum = Math.round(eatenFoodsYum[foodId] * 1) / 1;
			var foodValueMeh = Math.round(eatenFoodsMeh[foodId] * 1) / 1;
			var foodValueYumBoni = Math.round(eatenFoodsYumBoni[foodId] * 1) / 1;
			var foodValueMehMali = Math.round(eatenFoodsMehMali[foodId] * 1) / 1;
			var percent = Math.round(eatenFoodValues[foodId] / total * 100) / 1;

			writer.writeString('${percent}% pipes: ${foodValue} ${foodData.name}[${foodData.id}] yum: ${foodValueYum} meh: ${foodValueMeh} boni: ${foodValueYumBoni} mali: ${foodValueMehMali}\n');
		}
		writer.close();
	}

	private function writeIndexFile(path:String, tmpDataNumber:Int) {
		var writer = File.write(path, false);
		writer.writeString('$tmpDataNumber\n');
		writer.writeString('$backupDataNumber\n');
		writer.writeString('${TimeHelper.tick}\n');
		writer.writeString('${Server.server.playerIndex}\n');
		writer.writeString('${PlayerAccount.AccountIdIndex}\n');
		writer.writeString('${ObjectHelper.dataVersionNumberForWrite}\n');
		writer.close();
	}

	private function readIndexFileAndInitVariables(path:String) {
		var reader = File.read(path, false);
		this.saveDataNumber = Std.parseInt(reader.readLine());
		this.backupDataNumber = Std.parseInt(reader.readLine());
		TimeHelper.tick = Std.parseFloat(reader.readLine());
		TimeHelper.lastTick = TimeHelper.tick;
		AiBase.tick = TimeHelper.tick;
		AiBase.lastTick = TimeHelper.lastTick;
		Server.server.playerIndex = Std.parseInt(reader.readLine());
		PlayerAccount.AccountIdIndex = Std.parseInt(reader.readLine());
		
		try{ObjectHelper.dataVersionNumberForRead = Std.parseInt(reader.readLine());}
		catch(ex){
			ObjectHelper.dataVersionNumberForRead = 4;
			trace('WARNING: Could not read ObjectHelper.dataVersionNumberForRead');
		}
		// trace('PlayerAccount.AccountIdIndex: ${PlayerAccount.AccountIdIndex}');

		reader.close();
	}

	private function fixObjectIds(desc:String) {

		for(i in 0...this.objects.length){
			var obj = objects[i];
			var objData = ObjectData.getObjectData(obj[0]);

			if(objData == null){
				trace('WARNING no object data: ${obj[0]}');
				objects[i] = [0];
				continue;
			} 
		}
		
		for (helper in objectHelpers) {
			if (helper == null) continue;
			var obj = getObjectId(helper.tx, helper.ty);

			if (obj[0] != helper.dummyId()) {
				// TODO look who sets the dummy wrong
				var objData = ObjectData.getObjectData(obj[0]);

				trace('WARNING $desc: ${helper.tx},${helper.ty} object Id: ${obj[0]} ${objData.description} did not fit to object.dummyId: ${helper.dummyId()} helper.id: ${helper.id} ${helper.description} NumberUses: ${helper.numberOfUses}');

				this.setObjectHelper(helper.tx, helper.ty, helper);
			}

			/*obj = getObjectId(helper.tx, helper.ty);

				if(obj[0] != helper.dummyId())
				{
					// TODO look who sets the dummy wrong
					var objData = ObjectData.getObjectData(obj[0]);

					trace('WARNING 2 $desc: ${helper.tx},${helper.ty} object Id: ${obj[0]} ${objData.description} did not fit to object.dummyId: ${helper.dummyId()} helper.id: ${helper.id} ${helper.description} NumberUses: ${helper.numberOfUses}');

					this.setObjectHelper(helper.tx, helper.ty, helper);
			}*/
		}
	}

	public function readFromDisk():Bool {
		this.mutex.acquire();
		var done = false;

		Macro.exception(done = readFromDiskHelper());

		this.mutex.release();

		return done;
	}

	private function readFromDiskHelper():Bool {
		var dir = './${ServerSettings.SaveDirectory}/';

		if (sys.FileSystem.exists(dir) == false || sys.FileSystem.isDirectory(dir) == false) {
			trace('Save $dir could not be found!');
			return false;
		}

		if (sys.FileSystem.exists(dir + "lastDataNumber.txt") == false) {
			trace('Init file: ${dir + "lastDataNumber.txt"} could not be found!');
			return false;
		}

		readIndexFileAndInitVariables(dir + "lastDataNumber.txt");

		trace('saveDataNumber: $saveDataNumber backupDataNumber: $backupDataNumber tick: ${TimeHelper.tick}');

		this.originalBiomes = readMapBiomes(dir + ServerSettings.OriginalBiomesFileName + ".bin");

		this.originalObjects = readMapObjects(dir + ServerSettings.OriginalObjectsFileName + ".bin");

		this.biomes = readMapBiomes(dir + ServerSettings.CurrentBiomesFileName + saveDataNumber + ".bin");

		this.floors = readMapFloors(dir + ServerSettings.CurrentFloorsFileName + saveDataNumber + ".bin");

		this.objects = readMapObjects(dir + ServerSettings.CurrentObjectsFileName + saveDataNumber + ".bin");

		this.hiddenObjects = readMapObjects(dir + ServerSettings.CurrentObjectsFileName + "Hidden" + saveDataNumber + ".bin");

		ObjectHelper.ReadMapObjHelpers(dir + ServerSettings.CurrentObjHelpersFileName + saveDataNumber + ".bin");

		Macro.exception(PlayerAccount.ReadPlayerAccounts(dir + "PlayerAccounts" + saveDataNumber + ".bin"));

		//Lineage.ReadAndSetLineages(dir + "Lineages" + saveDataNumber + ".bin");

		Lineage.ReadAndSaveAllLineages(dir + "LineagesAll.bin", dir + "Lineages" + saveDataNumber + ".bin");

		if (ServerSettings.LoadPlayers) GlobalPlayerInstance.ReadPlayers(dir + "Players" + saveDataNumber + ".bin");

		fixObjectIds('read');

		this.originalObjectsCount = countObjects(this.originalObjects);

		this.currentObjectsCount = countObjects(this.objects);

		ObjectHelper.InitObjectHelpersAfterRead();

		Lineage.WriteLineageStatistics();

		return true;
	}

	public function writeMapBiomes(path:String, biomesToWrite:Vector<Int>) {
		// trace('Wrtie to file: $path width: $width height: $height length: $length');

		if (width * height != length) throw new Exception('width * height != length');
		if (biomesToWrite.length != length) throw new Exception('biomesToWrite.length != length');

		var writer = File.write(path, true);
		var dataVersion = 1;

		writer.writeInt32(dataVersion);
		writer.writeInt32(width);
		writer.writeInt32(height);

		for (biome in biomesToWrite) {
			writer.writeInt8(biome);
		}

		writer.close();
	}

	public function readMapBiomes(path:String):Vector<Int> {
		var reader = File.read(path, true);
		var dataVersion = reader.readInt32();
		this.width = reader.readInt32();
		this.height = reader.readInt32();
		this.length = width * height;
		var newBiomes = new Vector<Int>(length);

		trace('Read from file: $path width: $width height: $height length: $length');

		if (width * height != length) throw new Exception('width * height != length');

		for (i in 0...newBiomes.length) {
			newBiomes[i] = reader.readInt8();
		}

		reader.close();

		return newBiomes;
	}

	public function writeMapFloors(path:String, floorsToWrite:Vector<Int>) {
		// trace('Wrtie to file: $path width: $width height: $height length: $length');

		if (width * height != length) throw new Exception('width * height != length');
		if (floorsToWrite.length != length) throw new Exception('floorsToWrite.length != length');

		var writer = File.write(path, true);
		var dataVersion = 1;

		writer.writeInt32(dataVersion);
		writer.writeInt32(width);
		writer.writeInt32(height);

		for (floor in floorsToWrite) {
			writer.writeInt32(floor);
		}

		writer.close();
	}

	public function readMapFloors(path:String):Vector<Int> {
		var reader = File.read(path, true);
		var dataVersion = reader.readInt32();
		var width = reader.readInt32();
		var height = reader.readInt32();
		var length = width * height;
		var newFloors = new Vector<Int>(length);

		trace('Read from file: $path width: $width height: $height length: $length');

		if (width != this.width) throw new Exception('width != this.width');
		if (height != this.height) throw new Exception('height != this.height');
		if (length != this.length) throw new Exception('length != this.length');

		for (i in 0...newFloors.length) {
			newFloors[i] = reader.readInt32();
		}

		reader.close();

		return newFloors;
	}

	public function writeMapObjects(path:String, objectsToWrite:Vector<Array<Int>>) {
		// trace('Wrtie to file: $path width: $width height: $height length: $length');
		if (objectsToWrite.length != length) throw new Exception('objectsToWrite.length != length');

		var writer = File.write(path, true);
		var dataVersion = 1;

		writer.writeInt32(dataVersion);
		writer.writeInt32(width);
		writer.writeInt32(height);

		var count = 0;

		for (obj in objectsToWrite) {
			if (obj == null) {
				obj = [0];
				objectsToWrite[count] = obj;
			}
			writer.writeInt32(obj[0]);

			count++;
		}

		writer.close();
	}

	public function readMapObjects(path:String):Vector<Array<Int>> {
		var reader = File.read(path, true);
		var dataVersion = reader.readInt32();
		var width = reader.readInt32();
		var height = reader.readInt32();
		var length = width * height;
		var newObjects = new Vector<Array<Int>>(length);

		trace('Read from file: $path width: $width height: $height length: $length');

		if (width != this.width) throw new Exception('width != this.width');
		if (height != this.height) throw new Exception('height != this.height');
		if (length != this.length) throw new Exception('length != this.length');

		for (i in 0...newObjects.length) {
			newObjects[i] = [reader.readInt32()];
		}

		reader.close();

		return newObjects;
	}

	public function countObjects(objectsToCount:Vector<Array<Int>>, objHelpersToCount:Vector<ObjectHelper> = null):Map<Int, Int> {
		var objList = new Map<Int, Int>();

		for (obj in objectsToCount) {
			if (obj[0] == 0) continue;

			var objData = ObjectData.getObjectData(obj[0]);

			if (objData.countsOrGrowsAs != 0) {
				objData = ObjectData.getObjectData(objData.countsOrGrowsAs);
			}

			objList[objData.parentId]++;
		}

		if (objHelpersToCount == null) return objList;

		for (obj in objHelpersToCount) {
			if (obj == null) continue;

			for (containedObj in obj.containedObjects) {
				objList[containedObj.parentId]++;

				for (subContainedObj in containedObj.containedObjects) {
					objList[subContainedObj.parentId]++;
				}
			}
		}

		return objList;
	}

	function addExtraBiomes() {
		var dist = ServerSettings.CreateGreenBiomeDistance;
		var tmpIsPlaced = new Vector<Bool>(length);

		for (y in 0...height) {
			for (x in 0...width) {
				if (tmpIsPlaced[index(x, y)]) continue;

				var biome = getBiomeId(x, y);

				if (biome == BiomeTag.RIVER || biome == BiomeTag.PASSABLERIVER || biome == BiomeTag.JUNGLE || biome == BiomeTag.SWAMP) {
					for (ix in -dist...dist + 1) {
						for (iy in -dist...dist + 1) {
							var tmpX = x + ix;
							var tmpY = y + iy;

							if (tmpIsPlaced[index(tmpX, tmpY)]) continue;
							var nextBiome = getBiomeId(tmpX, tmpY);

							if ((biome == BiomeTag.RIVER || biome == BiomeTag.PASSABLERIVER) && ix * ix < 2 && iy * iy < 2) {
								if (nextBiome == BiomeTag.GREEN || nextBiome == BiomeTag.YELLOW || nextBiome == BiomeTag.DESERT || nextBiome == BiomeTag.RIVER) {
									// trace('$ix,$iy biome: $biome nextBiome: $nextBiome ');
									if (biome == BiomeTag.PASSABLERIVER || nextBiome != BiomeTag.RIVER) {
										// trace('SET!!! $ix,$iy biome: $biome nextBiome: $nextBiome ');
										setBiomeId(tmpX, tmpY, BiomeTag.PASSABLERIVER);
										tmpIsPlaced[index(tmpX, tmpY)] = true;
									}
								}
							} else {
								if (nextBiome == BiomeTag.YELLOW || nextBiome == BiomeTag.DESERT) {
									setBiomeId(tmpX, tmpY, BiomeTag.GREEN);
									// tmpIsPlaced[index(tmpX, tmpY)] = true;
								}
							}
						}
					}
				}
			}
		}
	}

	function generateObjects() {
		var generatedObjects = 0;
		originalObjectsCount = new Map<Int, Int>();
		currentObjectsCount = new Map<Int, Int>();

		for (y in 0...height) {
			for (x in 0...width) {
				var biomeInt = biomes[x + y * width];

				objects[x + y * width] = [0];
				// if(x+y*width < 10000) objects[x+y*width] = [4746];

				// if(x < 200 || x > 600) continue;

				// if there is a object below allready continue
				if (y > 0 && objects[x + (y - 1) * width][0] != 0) continue;
				if (randomFloat() > 0.4) continue;
				if (getBiomeId(x, y) == BiomeTag.GREEN && randomFloat() < 0.3) {

					var isNearLand = getBiomeId(x + 1, y) == BiomeTag.PASSABLERIVER;
					isNearLand = isNearLand || getBiomeId(x - 1, y) == BiomeTag.PASSABLERIVER;
					isNearLand = isNearLand || getBiomeId(x, y + 1) == BiomeTag.PASSABLERIVER;
					isNearLand = isNearLand || getBiomeId(x, y - 1) == BiomeTag.PASSABLERIVER;

					if (isNearLand){
						setObjectId(x, y, [141]); // Canada Goose Pond
						continue;
					}
				}

				var set:Bool = false;

				var biomeData = ObjectData.biomeObjectData[biomeInt];

				if (biomeData == null) continue;

				var random = randomFloat() * ObjectData.biomeTotalChance[biomeInt];
				var sumChance = 0.0;

				for (obj in biomeData) {
					if (set) continue;
					var chance = obj.mapChance;
					sumChance += chance;

					if (random <= sumChance) {
						objects[x + y * width] = [obj.id];

						originalObjectsCount[obj.id] += 1;
						currentObjectsCount[obj.id] += 1;

						// trace('generate: bi: $biomeInt id: ${obj.id} rand: $random sc: $sumChance');
						set = true;
						generatedObjects++;
					}
				}
			}
		}

		trace('generatedObjects: $generatedObjects');

		if (ServerSettings.TraceCountObjects) {
			for (key in originalObjectsCount.keys()) {
				var objData = ObjectData.getObjectData(key);
				trace('Generated obj[${key}] ${objData.description}: ${originalObjectsCount[key]}');
			}
		}
	}

	function generateExtraStuff() {
		var tmpIsPlaced = new Vector<Bool>(length);

		for (y in 0...height) {
			for (x in 0...width) {
				var obj = objects[x + y * width];

				// 942 Muddy Iron Vein --> // 3961 Iron Vein
				// TODO better patch the data
				//if (obj[0] == 942) objects[x + y * width] = [3961]; 

				/*if (obj[0] == 942 || obj[0] == 3030) // 942 Muddy iron vein // 3030 Natural Spring
				{
					// generate also some random stones
					var random = randomInt(2) + 1;

					for (i in 0...100) {
						var dist = 3;
						var tx = x + randomInt(dist * 2) - dist;
						var ty = y + randomInt(dist * 2) - dist;

						if (((tx - x) * (tx - x)) + ((ty - y) * (ty - y)) > dist * dist) continue;

						var biome = getBiomeId(tx, ty);

						if (biome != BiomeTag.GREY && biome != BiomeTag.YELLOW && biome != BiomeTag.GREEN) continue;

						if (getObjectId(tx, ty)[0] != 0) continue;
						if (getObjectId(tx, ty - 1)[0] != 0) continue;
						if (getObjectId(tx, ty + 1)[0] != 0) continue;
						if (getObjectId(tx - 1, ty)[0] != 0) continue;

						setObjectId(tx, ty, [503]); // Dug Big Rock

						random -= 1;
						if (random <= 0) break;
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
					}
				}*/

				var tmpObj = getObjectId(x, y);

				if (tmpObj[0] == 0) continue;

				if (tmpIsPlaced[index(x, y)]) continue;

				// if obj is no iron, no tary spot and no spring there is a chance for winning lottery
				if (ServerSettings.CanObjectBeLuckySpot(tmpObj[0]) == false) continue;

				if (getBiomeId(x, y) == BiomeTag.PASSABLERIVER) continue;

				if (randomFloat() < ServerSettings.ChanceForLuckySpot) {
					// var objData = ObjectData.getObjectData(tmpObj[0]);
					var timeTransition = TransitionImporter.GetTransition(-1, tmpObj[0], false, false);
					var random = 2 + randomInt(timeTransition != null ? 1 : 5);

					var tmpRandom = random;

					for (i in 0...100) {
						var dist = 9;
						var tx = x + randomInt(dist * 2) - dist;
						var ty = y + randomInt(dist * 2) - dist;

						if (((tx - x) * (tx - x)) + ((ty - y) * (ty - y)) > dist * dist) continue;

						var biomeId = getBiomeId(x, y);

						if (biomeId != getBiomeId(tx, ty)) continue;

						if (getObjectId(tx, ty)[0] != 0) continue;
						if (getObjectId(tx, ty - 1)[0] != 0) continue;
						if (getObjectId(tx, ty + 1)[0] != 0) continue;
						if (getObjectId(tx - 1, ty)[0] != 0) continue;

						tmpIsPlaced[index(tx, ty)] = true;
						setObjectId(tx, ty, tmpObj);

						random -= 1;
						if (random <= 0) break;
					}

					// trace('lucky: ${objData.description} placed: ${tmpRandom-random} from $tmpRandom');
				}
			}
		}
	}

	function readPixels(file:String):{data:Bytes, width:Int, height:Int} {
		var handle = sys.io.File.read(file, true);
		var d = new Reader(handle).read();
		var hdr = format.png.Tools.getHeader(d);
		var ret = {
			data: format.png.Tools.extract32(d),
			width: hdr.width,
			height: hdr.height
		};
		handle.close();
		return ret;
	}

	public function getChunk(x:Int, y:Int, width:Int, height:Int):WorldMap {
		var map = new WorldMap();
		var length = width * height;
		map.createVectors(length);
		for (px in 0...width) {
			for (py in 0...height) {
				var localIndex = px + py * width;
				var index = index(x + px, y + py);

				map.biomes[localIndex] = biomes[index];
				map.floors[localIndex] = floors[index];
				map.objects[localIndex] = objects[index];
			}
		}
		return map;
	}

	private inline function sigmoid(input:Float, knee:Float):Float {
		var shifted = input * 2 - 1;
		var sign = input < 0 ? -1 : 1;
		var k = -1 - knee;
		var abs = Math.abs(shifted);
		var out = sign * abs * k / (1 + k - abs);
		return (out + 1) * 0.5;
	}

	public function toString():String {
		var string = "";

		for (i in 0...length) {
			var obj = MapData.stringID(objects[i]);
			string += ' ${biomes[i]}:${floors[i]}:$obj';
		}
		return string.substr(1);
	}

	public static function PlaceObjectById(tx:Int, ty:Int, objId:Int):Bool {
		if(objId == 0) return true;
		var obj = new ObjectHelper(null, objId);
		return PlaceObject(tx, ty, obj);
	}

	public static function TransformObject(obj:ObjectHelper){
		var objId = obj.parentId;
		// Horse-Drawn Cart 778 // Horse-Drawn Tire Cart 3158
		if(objId != 778 && objId != 3158) return false;

		// transform placed object back to a not held one in case its a held one like a horse cart
		var trans = TransitionImporter.GetTransition(objId, -1);
		if(trans == null) return false;

		trace('PlaceObject transform held object: ${trans.getDescription()}');
		obj.id =  trans.newTargetID;
		
		return true;
	}

	public static function PlaceObject(tx:Int, ty:Int, objectToPlace:ObjectHelper, allowReplaceObject:Bool = false):Bool {
		// should not be on ground Horse-Drawn Cart 778 // Horse-Drawn Tire Cart 3158
		TransformObject(objectToPlace);

		var originalObjectToPlace = objectToPlace;

		objectToPlace = TryPlaceObject(tx, ty, objectToPlace, allowReplaceObject);

		if (objectToPlace == null) return true;

		var distance = 1;

		for (i in 1...10000) {
			if (originalObjectToPlace != objectToPlace) allowReplaceObject = false;

			distance = Math.ceil(i / (20 * distance * distance));
			// trace('place $i distance: $distance');

			var tmpX = tx + world.randomInt(distance * 2) - distance;
			var tmpY = ty + world.randomInt(distance * 2) - distance;

			objectToPlace = TryPlaceObject(tmpX, tmpY, objectToPlace, allowReplaceObject);

			if (objectToPlace == null) return true;
		}

		return false;
	}

	private static function TryPlaceObject(x:Int, y:Int, objectToPlace:ObjectHelper, allowReplaceObject:Bool):ObjectHelper {
		if (WorldMap.isBiomeBlocking(x, y)) return objectToPlace;

		var world = Server.server.map;
		var objId = world.getObjectId(x, y);
		var objDataBelow = world.getObjectDataAtPosition(x, y - 1);

		if(objDataBelow.isTree()) return objectToPlace; // dont place behind a tree

		if (objId[0] == 0) {
			world.setObjectHelper(x, y, objectToPlace);

			Connection.SendMapUpdateToAllClosePlayers(x, y);

			// trace('TryPlaceObject Done ${objectToPlace.description}');

			return null;
		}

		var obj = world.getObjectHelper(x, y);

		/*if(obj.canBePlacedIn(objectToPlace)){
		
			objectToPlace.containedObjects.push(obj);

			world.setObjectHelper(x, y, objectToPlace);

			Connection.SendMapUpdateToAllClosePlayers(x, y);

			trace('TryPlaceObject Done in container ${objectToPlace.description}');

			return null;
		}
		*/

		if (allowReplaceObject && obj.isPermanent() == false) {
			world.setObjectHelper(x, y, objectToPlace);

			Connection.SendMapUpdateToAllClosePlayers(x, y);

			return obj;
		}

		return objectToPlace;
	}

	public static function WriteInt32Array(writer:FileOutput, array:Array<Int32>) {
		writer.writeInt8(array.length);

		for (i in array) {
			writer.writeInt32(i);
		}
	}

	// TODO int64

	/**readBytes and use setInt64 and getInt64 in Bytes
		It should be included into io unfortunately not yet**/
	public static function ReadInt32Array(reader:FileInput):Array<Int32> {
		var arrayLength = reader.readInt8();
		if (arrayLength == 100) return null; // reached the end
		if (arrayLength > 100) throw new Exception('array length is: $arrayLength > 100');

		var newArray = new Array<Int>();

		for (i in 0...arrayLength) {
			newArray.push(reader.readInt32());
		}

		return newArray;
	}

	public function updateObjectCounts() {
		var time = Sys.time();

		this.currentObjectsCount = countObjects(objects, objectHelpers);

		if (ServerSettings.TraceCountObjects) {
			trace('count objects time: ${Sys.time() - time}');

			for (key in currentObjectsCount.keys()) {
				var objData = ObjectData.getObjectData(key);
				trace('Count object: [${key}] ${objData.description}: ${currentObjectsCount[key]} original: ${originalObjectsCount[key]}');
			}
		}
	}

	public function transformX(p:PlayerInterface, tx:Int) {
		var x = tx - p.gx; // make relative to player
		if(x - p.x > this.width / 2) x -= this.width; // consider that world is round
		else if(x - p.x < -this.width / 2) x += this.width; // consider that world is round

		// TODO consider if walked more then one time round the world
		return x;
	}

	public function transformY(p:PlayerInterface, ty:Int) {
		var y = ty - p.gy; // make relative to player
		if(y - p.y > this.height / 2) y -= this.height; // consider that world is round
		else if(y - p.y < -this.height / 2) y += this.height; // consider that world is round

		// TODO consider if walked more then one time round the world
		return y;
	}

	public function transformFloatX(p:PlayerInterface, tx:Float) {
		var x = tx - p.gx; // make relative to player
		if(x - p.x > this.width / 2) x -= this.width; // consider that world is round
		else if(x - p.x < -this.width / 2) x += this.width; // consider that world is round

		// TODO consider if walked more then one time round the world
		return x;
	}

	public function transformFloatY(p:PlayerInterface, ty:Float) {
		var y = ty - p.gy; // make relative to player
		if(y - p.y > this.height / 2) y -= this.height; // consider that world is round
		else if(y - p.y < -this.height / 2) y += this.height; // consider that world is round

		// TODO consider if walked more then one time round the world
		return y;
	}

	public function addFoodStatistic(foodData:ObjectData, foodValue:Float){
		var foodId = foodData.parentId;
		var yum = foodValue - foodData.foodValue;
		//var meh = food.objectData.foodValue - foodValue;

		//trace('addFoodStatistic: ${foodData.name} foodValue: ${Math.round(foodValue*10)/10} all total: ${Math.round(this.eatenFoodValues[foodId]*10)/10} yum: ${Math.round(yum*10)/10}');

		this.eatenFoodValues[foodId] += foodValue;
		if(yum > 0) this.eatenFoodsYum[foodId] += foodValue; 
		else this.eatenFoodsMeh[foodId] += foodValue; 

		if(yum > 0) this.eatenFoodsYumBoni[foodId] += yum; 
		else this.eatenFoodsMehMali[foodId] -= yum; 
	}
}
#end
