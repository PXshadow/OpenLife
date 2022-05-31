package openlife.data.object;

import haxe.Exception;
import haxe.Serializer;
import haxe.Unserializer;
import haxe.ds.Vector;
import haxe.io.BytesData;
import haxe.io.BytesOutput;
import haxe.io.Input;
import haxe.macro.Expr.Catch;
import openlife.data.sound.SoundData;
import openlife.data.transition.TransitionData;
import openlife.data.transition.TransitionImporter;
import openlife.engine.Engine;
import openlife.engine.Utility;
import openlife.resources.ObjectBake;
import openlife.resources.Resource;
import openlife.server.Biome.BiomeTag;
import openlife.server.Lineage.PrestigeClass;
import openlife.server.Server;
import openlife.server.WorldMap;
import openlife.settings.ServerSettings;
import sys.FileSystem;
import sys.io.File;
import sys.io.FileInput;
import sys.io.FileOutput;

@:enum abstract PersonColor(Int) from Int to Int {
	public var Black = 1;
	public var Brown = 3;
	public var White = 4;
	public var Ginger = 6;

	public static function getPersonColorByBiome(biome:Int):Int {
		if (biome == BiomeTag.DESERT) return PersonColor.Black;
		if (biome == BiomeTag.JUNGLE) return PersonColor.Brown;
		if (biome == BiomeTag.GREY) return PersonColor.White;
		if (biome == BiomeTag.SNOW) return PersonColor.Ginger;

		return -1;
	}
}

@:expose
@:rtti
class ObjectData extends LineReader {
	public static var dataVersionNumber:Int;
	public static var importedObjectData:Vector<ObjectData>;

	// stores all ObjectData including dummy objects for objects with numUse > 2
	private static var objectDataMap:Map<Int, ObjectData> = [];

	// used for creation of the inital objects on the worldmap
	public static var biomeTotalChance:Map<Int, Float>;
	public static var biomeObjectData:Map<Int, Array<ObjectData>>;

	// lists all objects that can be eaten
	public static var foodObjects:Array<ObjectData> = [];

	// to store the different persons
	// person ==> White = 4 / Brown = 3 / Ginger = 6 / Black = 1
	public static var personObjectData:Array<ObjectData> = [];
	public static var maleByRaceObjectData:Map<Int, Array<ObjectData>> = [];
	public static var femaleByRaceObjectData:Map<Int, Array<ObjectData>> = [];

	/**
	 * Toolset record set
	 */
	public static var toolsetRecord:Array<ToolSetRecord> = [];

	//**searchable Name in upper letters**/
	public var name:String; // not saved / generated from description
	public var isOwned:Bool = false; // not saved / generated from description

	// Indicates that this object can be used to creat foodFromTarget
	public var foodFromTarget:ObjectData; // do not save on disk since it can be calculated after loading
	// Indicates that this object can be used to creat foodFromTarget with a tool
	public var foodFromTargetWithTool:ObjectData; // do not save on disk since it can be calculated after loading
	// Indicates that this object can be used to creat foodFromActor
	public var foodFromActor:ObjectData; // do not save on disk since it can be calculated after loading

	public var dummyObjects:Array<ObjectData> = [];
	public var lastUseObject:Int = 0; // TODO set for all according to transition // like Berry numUses: 1 ==> 0
	public var undoLastUseObject:Int = 0; // TODO set for all according to transition // like Berry numUses: 0 ==> 1

	/**is used for different wound decay on player**/
	public var alternativeTimeOutcome:Int = -1;

	public var secondTimeOutcome:Int = -1;
	public var secondTimeOutcomeTimeToChange:Int = -1;

	public var hungryWork:Float = 0;
	public var alternativeTransitionOutcome:Array<Int> = new Array<Int>();

	// not saved
	public var decayFactor:Float = 1;
	public var winterDecayFactor:Float = 0;
	public var springRegrowFactor:Float = 0;
	public var countsOrGrowsAs:Int = 0; // the object should be counted as this, or in case it regrows it regrows as this.

	public var damage:Float = 0;
	public var damageProtectionFactor:Float = 1;
	public var prestigeClass:PrestigeClass = PrestigeClass.NotSet;

	public var moves:Int = 0; // animal movements
	public var woundFactor:Float = 0.5; // player gets wound if X% hitpoints left
	public var animalEscapeFactor:Float = 0.7; // chance an animal escapes

	public var isBoat:Bool = false;

	public var carftingSteps:Int = -1;

	// saved

	/**
	 * Max clothing pieces
	 */
	public static inline var CLOTHING_PIECES:Int = 6;

	/**
	 * Id of object
	 */
	public var id:Int = 0;

	/**
	 * Description of object
	 */
	public var description:String = "";

	/**
	 * Whether an object is containable
	 */
	public var containable:Bool = false;

	/**
	 * N/A
	 */
	public var containSize:Float = 0.000000;

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
	 * Is held in hand boolean, hide closest arm
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
	public var mapChance:Float = 0.000000; // #biomes_0

	public var biomes:Array<Int> = [0];

	/**
	 * Heat value of object
	 */
	public var heatValue:Int = 0;

	/**
	 * N/A
	 */
	public var rValue:Float = 0.000000;

	/**
	 * If person
	 */
	public var person:Int = 0;

	/**
	 * N/A
	 */
	public var noSpawn:Bool = false;

	/**
	 * If person object is male
	 */
	public var male:Bool = false; // =0

	/**
	 * N/A
	 */
	public var deathMarker:Int = 0;

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
	public var foodValue:Int = 0;

	/**
	 * Multiply speed of player if held
	 */
	public var speedMult:Float = 1.000000;

	/**
	 * Position offset of object when held
	 */
	public var heldOffsetX:Float = 0; // =0.000000,0.000000

	public var heldOffsetY:Float = 0;

	/**
	 * N/A
	 */
	public var clothing:String = "n";

	/**
	 * weapon can not be dropped
	 */
	public var neverDrop:Bool = false;

	/**
	 * Offset of object when worn
	 */
	public var clothingOffsetX:Float = 0;

	public var clothingOffsetY:Float = 0;

	/**
	 * Deadly distance in tiles
	 */
	public var deadlyDistance:Float = 0;

	/**
	 * Use distance in tiles
	 */
	public var useDistance:Int = 1;

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
	public var numSlots:Int = 0;

	/**
	 * N/A
	 */
	public var timeStretch:Float = 1.000000;

	/**
	 * N/A
	 */
	public var slotSize:Float = 1;

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
	public var numSprites:Int = 0;

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

	// eyes offset
	// derrived automatically from whatever eyes are visible at age 30
	// (old eyes may have wrinkles around them, so they end up
	// getting centered differently)

	/**
	 * Generated index from sprites of eye index
	 */
	public var eyesOffsetX:Float = 0;

	public var eyesOffsetY:Float = 0;

	/**
	 * Number of uses for object
	 */
	public var numUses:Int = 0;

	/**
	 * Number of chances for object
	 */
	public var useChance:Float = 0;

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

	public var tool:Bool = false;

	/**
	 * Sound array for objects
	 */
	public var sounds:Vector<SoundData>; // =-1:0.250000,-1:0.250000,-1:0.250000,-1:1.000000

	/**
	 * dummy bool for generated object
	 */
	public var dummy:Bool = false;

	/**
	 * dummy parent id
	 */
	// public var dummyParent:Int = 0;
	public var dummyParent:ObjectData = null;

	/**
	 * dummyIndex, the amount of uses
	 */
	// public var dummyIndex:Int = 0;

	/**
	 * New Object Data
	 * @param i id
	 */
	var maxWideRadius:Int = 0;

	var onlyDescription:Bool;

	public var noBackAcess:Bool = false;

	/**
		gives back a non dummy id
	**/
	public var parentId(get, null):Int;

	public function get_parentId() {
		var objectData = this;

		if (objectData.dummyParent != null) return objectData.dummyParent.id;

		return objectData.id;
	}

	public static function getObjectData(id:Int) {
		return objectDataMap[id];
	}

	public static function DoAllTheObjectInititalisationStuff(init:Bool = false, spriteDataBool:Bool = false) {
		dataVersionNumber = Resource.dataVersionNumber();

		trace('dataVersionNumber: $dataVersionNumber');

		Init();

		if (init || ReadAllFromFile(dataVersionNumber, spriteDataBool) == false) {
			ImportObjectData();
			WriteAllToFile(dataVersionNumber);
		}

		GenerateSearchableNames();
		CreatePersonArray();
		CreateAndAddDummyObjectData();
		CreateFoodObjectArray();
		ServerSettings.PatchObjectData();
		GenerateBiomeObjectData();

		/*
			trace('Name: Axe id: ${GetObjectByName('Axe')}');
			trace('Name2: Axe id: ${GetObjectByName('Axe', false)}');
			trace('Name2: Steel Axe id: ${GetObjectByName('Steel Axe', false)}');
			trace('Name3: Steel Axe id: ${GetObjectByName('Steel Axe', false, true)}');

			trace('Name: Tree id: ${GetObjectByName('Tree')}');
			trace('Name2: Tree id: ${GetObjectByName('Tree', false)}');
			trace('Name3: Tree id: ${GetObjectByName('Tree', false, true)}');
		 */
	}

	// do before dummies
	private static function GenerateSearchableNames() {
		for (objData in objectDataMap) {
			var tmpName = objData.description.toUpperCase();
			tmpName = StringTools.replace(tmpName, '\n', '');
			tmpName = StringTools.replace(tmpName, '\r', '');

			// trace('Name1: $tmpName');

			tmpName = tmpName.split('#')[0];
			tmpName = tmpName.split('+')[0];
			tmpName = tmpName.split('-')[0];

			tmpName = StringTools.trim(tmpName);

			// trace('Name: ${objData.id} ${objData.description} --> $tmpName');

			objData.name = tmpName;
		}

		// var obj = getObjectData(6679);
		// trace('Name!!!: ${obj.name}');
	}

	public static function GetObjectByName(searchName:String, exactName:Bool = true, searchFromEnd:Bool = false):Int {
		searchName = searchName.toUpperCase();

		for (objData in ObjectData.importedObjectData) {
			if (searchName == objData.name) {
				return objData.id;
			}
		}

		if (exactName) return -1;

		for (objData in ObjectData.importedObjectData) {
			if (searchFromEnd) {
				if (StringTools.endsWith(objData.name, searchName)) return objData.id;
			} else {
				if (StringTools.startsWith(objData.name, searchName)) return objData.id;
			}
		}

		for (objData in ObjectData.importedObjectData) {
			if (objData.name.indexOf(searchName) != -1) return objData.id;
		}

		return -1;
	}

	public static function Init() {
		ObjectBake.objectList();
	}

	public static function ImportObjectData() {
		trace("Import Object Data...");
		var startTime = Sys.time();

		var tmp = ObjectBake.objectList();
		importedObjectData = new Vector<ObjectData>(tmp.length);

		objectDataMap = [];

		// Add empty and time ObjectData
		addEmptyAndTimeObjectData();

		for (i in 0...importedObjectData.length) {
			if (i % 400 == 0) trace('Create Object Data... $i from ${importedObjectData.length}');
			var objectData = new ObjectData(tmp[i]);
			importedObjectData[i] = objectData;
			objectDataMap[objectData.id] = objectData;
		}

		trace('Object Data imported: Time: ${Sys.time() - startTime} Count: ' + importedObjectData.length);
	}

	public static function addEmptyAndTimeObjectData() {
		objectDataMap[0] = new ObjectData(0, false, true);
		objectDataMap[-1] = new ObjectData(-1, false, true); // Add Time Object Data
		objectDataMap[-1].description = "TIME";
		objectDataMap[-1].id = -1;

		objectDataMap[-2] = new ObjectData(-2, false, true); // Add Player Object Data
		objectDataMap[-2].description = "PLAYER";
		objectDataMap[-2].id = -2;

		// trace('TEST: ${objectDataMap[-1].id}');
	}

	// link dummy ObjectData for objects with numUse > 2
	public static function CreateAndAddDummyObjectData() {
		// var dummyId = importedObjectData[importedObjectData.length-1].id + 1;

		var startingId = ObjectBake.nextObjectNumber;
		var dummyId = startingId;

		trace('starting dummyId :$startingId');
		if (startingId < importedObjectData[importedObjectData.length - 1].id + 1) throw new Exception('starting dummyId was not loaded correctly!');

		for (i in 0...importedObjectData.length) {
			if (importedObjectData[i].numUses < 2) continue;

			for (ii in 0...importedObjectData[i].numUses - 1) {
				// if(importedObjectData[i].id <= 30) trace('id: ${importedObjectData[i].id} dummyID: $dummyId ${importedObjectData[i].description}');

				var dummy = importedObjectData[i].clone();
				dummy.dummy = true;
				dummy.id = dummyId;
				dummy.dummyParent = importedObjectData[i];
				importedObjectData[i].dummyObjects.push(dummy);

				objectDataMap[dummyId] = importedObjectData[i];

				dummyId++;
			}
		}

		trace('finished adding dummy objects with numUse > 2 :${dummyId - startingId}');
	}

	public static function CreateFoodObjectArray() {
		foodObjects = [];
		// var index = 0;

		for (obj in importedObjectData) {
			if (obj.foodValue < 1) continue;

			// index++;

			// trace('Food: $index id: ${obj.id} ${obj.description}');

			foodObjects.push(obj);
		}
	}

	public static function GenerateBiomeObjectData() {
		biomeObjectData = [];
		biomeTotalChance = [];

		for (obj in importedObjectData) {
			if (obj.mapChance == 0) continue;

			if (obj.description.indexOf("Expert Way Stone") != -1) {
				obj.mapChance = 0;
				continue;
			}

			for (biome in obj.biomes) {
				var biomeData = biomeObjectData[biome];
				if (biomeData == null) {
					biomeData = [];
					biomeObjectData[biome] = biomeData;
					biomeTotalChance[biome] = 0;
				}
				biomeData.push(obj);
				biomeTotalChance[biome] += obj.mapChance;

				// var objectDataTarget = Server.objectDataMap[obj.id];
				// if(objectDataTarget != null) trace('biome: $biome c:${obj.mapChance} tc:${this.biomeTotalChance[biome]} ${objectDataTarget.description}');
			}
		}
	}

	public function new(i:Int = 0, onlyDescription:Bool = false, createNullObject:Bool = false) {
		super();

		if (i == 0 || createNullObject) {
			this.description = "EMPTY";
			this.name = "EMPTY";
			return;
		}
		if (i == -1) {
			this.description = "TIME";
			this.name = "TIME";
			return;
		}
		if (i == -2) {
			this.description = "PLAYER";
			this.name = "PLAYER";
			return;
		}

		this.onlyDescription = onlyDescription;
		var string:String;
		try {
			string = openlife.resources.Resource.objectData(i);
		} catch (e) {
			var int = ObjectBake.dummiesMap.get(i);
			if (int == null) return;
			string = openlife.resources.Resource.objectData(int);
		}
		if (i <= 0 || !readLines(string)) return;
		read();
	}

	/**
	 * Read data to set
	 */
	public /*inline*/ function read() {
		id = getInt();
		description = getString();
		if (onlyDescription) return;
		// tool setup
		var toolPos = description.indexOf("+tool");
		if (toolPos > -1) {
			var setTag = description.substring(toolPos + 5, description.length);
			var set:Bool = false;
			for (record in toolsetRecord) {
				if (record.setTag == setTag) {
					// already exists
					if (record.setMembership.indexOf(id) == -1) {
						record.setMembership.push(id);
						set = true;
						break;
					}
				}
			}
			// new
			if (!set) toolsetRecord.push({setTag: setTag, setMembership: [id]});
			tool = true;
		}
		if (description.indexOf("+noBackAccess") > -1) {
			noBackAcess = true;
		}
		containable = getBool();

		var i = getArrayInt();
		containSize = i[0];
		vertSlotRot = i[1];

		i = getArrayInt();
		permanent = i[0];
		minPickupAge = i[1];

		if (readName("noFlip")) {
			noFlip = getBool();
		}
		if (readName("sideAccess")) {
			sideAcess = getBool();
		}

		var string = getString();
		if (string == "1") heldInHand = true;
		if (string == "2") rideable = true;
		i = getArrayInt();
		blocksWalking = (i[0] == 1);
		leftBlockingRadius = i[1];
		rightBlockingRadius = i[2];
		drawBehindPlayer = (i[3] == 1);

		var wide = (leftBlockingRadius > 0 || rightBlockingRadius > 0);

		if (wide) {
			drawBehindPlayer = true;
			if (leftBlockingRadius > maxWideRadius) {
				maxWideRadius = leftBlockingRadius;
			}
			if (rightBlockingRadius > maxWideRadius) {
				maxWideRadius = rightBlockingRadius;
			}
		}

		var map = getString(); // mapchance
		var index = map.indexOf("#");
		mapChance = Std.parseFloat(map.substring(0, index));
		if (mapChance > 0) {
			// extract biome from spawn data
			index = map.indexOf("_", index);
			var array = map.substring(index + 1, map.length).split(",");
			biomes = [];
			for (string in array)
				biomes.push(Std.parseInt(string));
			// trace('MC: $map');
		}
		// values
		heatValue = getInt();
		rValue = getFloat();

		i = getArrayInt();
		// person is the race of the person
		person = i[0];
		noSpawn = (i[2] == 1);

		male = getBool();

		deathMarker = getInt();

		// from death (I don't know what this does)
		if (readName("fromDeath")) {
			trace("from death " + line[next]);
		}
		if (readName("homeMarker")) {
			homeMarker = getInt();
		}
		if (readName("floor")) {
			floor = getBool();
		}
		if (readName("floorHugging")) {
			floorHugging = getBool();
		}

		foodValue = getInt();
		speedMult = getFloat();

		var p = getPoint();
		heldOffsetX = p.x;
		heldOffsetY = p.y;

		clothing = getString();
		p = getPoint();
		clothingOffsetX = p.x;
		clothingOffsetY = p.y;

		deadlyDistance = getInt();

		if (readName("useDistance")) {
			useDistance = getInt();
		}
		if (readName("sounds")) {
			#if sound
			var array = getStringArray();
			sounds = new Vector<SoundData>(array.length);
			for (i in 0...array.length)
				sounds[i] = new SoundData(array[i]);
			#else
			getString();
			#end
		}

		if (readName("creationSoundInitialOnly")) creationSoundInitialOnly = getBool();
		if (readName("creationSoundForce")) creationSoundForce = getBool();

		// num slots and time stretch
		string = getString();
		string = string.substring(0, string.indexOf("#"));
		numSlots = Std.parseInt(string);

		slotSize = getInt();
		if (readName("slotsLocked")) {
			slotsLocked = getInt();
		}
		slotPos = new Vector<Point>(numSlots);
		slotVert = new Vector<Bool>(numSlots);
		slotParent = new Vector<Int>(numSlots);
		var set:Int = 0;
		for (j in 0...numSlots) {
			string = getString();
			set = string.indexOf(",");
			slotPos[j] = new Point(Std.parseInt(string.substring(0, set)), Std.parseInt(string.substring(set + 1, set = string.indexOf(",", set))));
			set = string.indexOf("=", set) + 1;
			slotVert[j] = string.substring(set, set = string.indexOf(",", set)) == "1";
			set = string.indexOf("=", set) + 1;
			slotParent[j] = Std.parseInt(string.substring(set, string.length));
		}
		// visual
		numSprites = getInt();
		// monument description set bools
		if (description.indexOf("monument") > -1) monumentStep = true;
		if (description.indexOf("monumentStep") > -1) monumentStep = true;
		if (description.indexOf("monumentCall") > -1) monumentCall = true;

		spriteArray = new Vector<SpriteData>(numSprites);
		for (j in 0...numSprites) {
			spriteArray[j] = new SpriteData();
			spriteArray[j].spriteID = getInt();
			p = getPoint();
			spriteArray[j].x = p.x;
			spriteArray[j].y = p.y;
			spriteArray[j].rot = getFloat();
			spriteArray[j].hFlip = getInt();
			spriteArray[j].color = getFloatArray();
			spriteArray[j].ageRange = getFloatArray();
			spriteArray[j].parent = getInt();
			// invis holding, invisWorn, behind slots
			var array = getArrayInt();
			spriteArray[j].invisHolding = array[0];
			spriteArray[j].invisWorn = array[1];
			spriteArray[j].behindSlots = array[2];
			if (readName("invisCont")) spriteArray[j].invisCont = getBool();
		}
		// get offset center
		getSpriteData();

		// extra
		if (readName("spritesDrawnBehind")) {
			// throw("sprite drawn behind " + line[next]);
			next++;
		}
		if (readName("spritesAdditiveBlend")) {
			// throw("sprite additive blend " + line[next]);
			next++;
		}

		headIndex = getInt();
		bodyIndex = getInt();
		// arrays
		backFootIndex = getInt();
		frontFootIndex = getInt();

		if (next < line.length) {
			var array = getFloatArray();
			numUses = Std.int(array[0]);
			if (array.length > 1) {
				useChance = array[1];
			}
			if (next < line.length) useVanishIndex = getIntArray();
			if (next < line.length) useAppearIndex = getIntArray();
			if (next < line.length) cacheHeight = getInt(); // pixHeight
		}
	}

	public function getSpriteData() {
		// get sprite data
		for (i in 0...spriteArray.length) {
			var s:String = openlife.resources.Resource.spriteData(spriteArray[i].spriteID);
			if (s.length == 0) continue;
			var j:Int = 0;
			var a = s.split(" ");
			for (string in a) {
				switch (j++) {
					case 0:
						// name
						spriteArray[i].name = string;
					case 1:
					// multitag

					case 2:
						// centerX
						spriteArray[i].inCenterXOffset = Std.parseInt(string);
					case 3:
						// centerY
						spriteArray[i].inCenterYOffset = Std.parseInt(string);
				}
			}
		}
	}

	public inline function isNatural():Bool {
		return mapChance > 0;
	}

	public function isSpawningIn(biomeId:Int):Bool {
		var objData = this;

		if (countsOrGrowsAs != 0) objData = ObjectData.getObjectData(countsOrGrowsAs);

		for (biome in objData.biomes) {
			if (biomeId == biome) {
				return true;
			}
		}

		return false;
	}

	public function toFileString():String {
		if (this.id == 0) return "Empty";

		var objectString = 'id=$id${LineReader.EOL}'
			+ '$description${LineReader.EOL}'
			+ 'containable=${containable ? "1" : "0"}${LineReader.EOL}'
			+ 'containSize=$containSize,vertSlotRot=$vertSlotRot${LineReader.EOL}'
			+ 'permanent=$permanent,minPickupAge=$minPickupAge${LineReader.EOL}'
			+ 'noFlip=${noFlip ? "1" : "0"}${LineReader.EOL}'
			+ 'sideAccess=${sideAcess ? "1" : "0"}${LineReader.EOL}'
			+ 'heldInHand=${heldInHand ? "1" : "0"}${LineReader.EOL}'
			+
			'blocksWalking=${blocksWalking ? "1" : "0"},leftBlockingRadius=$leftBlockingRadius,rightBlockingRadius=$rightBlockingRadius,drawBehindPlayer=${drawBehindPlayer ? "1" : "0"}${LineReader.EOL}'
			+ 'mapChance=$mapChance${LineReader.EOL}'
			+ // TODO: include #biomes_0
			'heatValue=$heatValue${LineReader.EOL}'
			+ 'rValue=$rValue${LineReader.EOL}'
			+ 'person=$person,noSpawn=${noSpawn ? "1" : "0"}${LineReader.EOL}'
			+ 'male=${male ? "1" : "0"}${LineReader.EOL}'
			+ 'deathMarker=$deathMarker${LineReader.EOL}'
			+ 'homeMarker=$homeMarker${LineReader.EOL}'
			+ 'floor=${floor ? "1" : "0"}${LineReader.EOL}'
			+ 'floorHugging=${floorHugging ? "1" : "0"}${LineReader.EOL}'
			+ 'foodValue=$foodValue${LineReader.EOL}'
			+ 'speedMult=$speedMult${LineReader.EOL}'
			+ 'heldOffset=${heldOffsetX},${heldOffsetY}${LineReader.EOL}'
			+ 'clothing=$clothing${LineReader.EOL}'
			+ 'clothingOffset=${clothingOffsetX},${clothingOffsetY}${LineReader.EOL}'
			+ 'deadlyDistance=$deadlyDistance${LineReader.EOL}'
			+ 'useDistance=$useDistance${LineReader.EOL}'
			+ 'sounds=0:0,0:0.0,0:0.0,0:0.0${LineReader.EOL}'
			+ // TODO: implement sound
			'creationSoundInitalOnly=$creationSoundInitialOnly${LineReader.EOL}'
			+ 'creationSoundForce=$creationSoundForce${LineReader.EOL}'
			+ 'numSlots=$numSlots#timeStrech=$timeStretch${LineReader.EOL}'
			+ 'slotsSize=$slotSize${LineReader.EOL}'
			+ 'slotsLocked=$slotsLocked${LineReader.EOL}'
			+ 'numSprites=$numSprites${LineReader.EOL}';
		for (sprite in spriteArray)
			objectString += sprite.toString(); // add sprite data
		return objectString;
	}

	/**
	 * clone data
	 */
	public function clone():ObjectData {
		var object = new ObjectData();
		object.id = id;
		object.apocalypseTrigger = apocalypseTrigger;
		object.backFootIndex = backFootIndex;
		object.blocksWalking = blocksWalking;
		object.bodyIndex = bodyIndex;
		object.cacheHeight = cacheHeight;
		object.clothing = clothing;
		object.clothingOffsetX = clothingOffsetX;
		object.clothingOffsetY = clothingOffsetY;
		object.containSize = containSize;
		object.containable = containable;
		object.creationSoundForce = creationSoundForce;
		object.creationSoundInitialOnly = creationSoundInitialOnly;
		object.deadlyDistance = deadlyDistance;
		object.deathMarker = deathMarker;
		object.description = description;
		object.drawBehindPlayer = drawBehindPlayer;
		object.eyesIndex = eyesIndex;
		object.eyesOffsetX = eyesOffsetX;
		object.eyesOffsetY = eyesOffsetY;
		object.floor = floor;
		object.floorHugging = floorHugging;
		object.foodValue = foodValue;
		object.frontFootIndex = frontFootIndex;
		object.headIndex = headIndex;
		object.heatValue = heatValue;
		object.heldInHand = heldInHand;
		object.heldOffsetX = heldOffsetX;
		object.heldOffsetY = heldOffsetY;
		object.homeMarker = homeMarker;
		object.id = id;
		object.leftBlockingRadius = leftBlockingRadius;
		object.male = male;
		object.mapChance = mapChance;
		object.minPickupAge = minPickupAge;
		object.monumentCall = monumentCall;
		object.monumentDone = monumentCall;
		object.monumentStep = monumentStep;
		object.mouthIndex = mouthIndex;
		object.neverDrop = neverDrop;
		object.noFlip = noFlip;
		object.noSpawn = noSpawn;
		object.numSlots = numSlots;
		object.numSprites = numSprites;
		object.numUses = numUses;
		object.permanent = permanent;
		object.person = person;
		object.rValue = rValue;
		object.rideable = rideable;
		object.rightBlockingRadius = rightBlockingRadius;
		object.sideAcess = sideAcess;
		object.slotParent = slotParent;
		object.slotPos = slotPos;
		object.slotSize = slotSize;
		object.slotVert = slotVert;
		object.slotsLocked = slotsLocked;
		object.sounds = sounds;
		object.speedMult = speedMult;
		object.spriteArray = spriteArray;
		object.timeStretch = timeStretch;
		object.toolLearned = toolLearned;
		object.toolSetIndex = toolSetIndex;
		object.useAppearIndex = useAppearIndex;
		object.useChance = useChance;
		object.useDistance = useDistance;
		object.useVanishIndex = useVanishIndex;
		object.vertSlotRot = vertSlotRot;
		return object;
	}

	private static function writeToFile(obj:ObjectData, writer:FileOutput) {
		writer.writeInt32(obj.id);
		writer.writeFloat(obj.decayFactor);
		writer.writeInt16(obj.description.length);
		writer.writeString(obj.description);
		writer.writeInt8(obj.containable ? 1 : 0);
		writer.writeFloat(obj.containSize);
		writer.writeInt8(obj.noFlip ? 1 : 0);
		writer.writeInt8(obj.sideAcess ? 1 : 0);
		writer.writeFloat(obj.vertSlotRot);
		writer.writeInt32(obj.permanent);
		writer.writeInt32(obj.minPickupAge);
		writer.writeInt8(obj.heldInHand ? 1 : 0);
		writer.writeInt8(obj.rideable ? 1 : 0);
		writer.writeInt8(obj.blocksWalking ? 1 : 0);
		writer.writeInt32(obj.leftBlockingRadius);
		writer.writeInt32(obj.rightBlockingRadius);
		writer.writeInt8(obj.drawBehindPlayer ? 1 : 0);
		writer.writeFloat(obj.mapChance);
		writer.writeInt16(obj.biomes.length);
		for (i in obj.biomes)
			writer.writeInt32(i);
		writer.writeInt32(obj.heatValue);
		writer.writeFloat(obj.rValue);
		writer.writeInt32(obj.person);
		writer.writeInt8(obj.noSpawn ? 1 : 0);
		writer.writeInt8(obj.male ? 1 : 0);
		writer.writeInt32(obj.deathMarker);
		writer.writeInt32(obj.homeMarker);
		writer.writeInt8(obj.floor ? 1 : 0);
		writer.writeInt8(obj.floorHugging ? 1 : 0);
		writer.writeInt32(obj.foodValue);
		writer.writeFloat(obj.speedMult);
		writer.writeInt16(obj.clothing.length);
		writer.writeString(obj.clothing);
		writer.writeInt8(obj.neverDrop ? 1 : 0);
		writer.writeFloat(obj.deadlyDistance);
		writer.writeInt32(obj.useDistance);
		writer.writeInt8(obj.creationSoundInitialOnly ? 1 : 0);
		writer.writeInt8(obj.creationSoundForce ? 1 : 0);
		writer.writeInt32(obj.numSlots);
		writer.writeFloat(obj.timeStretch);
		writer.writeFloat(obj.slotSize);
		writer.writeInt32(obj.slotsLocked);
		writer.writeInt32(obj.numSprites);
		writer.writeInt32(obj.headIndex);
		writer.writeInt32(obj.bodyIndex);
		writer.writeInt32(obj.backFootIndex);
		writer.writeInt32(obj.frontFootIndex);
		writer.writeInt32(obj.eyesIndex);
		writer.writeInt32(obj.mouthIndex);
		writer.writeInt32(obj.numUses);
		writer.writeFloat(obj.useChance);
		writer.writeInt16(obj.useVanishIndex.length);
		for (i in obj.useVanishIndex)
			writer.writeInt32(i);
		writer.writeInt16(obj.useAppearIndex.length);
		for (i in obj.useAppearIndex)
			writer.writeInt32(i);
		writer.writeInt32(obj.cacheHeight);
		writer.writeInt8(obj.apocalypseTrigger ? 1 : 0);
		writer.writeInt8(obj.monumentStep ? 1 : 0);
		writer.writeInt8(obj.monumentDone ? 1 : 0);
		writer.writeInt8(obj.monumentCall ? 1 : 0);
		writer.writeInt32(obj.toolSetIndex);
		writer.writeInt8(obj.toolLearned ? 1 : 0);
		writer.writeInt8(obj.tool ? 1 : 0);
		writer.writeInt8(obj.dummy ? 1 : 0);
		writer.writeInt32(obj.maxWideRadius);
		writer.writeInt8(obj.onlyDescription ? 1 : 0);
		writer.writeInt8(obj.noBackAcess ? 1 : 0);
		// spritedata
		var spriteWriter = new haxe.io.BytesOutput();
		spriteWriter.writeInt32(obj.spriteArray.length); // length of sprite array
		for (sprite in obj.spriteArray) {
			spriteWriter.writeInt32(sprite.spriteID);
			spriteWriter.writeInt32(sprite.ageRange.length);
			for (age in sprite.ageRange) {
				spriteWriter.writeFloat(age);
			}
			spriteWriter.writeInt32(sprite.behindSlots);
			spriteWriter.writeInt32(sprite.color.length);
			for (color in sprite.color) {
				spriteWriter.writeFloat(color);
			}
			spriteWriter.writeInt32(sprite.hFlip);
			spriteWriter.writeInt32(sprite.inCenterXOffset);
			spriteWriter.writeInt32(sprite.inCenterYOffset);
			spriteWriter.writeInt8(sprite.invisCont ? 1 : 0);
			spriteWriter.writeInt32(sprite.invisHolding);
			spriteWriter.writeInt32(sprite.invisWorn);
			spriteWriter.writeInt32(sprite.name.length); // length of name string
			spriteWriter.writeString(sprite.name);
			spriteWriter.writeInt32(sprite.parent);
			spriteWriter.writeFloat(sprite.x);
			spriteWriter.writeFloat(sprite.y);
			spriteWriter.writeFloat(sprite.rot);
		}
		writer.writeInt32(spriteWriter.length);
		writer.writeBytes(spriteWriter.getBytes(), 0, spriteWriter.length);
		writer.writeInt32(obj.id); // write twice to check if data is corrupted
	}

	// note to future self, if you happen to wander by, the client doesn't save special data such as sound data
	public static function readFromFile(obj:ObjectData, reader:haxe.io.Input, spriteDataBool:Bool = false) {
		obj.id = reader.readInt32();
		obj.decayFactor = reader.readFloat();
		var len = reader.readInt16();
		obj.description = reader.readString(len);
		obj.containable = reader.readInt8() != 0 ? true : false;
		obj.containSize = reader.readFloat();
		obj.noFlip = reader.readInt8() != 0 ? true : false;
		obj.sideAcess = reader.readInt8() != 0 ? true : false;
		obj.vertSlotRot = reader.readFloat();
		obj.permanent = reader.readInt32();
		obj.minPickupAge = reader.readInt32();
		obj.heldInHand = reader.readInt8() != 0 ? true : false;
		obj.rideable = reader.readInt8() != 0 ? true : false;
		obj.blocksWalking = reader.readInt8() != 0 ? true : false;
		obj.leftBlockingRadius = reader.readInt32();
		obj.rightBlockingRadius = reader.readInt32();
		obj.drawBehindPlayer = reader.readInt8() != 0 ? true : false;
		obj.mapChance = reader.readFloat();
		obj.biomes = new Array<Int>();
		var len = reader.readInt16();
		for (i in 0...len) {
			obj.biomes[i] = reader.readInt32();
		}
		obj.heatValue = reader.readInt32();
		obj.rValue = reader.readFloat();
		obj.person = reader.readInt32();
		obj.noSpawn = reader.readInt8() != 0 ? true : false;
		obj.male = reader.readInt8() != 0 ? true : false;
		obj.deathMarker = reader.readInt32();
		obj.homeMarker = reader.readInt32();
		obj.floor = reader.readInt8() != 0 ? true : false;
		obj.floorHugging = reader.readInt8() != 0 ? true : false;
		obj.foodValue = reader.readInt32();
		obj.speedMult = reader.readFloat();
		var len = reader.readInt16();
		obj.clothing = reader.readString(len);
		obj.neverDrop = reader.readInt8() != 0 ? true : false;
		obj.deadlyDistance = reader.readFloat();
		obj.useDistance = reader.readInt32();
		obj.creationSoundInitialOnly = reader.readInt8() != 0 ? true : false;
		obj.creationSoundForce = reader.readInt8() != 0 ? true : false;
		obj.numSlots = reader.readInt32();
		obj.timeStretch = reader.readFloat();
		obj.slotSize = reader.readFloat();
		obj.slotsLocked = reader.readInt32();
		obj.numSprites = reader.readInt32();
		obj.headIndex = reader.readInt32();
		obj.bodyIndex = reader.readInt32();
		obj.backFootIndex = reader.readInt32();
		obj.frontFootIndex = reader.readInt32();
		obj.eyesIndex = reader.readInt32();
		obj.mouthIndex = reader.readInt32();
		obj.numUses = reader.readInt32();
		obj.useChance = reader.readFloat();
		obj.useVanishIndex = new Array<Int>();
		var len = reader.readInt16();
		for (i in 0...len) {
			obj.useVanishIndex[i] = reader.readInt32();
		}
		obj.useAppearIndex = new Array<Int>();
		var len = reader.readInt16();
		for (i in 0...len) {
			obj.useAppearIndex[i] = reader.readInt32();
		}
		obj.cacheHeight = reader.readInt32();
		obj.apocalypseTrigger = reader.readInt8() != 0 ? true : false;
		obj.monumentStep = reader.readInt8() != 0 ? true : false;
		obj.monumentDone = reader.readInt8() != 0 ? true : false;
		obj.monumentCall = reader.readInt8() != 0 ? true : false;
		obj.toolSetIndex = reader.readInt32();
		obj.toolLearned = reader.readInt8() != 0 ? true : false;
		obj.tool = reader.readInt8() != 0 ? true : false;
		obj.dummy = reader.readInt8() != 0 ? true : false;
		obj.maxWideRadius = reader.readInt32();
		obj.onlyDescription = reader.readInt8() != 0 ? true : false;
		obj.noBackAcess = reader.readInt8() != 0 ? true : false;

		var spriteDataLen = reader.readInt32();

		// spritedata
		if (spriteDataBool) {
			var len = reader.readInt32();
			obj.spriteArray = new Vector<SpriteData>(len);
			for (i in 0...obj.spriteArray.length) {
				final sprite = new SpriteData();
				obj.spriteArray[i] = sprite;
				sprite.spriteID = reader.readInt32();
				var len = reader.readInt32();
				for (i in 0...len) {
					sprite.ageRange[i] = reader.readFloat();
				}
				sprite.behindSlots = reader.readInt32();
				var len = reader.readInt32();
				for (i in 0...len) {
					sprite.color[i] = reader.readFloat();
				}
				sprite.hFlip = reader.readInt32();
				sprite.inCenterXOffset = reader.readInt32();
				sprite.inCenterYOffset = reader.readInt32();
				sprite.invisCont = reader.readInt8() == 1;
				sprite.invisHolding = reader.readInt32();
				sprite.invisWorn = reader.readInt32();
				var len = reader.readInt32();
				sprite.name = reader.readString(len);
				sprite.parent = reader.readInt32();
				sprite.x = reader.readFloat();
				sprite.y = reader.readFloat();
				sprite.rot = reader.readFloat();
			}
		} else {
			reader.readAll(spriteDataLen);
		}
		var tmpId = reader.readInt32();
		if (obj.id != tmpId) {
			var errorMessage = 'Read Object Data Corrupted: ObjectId: ${obj.id} != $tmpId';
			trace(errorMessage);
			throw new Exception(errorMessage);
		}

		// trace('${obj.id}: ' + obj.description);
	}

	public static function WriteAllToFile(dataVersionNumber:Int) {
		var startTime = Sys.time();
		var dir = './${ServerSettings.SaveDirectory}/';
		var path = dir + "saveObjectData.bin";

		if (FileSystem.exists(dir) == false) FileSystem.createDirectory(dir);

		var writer = File.write(path, true);

		writer.writeInt32(dataVersionNumber);
		writer.writeInt32(importedObjectData.length);

		for (objectData in importedObjectData) {
			writeToFile(objectData, writer);
		}

		trace('Write ${importedObjectData.length}  data version number: ${dataVersionNumber} ObjectData Time: ${Sys.time() - startTime}');

		writer.writeInt32(-1);

		writer.close();
	}

	public static function ReadAllFromFile(dataVersionNumber:Int, spriteDataBool:Bool = false):Bool {
		var reader = null;

		try {
			var startTime = Sys.time();
			var dir = './${ServerSettings.SaveDirectory}/';
			var path = dir + "saveObjectData.bin";

			reader = new haxe.io.BytesInput(File.getBytes(path)); // File.read(path, true);
			var fileVersionNumber = reader.readInt32();
			if (fileVersionNumber != dataVersionNumber)
				throw new Exception('server data version number ${dataVersionNumber} did not fit with file $fileVersionNumber');
			var count = reader.readInt32();

			importedObjectData = new Vector<ObjectData>(count);
			objectDataMap = [];
			addEmptyAndTimeObjectData();

			for (i in 0...count) {
				var obj = new ObjectData();
				readFromFile(obj, reader, spriteDataBool);
				importedObjectData[i] = obj;
				objectDataMap[obj.id] = obj;
			}

			// reader.close();
			reader = null;

			trace('Read ${importedObjectData.length} ObjectData  data version number: ${dataVersionNumber} Time: ${Sys.time() - startTime}');
		} catch (ex) {
			trace(ex);
			// if (reader != null) reader.close();
			reader = null;
			return false;
		}

		return true;
	}

	// insulation reaches from 0 to 2
	public function getInsulation():Float {
		// original: {'h': 0.25, 't': 0.35, 'b': 0.2, 's': 0.1, 'p': 0.1};
		var parts:Map<String, Float> = ["h" => 0.4, "t" => 0.4, "b" => 0.4, "s" => 0.2, "p" => 0.4];

		// trace('Insulation: clothing: ${this.clothing} ' + parts);

		if (this.clothing.length > 1) this.clothing = StringTools.trim(this.clothing);

		if (parts[this.clothing] == 0) return this.rValue;

		// trace('Insulation: clothing: ${this.clothing} ${this.clothing.length} ${parts[this.clothing]}');

		if (rValue > 0) return parts[this.clothing] * rValue; else
			return parts[this.clothing];
	}

	// TODO allow to set custom heat value that overrides rvalue
	public function getHeatProtection():Float {
		// original: {'h': 0.25, 't': 0.35, 'b': 0.2, 's': 0.1, 'p': 0.1};
		var parts:Map<String, Float> = ["h" => 0.4, "t" => 0.4, "b" => 0.4, "s" => 0.2, "p" => 0.4];

		// trace('Insulation: clothing: ${this.clothing} ' + parts);

		if (this.clothing.length > 1) this.clothing = StringTools.trim(this.clothing);

		if (parts[this.clothing] == 0) return this.rValue;

		// trace('Insulation: clothing: ${this.clothing} ${this.clothing.length} ${parts[this.clothing]}');

		if (rValue > 0) return parts[this.clothing] * (1 - rValue); else
			return parts[this.clothing];
	}

	public function isDrugs():Bool {
		// 838 Dont eat the dam drugs! Wormless Soil Pit with Mushroom // 837 Psilocybe Mushroom
		return (this.id == 838 || this.id == 837);
	}

	public static function CreatePersonArray() {
		personObjectData = [];
		maleByRaceObjectData = new Map<Int, Array<ObjectData>>();
		femaleByRaceObjectData = new Map<Int, Array<ObjectData>>();

		for (obj in ObjectData.importedObjectData) {
			if (obj.person == 0) continue;

			if (obj.description.indexOf('Jason') != -1) continue;

			personObjectData.push(obj);

			var personByRace = obj.male ? maleByRaceObjectData : femaleByRaceObjectData;

			var race = personByRace[obj.person];

			if (race == null) {
				race = [];
				personByRace[obj.person] = race;
			}

			race.push(obj);

			// trace('${obj.description} Id: ${obj.parentId} P: ${obj.person} race-length: ${race.length}');
		}
	}

	public function getTimeTrans():TransitionData {
		var timeTransition = TransitionImporter.GetTransition(-1, this.parentId, false, false);

		// trace('TIME: has time transition: ${transition.newTargetID} ${newTargetObjectData.description} time: ${timeTransition.autoDecaySeconds}');

		return timeTransition;
	}

	/**Considers food parent in case of dummy and if food can be made with one transition**/
	public function getFoodId():Int {
		var foodObjData = dummyParent == null ? this : dummyParent;
		var foodId = foodObjData.foodFromTarget == null ? foodObjData.parentId : foodObjData.foodFromTarget.parentId;
		return foodId;
	}

	// TODO does not work for attacking wolf etc...
	public function isAnimal():Bool {
		return this.moves > 0 && this.id != 2156; // no Mosquito Swarm
	}

	public function getPileObjId() {
		var trans = TransitionImporter.GetTransition(this.id, this.id);
		if (trans == null) return -1;
		return trans.newTargetID;
	}

	public function getClothingSlot():Int {
		var objClothingSlot = -1;

		// if (this.o_id[0] < 1) return -1;

		var objectData = this; // ObjectData.getObjectData(this.o_id[0]);
		// trace("OD: " + objectData.toFileString());

		switch objectData.clothing.charAt(0) {
			case "h":
				objClothingSlot = 0; // head
			case "t":
				objClothingSlot = 1; // torso
			case "s":
				objClothingSlot = 2; // shoes
			// case "s": objClothingSlot = 3;    // shoes
			case "b":
				objClothingSlot = 4; // skirt / trouser
			case "p":
				objClothingSlot = 5; // backpack
		}

		// if (ServerSettings.DebugPlayer) trace('objectData.clothing: ${objectData.clothing} objClothingSlot:  ${objClothingSlot}');
		// trace('clothingSlot:  ${clothingSlot}');

		return objClothingSlot;
	}
}
