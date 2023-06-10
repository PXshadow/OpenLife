package openlife.data.object;

import haxe.Exception;
import haxe.ds.Vector;
import openlife.auto.PlayerInterface;
import openlife.data.transition.TransitionImporter;
import openlife.server.GlobalPlayerInstance;
import openlife.server.Lineage;
import openlife.server.PlayerAccount;
import openlife.server.TimeHelper;
import openlife.server.WorldMap;
import openlife.settings.ServerSettings;
import sys.io.File;
import sys.io.FileInput;
import sys.io.FileOutput;

@:enum abstract ObjectDataSaveIds(Int) from Int to Int {
	public var HITS = 1;
	public var COINS = 2;
	public var TEXT = 3;
	public var EXTERNID = 4; // For example used for key ID
	public var COUNTOBJ = 5; // For example count foritification material
}

class ObjectHelper {
	public static var dataVersionNumberForRead = 6;
	public static var dataVersionNumberForWrite = 6;

	public var objectData:ObjectData;
	public var numberOfUses = 0;
	public var creationTimeInTicks:Float;

	/**Time to next change in seconds / needed for time Transitions**/
	public var timeToChange:Float = 0;

	public var tx:Int = 0;
	public var ty:Int = 0;

	// public var preferredBiome:Int; // used for movement
	// needed to store ground object in case something moves on top
	public var groundObject:ObjectHelper;

	private var ownersByPlayerAccount:Array<Int> = [];
	private var livingOwners:Array<Int> = [];

	// to store contained objects in case object is a container
	public var containedObjects:Array<ObjectHelper> = [];

	public var hits:Float = 0;
	public var coins:Float = 0;
	public var text:String = '';
	public var externId:Int = 0; // For example for lock / key
	public var countObj:Float = 0; // For example count foritification material

	// public var isLovedSet = false; // not saved TODO save
	public var lovedTx:Int = 0; // not saved TODO save
	public var lovedTy:Int = 0; // not saved TODO saved

	public var failedMoves:Float = 0; // not saved yet
	public var target:ObjectHelper = null; // // not saved TODO saved

	public static function WriteMapObjHelpers(path:String, objHelpersToWrite:Vector<ObjectHelper>) {
		var width = WorldMap.world.width;
		var height = WorldMap.world.height;
		var length = WorldMap.world.length;

		// trace('Wrtie to file: $path width: $width height: $height length: $length');

		if (width * height != length) throw new Exception('width * height != length');
		if (objHelpersToWrite.length != length) throw new Exception('objHelpersToWrite.length != length');

		var count = 0;
		var dataVersion = ObjectHelper.dataVersionNumberForWrite;

		var writer = File.write(path, true);
		writer.writeInt32(dataVersion);
		writer.writeInt32(width);
		writer.writeInt32(height);

		for (obj in objHelpersToWrite) {
			if (obj == null) continue;
			// if (obj.id < 1>) continue;
			// if (obj.id < 1) trace('WriteMapObjHelpers: delete: ${obj.isHelperToBeDeleted()} numberOfUses: ${obj.numberOfUses} objectData.numUses: ${obj.objectData.numUses}');
			// if (obj.id < 1) trace('WriteMapObjHelpers: delete: ${obj.isHelperToBeDeleted()} containedObjects: ${obj.containedObjects.length} groundObject: ${obj.groundObject != null}');
			count++;
			WriteToFile(obj, writer);
		}

		writer.writeInt8(100); // end sign

		writer.close();

		if (ServerSettings.DebugWrite) trace('wrote $count ObjectHelpers...');
	}

	public static function ReadMapObjHelpers(path:String):Vector<ObjectHelper> {
		var reader = File.read(path, true);
		var expectedDataVersion = ObjectHelper.dataVersionNumberForRead;
		var dataVersion = reader.readInt32();
		var width = reader.readInt32();
		var height = reader.readInt32();
		var length = width * height;
		var count = 0;
		var newObjects = new Vector<ObjectHelper>(length);
		var world = WorldMap.world;

		world.objectHelpers = newObjects;

		trace('ReadMapObjHelpers: Data version is: $dataVersion expected data version is: $expectedDataVersion');

		if (dataVersion != expectedDataVersion)
			throw new Exception('ReadMapObjHelpers: Data version is: $dataVersion expected data version is: $expectedDataVersion');
		if (width != world.width) throw new Exception('width != this.width');
		if (height != world.height) throw new Exception('height != this.height');
		if (length != world.length) throw new Exception('length != this.length');

		trace('Read from file: $path width: $width height: $height length: $length');

		try {
			while (reader.eof() == false) {
				var newObject = ReadFromFile(reader, dataVersion);
				if (newObject == null) break;
				count++;
				// trace('read: $count');
				world.setObjectHelper(newObject.tx, newObject.ty, newObject);
				// newObjects[index(newObject.tx, newObject.ty)] = newObject;
				// objects[index(newObject.tx, newObject.ty)] = newObjArray;

				/*if(newObject.numberOfUses > 1 || newObject.containedObjects.length > 0)
					{
						// 1435 = bison // 1261 = Canada Goose Pond with Egg // 30 = Gooseberry Bush // 2142 = Banana Plant // 1323 = Wild Boar
						if(newObject.id != 1435 && newObject.id != 1261  && newObject.id != 30 && newObject.id != 2142 && newObject.id != 1323)
						{
							// trace('${newObject.description()} numberOfUses: ${newObject.numberOfUses} from  ${newObject.objectData.numUses} ' + newObjArray);
						}
				}*/
			}
		} catch (ex) {
			reader.close();
			throw ex;
		}

		reader.close();

		trace('read $count ObjectHelpers...');

		return newObjects;
	}

	public static function WriteToFile(obj:ObjectHelper, writer:FileOutput) {
		if (obj == null) {
			WorldMap.WriteInt32Array(writer, [-100]);
			return;
		}

		// save objects with parent ID, so that dummies can be fixed on data update to a new version
		// the new dummy ID is calculated with number of uses
		WorldMap.WriteInt32Array(writer, obj.toArray(true));
		WorldMap.WriteInt32Array(writer, obj.livingOwners);
		WorldMap.WriteInt32Array(writer, obj.ownersByPlayerAccount);

		writer.writeInt32(obj.tx);
		writer.writeInt32(obj.ty);
		writer.writeInt32(obj.numberOfUses);
		writer.writeDouble(obj.creationTimeInTicks);
		writer.writeFloat(obj.timeToChange);

		// write custom variables
		var count = 0;
		if (obj.hits != 0) count++;
		if (obj.coins != 0) count++;
		if (obj.text != '') count++;
		if (obj.externId != 0) count++;
		if (obj.countObj != 0) count++;

		writer.writeInt16(count);
		if (obj.hits != 0) writer.writeInt16(HITS);
		if (obj.hits != 0) writer.writeFloat(obj.hits);
		if (obj.coins != 0) writer.writeInt16(COINS);
		if (obj.coins != 0) writer.writeFloat(obj.coins);
		if (obj.text != '') writer.writeInt16(TEXT);
		if (obj.text != '') writer.writeInt16(obj.text.length);
		if (obj.text != '') writer.writeString(obj.text);
		if (obj.externId != 0) writer.writeInt16(EXTERNID);
		if (obj.externId != 0) writer.writeInt32(obj.externId);
		if (obj.countObj != 0) writer.writeInt16(COUNTOBJ);
		if (obj.countObj != 0) writer.writeFloat(obj.countObj);

		// write contained objects
		writer.writeByte(obj.containedObjects.length);

		for (containedObj in obj.containedObjects) {
			WriteToFile(containedObj, writer);
			// trace('containedObj: ${containedObj.name}');
		}
	}

	// TODO use dataVersion 5 also for held objects / cloths / wounds
	// Dataversion 5 stores also full contained object data
	public static function ReadFromFile(reader:FileInput, dataVersion:Int = -1):ObjectHelper {
		if (dataVersion < 1) dataVersion = dataVersionNumberForRead;

		var array = WorldMap.ReadInt32Array(reader);
		if (array == null) return null; // reached the end
		if (array[0] == -100) return null;
		if (array.length < 0 || array.length > 100) {
			trace('Array lenght does not fit: ' + array.length);
			throw new Exception('Array lenght does not fit: ' + array.length);
		}

		var newObject = ObjectHelper.readObjectHelper(null, array);
		newObject.livingOwners = WorldMap.ReadInt32Array(reader);
		newObject.ownersByPlayerAccount = WorldMap.ReadInt32Array(reader);
		newObject.tx = reader.readInt32();
		newObject.ty = reader.readInt32();
		newObject.numberOfUses = reader.readInt32();
		newObject.creationTimeInTicks = reader.readDouble();
		newObject.timeToChange = reader.readFloat();

		// read custom variables
		if (dataVersion >= 6) {
			var count = reader.readInt16();
			for (i in 0...count) {
				var dataId = reader.readInt16();
				switch (dataId) {
					case HITS:
						newObject.hits = reader.readFloat();
						trace('ReadFromFile: Hits: ${newObject.hits}');
					case COINS:
						newObject.coins = reader.readFloat();
						trace('ReadFromFile: Coins: ${newObject.coins}');
					case TEXT:
						var lenght = reader.readInt16();
						newObject.text = reader.readString(lenght);
						trace('ReadFromFile: Text: ${newObject.text}');
					case EXTERNID:
						newObject.externId = reader.readInt32();
						trace('ReadFromFile: externId: ${newObject.externId}');
					case COUNTOBJ:
						newObject.countObj = reader.readFloat();
						trace('ReadFromFile: countObj: ${newObject.countObj}');
					default:
						trace('ERROR: DataId: ${dataId} is unknown!');
						throw new Exception('DataId: ${dataId} is unknown!');
				}
			}
		}

		// read contained objects
		if (dataVersion >= 5) {
			/*var count = 0;
				for(containedObj in newObject.containedObjects){
					trace('Read O: $count containedObj: ${containedObj.name}');
					count++;
			}*/

			newObject.containedObjects = new Array<ObjectHelper>();
			var count = reader.readByte();
			for (i in 0...count) {
				var containedObj = ReadFromFile(reader, dataVersion);
				newObject.containedObjects.push(containedObj);
				// trace('Read: $i containedObj: ${containedObj.name}');
			}
		}
		newObject.TransformToDummy();

		if (newObject.creationTimeInTicks > TimeHelper.tick) newObject.creationTimeInTicks = TimeHelper.tick;
		return newObject;
	}

	public function deleteEmptyObjects() {
		for (obj in containedObjects) {
			if (obj.id > 0) continue;
			trace('WARNING: deleteEmptyObjects: ${this.name} remove contained: ${obj.id}');
			containedObjects.remove(obj);
		}
	}

	public static function InitObjectHelpersAfterRead() {
		for (obj in WorldMap.world.objectHelpers) {
			if (obj == null) continue;
			if (obj.id == 0) continue;

			var creatorLinage = obj.getLinage();
			if (creatorLinage != null) {
				// trace('${obj.name} Owner: ${creatorLinage.name}');
				creatorLinage.ownsObject = true; // mark to not delete
			}

			if (obj.isGrave()) {
				for (id in obj.ownersByPlayerAccount) {
					var account = PlayerAccount.GetPlayerAccountById(id);
					if (account == null) continue;

					account.graves.push(obj);
					var creatorLinage = obj.getLinage();

					if (creatorLinage != null) {
						// trace('${obj.name} Owner: ${account.email} ${creatorLinage.name}');
						// TODO mark all owners of all objects not only graves
						creatorLinage.ownsObject = true; // mark to not delete
					} else
						trace('WARNING: ${obj.name} Owner: ${account.email}');
				}
			} else if (obj.isOwned()) {
				for (id in obj.livingOwners) {
					// trace('${obj.name} Owner: ${id}');

					var player = GlobalPlayerInstance.AllPlayerMap[id];
					if (player == null) {
						trace('WARNING: ${obj.name} Owner: ${id} not found');
						obj.livingOwners.remove(id);
						continue;
					}

					if (player.deleted) obj.removeOwner(player);
					player.owning.push(obj);

					// trace('${obj.name} Owner: ${player.name}');
				}
			}
		}
	}

	public static function readObjectHelper(creator:GlobalPlayerInstance, ids:Array<Int>, i:Int = 0):ObjectHelper {
		var id = ids[i];
		var isFirst = (i == 0);
		var isInSubcontainer = false;

		// trace('read: id:$id i:$i ids:$ids isInSubcontainer: $isInSubcontainer');

		// negative values are used for subcontained items
		if (id < 0) {
			isInSubcontainer = true;
			id *= -1;
		}

		var helper = new ObjectHelper(creator, id);

		if (isInSubcontainer) return helper;

		i++;

		// read container items
		while (i < ids.length) {
			// negative values are used only for subcontained items so skip them
			if (isFirst && ids[i] < 0) {
				i++;
				continue;
			}

			// in subcontainer contained items must be negative, so return if there is no negative item
			if (isFirst == false && ids[i] >= 0) return helper;

			var item = readObjectHelper(creator, ids, i);
			helper.containedObjects.push(item);

			i++;
		}

		return helper;
	}

	public function toArray(useParentId = false):Array<Int> {
		return writeObjectHelper([], false, useParentId);
	}

	private function writeObjectHelper(ids:Array<Int>, isInSubcontainer:Bool = false, useParentId = false):Array<Int> {
		var first = (ids.length == 0);

		// trace('write: id:${this.objectData.id} ids:$ids isInSubcontainer: $isInSubcontainer');

		var objId = useParentId ? this.parentId : this.objectData.id;

		// negative values are used for subcontained items
		if (isInSubcontainer) {
			ids.push(objId * (-1));
			return ids;
		}

		ids.push(objId);

		for (item in containedObjects) {
			if (first) item.writeObjectHelper(ids, false, useParentId); else
				item.writeObjectHelper(ids, true, useParentId);
		}

		return ids;
	}

	public function toString():String {
		var objString = "";

		objString += '${this.objectData.id}';

		for (item in containedObjects) {
			objString += ',${item.objectData.id}';

			for (subitem in item.containedObjects) {
				objString += ':${subitem.objectData.id}';
			}
		}

		// trace('write obj to String: ${objString}');

		return objString;
	}

	public function new(creator:GlobalPlayerInstance, id:Int) {
		this.objectData = ObjectData.getObjectData(id);

		if (creator != null) {
			this.livingOwners.push(creator.p_id);
			this.ownersByPlayerAccount.push(creator.account.id);
		}

		this.creationTimeInTicks = TimeHelper.tick;
		this.numberOfUses = objectData.numUses;
	}

	public var parentObjData(get, null):ObjectData;

	public function get_parentObjData() {
		if (objectData.dummyParent != null) return objectData.dummyParent;

		return objectData;
	}

	/**
		gives back a non dummy id
	**/
	public var parentId(get, null):Int;

	public function get_parentId() {
		if (objectData.dummyParent != null) return objectData.dummyParent.id;

		return objectData.id;
	}

	public var id(get, set):Int; // TODO replace with parentId or dummyId

	public function get_id() {
		return objectData.id;
	}

	public function set_id(newID) {
		if (this.id == newID) return newID;

		var newObjectData = ObjectData.getObjectData(newID);
		if (newObjectData == null) throw new Exception('No ObjectData for: ${newID}');

		if (this.containedObjects.length > newObjectData.numSlots) {
			var obj = ObjectData.getObjectData(this.containedObjects[0].id);
			var message = 'WARNING: transform object: ${name} ${toArray()} --> ${newObjectData.name} slots: containedObjects.length > numSlots: ${newObjectData.numSlots} first contained: ${obj.name}';
			trace(message);
			throw new Exception(message);
		}

		this.objectData = newObjectData;
		this.timeToChange = ObjectHelper.CalculateTimeToChangeForObj(this);

		// TODO TransformToDummy();

		return newID;
	}

	public function dummyId():Int {
		if (objectData.dummyObjects.length <= 0 || numberOfUses == objectData.numUses) return objectData.id;

		return objectData.dummyObjects[numberOfUses - 1].id;
	}

	public function isPermanent():Bool {
		return objectData.isPermanent();
	}

	public function isNeverDrop() {
		if (objectData.neverDrop) return true;
		return StringTools.contains(objectData.description, '+neverDrop');
	}

	public var description(get, null):String;

	public function get_description() {
		return objectData.description;
	}

	public var name(get, null):String;

	public function get_name() {
		/*if(objectData.dummyParent == null) 
				trace('Name: ${objectData.id} ${objectData.name} ${objectData.description}');
			else
				trace('Name: ${objectData.id} ${objectData.name} --> ${objectData.dummyParent.name} ${objectData.dummyParent.description}');
		 */
		if (objectData.dummyParent != null) return objectData.dummyParent.name;
		return objectData.name;
	}

	// TODO make look like variable
	public function blocksWalking():Bool {
		return objectData.blocksWalking;
	}

	public function getCreatorId():Int {
		if (this.livingOwners.length < 1) return -1;
		return this.livingOwners[0];
	}

	public function getCreator():GlobalPlayerInstance {
		// GlobalPlayerInstance.AcquireMutex();
		var returnValue = GlobalPlayerInstance.AllPlayerMap[this.livingOwners[0]];
		// GlobalPlayerInstance.ReleaseMutex();
		return returnValue;
	}

	public function getLinage():Lineage {
		return Lineage.GetLineage(this.livingOwners[0]);
	}

	// returns removed object or null if there was none
	public function removeContainedObject(index:Int):ObjectHelper {
		if (index < 0) {
			return this.containedObjects.pop();
		}

		// TODO SEE if table switch objects can be fixed. Maybe add empty object in between, but never in the end

		var obj = this.containedObjects[index];
		this.containedObjects.remove(obj);

		return obj;
	}

	public static function CalculateTimeToChangeForObj(obj:ObjectHelper):Float {
		if (obj == null) return 0;

		var timeTransition = TransitionImporter.GetTransition(-1, obj.parentId, false, false);
		if (timeTransition == null) return 0;

		// trace('TIME: has time transition: ${transition.newTargetID} ${newTargetObjectData.description} time: ${timeTransition.autoDecaySeconds}');

		var timeToChange = timeTransition.calculateTimeToChange();

		if (obj.isAnimal() && obj.hits > 0.5) timeToChange /= 2;

		return timeToChange;
	}

	public function TransformToDummy() {
		var obj:ObjectHelper = this;
		var objectData = obj.objectData;
		if (objectData.dummyParent != null) objectData = objectData.dummyParent;

		// if it has not more uses then one, or can get more used by undo (like empty berry bush with a new berry), then there is nothing to do
		if (objectData.numUses < 2 && objectData.undoLastUseObject == 0) return;

		if (obj.numberOfUses < 1) {
			if (objectData.lastUseObject != 0) {
				objectData = ObjectData.getObjectData(objectData.lastUseObject);
				// trace('DUMMY LASTUSE:  ${objectData.description}');

				obj.objectData = objectData;
				obj.numberOfUses = 1;

				return;
			} else {
				var message = 'TransformToDummy: WARNING: ${objectData.description}: obj.numberOfUses < 1: ${obj.numberOfUses}';
				trace(message);

				// throw new Exception(message);

				obj.numberOfUses = 1;
			}
		}

		// in case of an maxUses object changing like a well site numOfUses can be too big
		if (obj.numberOfUses > objectData.numUses || (obj.numberOfUses > 1 && objectData.undoLastUseObject != 0)) {
			if (objectData.undoLastUseObject != 0) {
				objectData = ObjectData.getObjectData(objectData.undoLastUseObject);
				obj.numberOfUses = 1;

				// trace('DUMMY UNDO: ${objectData.description}');
			} else {
				obj.numberOfUses = objectData.numUses;
			}
		}

		if (obj.numberOfUses == objectData.numUses || objectData.undoLastUseObject != 0) {
			if (obj.objectData.dummy) {
				obj.objectData = obj.objectData.dummyParent;
			}
		} else {
			obj.objectData = objectData.dummyObjects[obj.numberOfUses - 1];
			if (obj.objectData == null) {
				trace('DUMMY UNDO: numberOfUses: ${obj.numberOfUses} ${objectData.description}');
				throw new Exception('TransformToDummy: no object Data!');
			}

			// trace('dummy id: ${obj.objectData.id}');
		}
	}

	public function isLastUse():Bool {
		return this.objectData.numUses > 1 && this.numberOfUses <= 1;
	}

	public function isHelperToBeDeleted():Bool {
		var helper = this;

		// Currently dummy objects for objects with more than numberOfUses get just the first unsed IDs
		// Therfore currently dummy object IDs change with each Data update that adds IDs
		// Therefore save object if number of uses is below original number of uses, so that dummy objects can be fixed are stored on data update
		var toDelete = (helper.numberOfUses == helper.objectData.numUses || helper.numberOfUses < 1 || helper.id < 1);
		// TODO maybe dont use a helper for time transitions?
		toDelete = toDelete && helper.timeToChange == 0 && helper.containedObjects.length == 0 && helper.groundObject == null;
		// toDelete = toDelete && helper.livingOwners.length < 1;
		toDelete = toDelete && helper.isOwned() == false && helper.isFollowerOwned() == false && helper.isGrave() == false;

		toDelete = toDelete && (helper.hits == 0 || helper.id < 1) && helper.coins <= 0 && externId == 0 && text == '';

		return toDelete;
	}

	public function isContainable():Bool {
		return this.objectData.containable;
	}

	public function isWound():Bool {
		if (StringTools.contains(description, 'Snake Bite')) return true;
		if (StringTools.contains(description, 'Hog Cut')) return true;
		return StringTools.contains(description, 'Wound');
	}

	public function isArrowWound():Bool {
		return StringTools.contains(description, 'Arrow Wound');
	}

	public function isDroppable():Bool {
		return this.id != 0 && this.isWound() == false;
	}

	public function isGrave():Bool {
		return objectData.isGrave();
	}

	public function isOwned():Bool {
		return StringTools.contains(description, '+owned');
	}

	public function hasOwners():Bool {
		return livingOwners.length > 0;
	}

	public function isFollowerOwned():Bool {
		return StringTools.contains(description, '+followerOwned');
	}

	public function isOwnedByPlayer(player:PlayerInterface):Bool {
		return isOwnedBy(player.getPlayerInstance().p_id);
	}

	public function isOwnedBy(playerId:Int):Bool {
		return livingOwners.contains(playerId);
	}

	public function setNewOwnerAndClearOld(player:GlobalPlayerInstance) {
		livingOwners = new Array<Int>();
		livingOwners.push(player.p_id);
		ownersByPlayerAccount = new Array<Int>();
		ownersByPlayerAccount.push(player.account.id);
	}

	public function addOwner(player:GlobalPlayerInstance) {
		if (isOwnedByPlayer(player)) return;

		livingOwners.push(player.p_id);

		if (ownersByPlayerAccount.contains(player.account.id)) return;
		ownersByPlayerAccount.push(player.account.id);
	}

	public function removeOwner(player:GlobalPlayerInstance) {
		livingOwners.remove(player.p_id);
		ownersByPlayerAccount.remove(player.account.id);
	}

	// is called from TransitionHelper
	public static function DoOwnerShip(obj:ObjectHelper, player:GlobalPlayerInstance) {
		if (obj.objectData.isOwned == false) return;

		obj.livingOwners = new Array<Int>(); // clear all former owners
		obj.ownersByPlayerAccount = new Array<Int>(); // clear all former owners
		obj.addOwner(player);

		player.owning.push(obj);
	}

	public function createOwnerString():String {
		var message = '';

		for (ownerId in livingOwners) {
			message += ' ${ownerId}';
		}

		return message;
	}

	public function getOwnerAccount() {
		if (this.ownersByPlayerAccount.length < 1) return null;
		return PlayerAccount.AllPlayerAccountsById[this.ownersByPlayerAccount[0]];
	}

	public function isBoneGrave():Bool {
		return this.objectData.isBoneGrave();
	}

	public function isGraveWithGraveStone():Bool {
		if (this.id == 1011) return false; // Buried Grave

		return isBoneGrave() == false;
	}

	public function timeUntillChange():Float {
		var passedTime = TimeHelper.CalculateTimeSinceTicksInSec(this.creationTimeInTicks);
		return this.timeToChange - passedTime;
	}

	public function isTimeToChangeReached():Bool {
		var passedTime = TimeHelper.CalculateTimeSinceTicksInSec(this.creationTimeInTicks);
		var timeToChange = this.timeToChange;

		return (passedTime >= timeToChange);
	}

	public function isKillableByBow():Bool {
		var trans = TransitionImporter.GetTransition(152, this.id); // Bow and Arrow
		return trans != null;
	}

	// TODO does not work for attacking wolf etc since they dont move...
	public function isAnimal():Bool {
		return objectData.isAnimal();
	}

	public function canMove():Bool {
		return objectData.canMove();
	}

	public function isFire():Bool {
		// 82 Fire // 82 Fire // 83 Large Fast Fire // 346 Large Slow Fire
		return objectData.parentId == 82 || objectData.parentId == 83 || objectData.parentId == 346;
	}

	public function canAddToQuiver():Bool {
		var quiver = this;
		return (quiver.objectData.numUses < 2 || quiver.numberOfUses < quiver.objectData.numUses);
	}

	public function isDomesticAnimal():Bool {
		return this.objectData.isDomesticAnimal();
	}

	public function isWall():Bool {
		return objectData.isWall();
	}

	public static function CalculateSurroundingWallStrength(tx:Int, ty:Int):Float {
		var world = WorldMap.world;

		var obj = world.getObjectDataAtPosition(tx, ty);
		var stength:Float = obj.isWall() ? 2 : 0;

		// Fence (obj.rValue < 0.1) is a Wall for auto align, but not for Wall Strength
		// TODO maybe make wall strength insulation dependend

		var obj = world.getObjectDataAtPosition(tx + 1, ty);
		stength += obj.isWall() && obj.rValue > 0.1 ? 2 : 0;
		var obj = world.getObjectDataAtPosition(tx - 1, ty);
		stength += obj.isWall() && obj.rValue > 0.1 ? 2 : 0;
		var obj = world.getObjectDataAtPosition(tx, ty + 1);
		stength += obj.isWall() && obj.rValue > 0.1 ? 2 : 0;
		var obj = world.getObjectDataAtPosition(tx, ty - 1);
		stength += obj.isWall() && obj.rValue > 0.1 ? 2 : 0;

		var obj = world.getObjectDataAtPosition(tx + 2, ty);
		stength += obj.isWall() && obj.rValue > 0.1 ? 2 : 0;
		var obj = world.getObjectDataAtPosition(tx - 2, ty);
		stength += obj.isWall() && obj.rValue > 0.1 ? 2 : 0;
		var obj = world.getObjectDataAtPosition(tx, ty + 2);
		stength += obj.isWall() && obj.rValue > 0.1 ? 2 : 0;
		var obj = world.getObjectDataAtPosition(tx, ty - 2);
		stength += obj.isWall() && obj.rValue > 0.1 ? 2 : 0;

		var obj = world.getObjectDataAtPosition(tx + 3, ty);
		stength += obj.isWall() && obj.rValue > 0.1 ? 1 : 0;
		var obj = world.getObjectDataAtPosition(tx - 3, ty);
		stength += obj.isWall() && obj.rValue > 0.1 ? 1 : 0;
		var obj = world.getObjectDataAtPosition(tx, ty + 3);
		stength += obj.isWall() && obj.rValue > 0.1 ? 1 : 0;
		var obj = world.getObjectDataAtPosition(tx, ty - 3);
		stength += obj.isWall() && obj.rValue > 0.1 ? 1 : 0;

		return stength;
	}

	public static function CalculateSurroundingFloorStrength(tx:Int, ty:Int):Float {
		var world = WorldMap.world;
		var objId = world.getFloorId(tx, ty);
		// trace('Decay ${obj.name} ${obj.floor}');
		var stength:Float = objId > 0 ? 1 : 0;

		// TODO give different floor strength? 0.1 for Road and Pine? 2 for Stone?

		var objId = world.getFloorId(tx + 1, ty);
		stength += objId > 0 ? 1 : 0;
		var objId = world.getFloorId(tx - 1, ty);
		stength += objId > 0 ? 1 : 0;
		var objId = world.getFloorId(tx, ty + 1);
		stength += objId > 0 ? 1 : 0;
		var objId = world.getFloorId(tx, ty - 1);
		stength += objId > 0 ? 1 : 0;

		return stength;
	}

	public function isBloody():Bool {
		return objectData.isBloody;
	}

	public function isDeadlyAnimal():Bool {
		return this.objectData.isDeadlyAnimal();
	}

	public function contains(searchContained:Array<Int>):Bool {
		var obj = this;
		for (item in obj.containedObjects) {
			if (searchContained.contains(item.parentId)) return true;
		}
		return false;
	}

	public function canBePlacedIn(container:ObjectHelper):Bool {
		if (this.objectData.containable == false) return false;
		if (container.containedObjects.length >= container.objectData.numSlots) return false;
		if (this.objectData.containSize > container.objectData.slotSize) return false;

		return true;
	}

	public function index() {
		return WorldMap.world.index(this.tx, this.ty);
	}
}
