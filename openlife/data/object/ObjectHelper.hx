package openlife.data.object;

import openlife.server.PlayerAccount;
import openlife.settings.ServerSettings;
import sys.io.File;
import openlife.auto.PlayerInterface;
import openlife.server.Lineage;
import openlife.data.transition.TransitionImporter;
import haxe.macro.Type.TVar;
import haxe.Exception;
import openlife.server.TimeHelper;
import openlife.server.WorldMap;
import openlife.data.transition.TransitionData;
import openlife.server.GlobalPlayerInstance;
import openlife.server.Server;
import haxe.ds.Vector;

class ObjectHelper
{
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

    public var hits:Float = 0; // not saved 

    public static function WriteMapObjHelpers(path:String, objHelpersToWrite:Vector<ObjectHelper>)
    { 
        var width = WorldMap.world.width;
        var height = WorldMap.world.height;
        var length = WorldMap.world.length;

        //trace('Wrtie to file: $path width: $width height: $height length: $length');

        if(width * height != length) throw new Exception('width * height != length');
        if(objHelpersToWrite.length != length) throw new Exception('objHelpersToWrite.length != length');

        var count = 0;
        var dataVersion = 4;

        var writer = File.write(path, true);
        writer.writeInt32(dataVersion);        
        writer.writeInt32(width);
        writer.writeInt32(height);        

        for(obj in objHelpersToWrite)
        {
            if(obj == null) continue;

            count++;

            WorldMap.WriteInt32Array(writer, obj.toArray());
            WorldMap.WriteInt32Array(writer, obj.livingOwners);
            WorldMap.WriteInt32Array(writer, obj.ownersByPlayerAccount);

            writer.writeInt32(obj.tx);
            writer.writeInt32(obj.ty);
            writer.writeInt32(obj.numberOfUses);
            writer.writeDouble(obj.creationTimeInTicks);
            writer.writeFloat(obj.timeToChange);
        }

        writer.writeInt8(100); // end sign

        writer.close();

        if(ServerSettings.DebugWrite) trace('wrote $count ObjectHelpers...');
    }

    public static function ReadMapObjHelpers(path:String) : Vector<ObjectHelper>
    {
        var reader = File.read(path, true);
        var expectedDataVersion = 4;
        var dataVersion = reader.readInt32();
        var width = reader.readInt32();
        var height = reader.readInt32();
        var length = width * height;
        var count = 0;
        var newObjects = new Vector<ObjectHelper>(length);
        var world = WorldMap.world;

        world.objectHelpers = newObjects;

        if(dataVersion != expectedDataVersion) throw new Exception('ReadMapObjHelpers: Data version is: $dataVersion expected data version is: $expectedDataVersion');
        if(width != world.width) throw new Exception('width != this.width');
        if(height != world.height) throw new Exception('height != this.height');
        if(length != world.length) throw new Exception('length != this.length');

        trace('Read from file: $path width: $width height: $height length: $length');

        try{
            while(reader.eof() == false)
            {
                var array = WorldMap.ReadInt32Array(reader);
                if(array == null) break; // reached the end
                count++;

                var newObject = ObjectHelper.readObjectHelper(null, array);
                newObject.livingOwners = WorldMap.ReadInt32Array(reader);
                newObject.ownersByPlayerAccount = WorldMap.ReadInt32Array(reader);
                newObject.tx = reader.readInt32();
                newObject.ty = reader.readInt32();
                newObject.numberOfUses = reader.readInt32();
                newObject.creationTimeInTicks = reader.readDouble();
                newObject.timeToChange = reader.readFloat();

                if(newObject.creationTimeInTicks > TimeHelper.tick) newObject.creationTimeInTicks = TimeHelper.tick;

                if(newObject.numberOfUses > 1 || newObject.containedObjects.length > 0)
                {
                    // 1435 = bison // 1261 = Canada Goose Pond with Egg // 30 = Gooseberry Bush // 2142 = Banana Plant // 1323 = Wild Boar
                    if(newObject.id != 1435 && newObject.id != 1261  && newObject.id != 30 && newObject.id != 2142 && newObject.id != 1323)
                    {
                        // trace('${newObject.description()} numberOfUses: ${newObject.numberOfUses} from  ${newObject.objectData.numUses} ' + newObjArray);
                    }
                }

                world.setObjectHelper(newObject.tx, newObject.ty, newObject);
                //newObjects[index(newObject.tx, newObject.ty)] = newObject;
                //objects[index(newObject.tx, newObject.ty)] = newObjArray;
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

    public static function InitObjectHelpersAfterRead()
    {
        for(obj in WorldMap.world.objectHelpers)
        {
            if(obj == null) continue;

            if(obj.isGrave())
            {
                for(id in obj.ownersByPlayerAccount)
                {
                    var account = PlayerAccount.GetPlayerAccountById(id);
                    if(account == null) continue;

                    account.graves.push(obj);
                }
            } else if(obj.isOwned()) 
            {
                // TODO Will only work once players are saved
                for(id in obj.livingOwners) 
                {
                    var player = GlobalPlayerInstance.AllPlayers[id];
                    if(player == null)
                    {
                        obj.livingOwners.remove(id);
                        continue; // TODO warning                        
                    }
                    if(player.deleted) obj.removeOwner(player);

                    player.owning.push(obj);
                }
            }
        }
    }
    
    public static function readObjectHelper(creator:GlobalPlayerInstance, ids:Array<Int>, i:Int = 0) : ObjectHelper
    {
        var id = ids[i];
        var isFirst = (i == 0);
        var isInSubcontainer = false;

        //trace('read: id:$id i:$i ids:$ids isInSubcontainer: $isInSubcontainer');

        // negative values are used for subcontained items
        if(id < 0){
            isInSubcontainer = true;
            id *= -1;
        }

        var helper = new ObjectHelper(creator, id);

        if(isInSubcontainer) return helper;

        i++;

        // read container items
        while(i < ids.length)
        {
            // negative values are used only for subcontained items so skip them
            if(isFirst && ids[i] < 0) {
                i++;
                continue;
            }
            
            // in subcontainer contained items must be negative, so return if there is no negative item
            if(isFirst == false && ids[i] >= 0) return helper;

            var item = readObjectHelper(creator, ids, i);
            helper.containedObjects.push(item);

            i++;
        }

        return helper;
    }

    public function toArray() : Array<Int>
    {
        return writeObjectHelper([]);
    }

    private function writeObjectHelper(ids:Array<Int>, isInSubcontainer:Bool = false) : Array<Int>
    {
        var first = (ids.length == 0);

        //trace('write: id:${this.objectData.id} ids:$ids isInSubcontainer: $isInSubcontainer');

        // negative values are used for subcontained items
        if(isInSubcontainer){
            ids.push(this.objectData.id * (-1));
            return ids;
        }
        
        ids.push(this.objectData.id);

        for(item in containedObjects){
            if(first) item.writeObjectHelper(ids);
            else item.writeObjectHelper(ids, true);
        }

        return ids;
    }

    public function toString() : String
    {
        var objString = "";

        objString += '${this.objectData.id}';

        for(item in containedObjects)
        {
            objString += ',${item.objectData.id}';

            for(subitem in item.containedObjects)
            {
                objString += ':${subitem.objectData.id}';
            }
        }

        //trace('write obj to String: ${objString}');

        return objString;
    }

    public function new(creator:GlobalPlayerInstance, id:Int)
    {
        this.objectData = ObjectData.getObjectData(id); 

        if(creator != null)
        {
            this.livingOwners.push(creator.p_id); 
            this.ownersByPlayerAccount.push(creator.account.id); 
        }

        this.creationTimeInTicks = TimeHelper.tick;
        this.numberOfUses = objectData.numUses;
    }

    /**
        gives back a non dummy id
    **/
    public var parentId(get, null):Int; 

    public function get_parentId()
    {
        if(objectData.dummyParent != null) return objectData.dummyParent.id;

        return objectData.id;
    }


    public var id(get, set):Int; // TODO replace with parentId or dummyId

    public function get_id()
    {
        return objectData.id;
    }

    public function set_id(newID)
    {
        if(this.id == newID) return newID;    

        this.objectData = ObjectData.getObjectData(newID);

        if(this.objectData == null) throw new Exception('No ObjectData for: ${newID}');

        return newID;
    }

    public function dummyId() : Int
    {
        if(objectData.dummyObjects.length <= 0 || numberOfUses == objectData.numUses) return objectData.id;

        return objectData.dummyObjects[numberOfUses-1].id;
    }

    public function isPermanent()
    {
        return objectData.permanent == 1;
    }

    public function isNeverDrop()
    {
        if(objectData.neverDrop) return true;
        return StringTools.contains(objectData.description,'+neverDrop');
    }

    public var description(get, null):String;

    public function get_description()
    {
        return objectData.description;
    }

    // TODO make look like variable
    public function blocksWalking() : Bool
    {
        return objectData.blocksWalking;
    }

    public function getCreatorId() : Int
    {
        if(this.livingOwners.length < 1) return -1;
        return this.livingOwners[0];
    }

    public function getCreator() : GlobalPlayerInstance
    {
        return GlobalPlayerInstance.AllPlayers[this.livingOwners[0]];
    }

    public function getLinage() : Lineage
    {
        return Lineage.GetLineage(this.livingOwners[0]);
    }

    // returns removed object or null if there was none
    public function removeContainedObject(index:Int) : ObjectHelper
    {
        if(index < 0){
            return this.containedObjects.pop();
        }

        // TODO SEE if table switch objects can be fixed. Maybe add empty object in between, but never in the end

        var obj = this.containedObjects[index];
        this.containedObjects.remove(obj);

        return obj;
    }

    public static function CalculateTimeToChangeForObj(obj:ObjectHelper) : Float
    {
        if(obj == null) return 0;
        
        var timeTransition = TransitionImporter.GetTransition(-1, obj.parentId, false, false);
        if(timeTransition == null) return 0;

        //trace('TIME: has time transition: ${transition.newTargetID} ${newTargetObjectData.description} time: ${timeTransition.autoDecaySeconds}');

        return CalculateTimeToChange(timeTransition);
    }

    public static function CalculateTimeToChange(timeTransition:TransitionData) : Float
    {
        // hours are negative
        var timeToChange = timeTransition.autoDecaySeconds < 0 ?  (-3600) * timeTransition.autoDecaySeconds : timeTransition.autoDecaySeconds;                 
        timeToChange = WorldMap.calculateRandomFloat() * timeToChange + timeToChange / 2;

        // if(timeTransition.targetID == 2992) trace('TIME33:  ${timeTransition.targetID} ${timeToChange}');

        return timeToChange;
    }

    public function TransformToDummy()
    {
        var obj:ObjectHelper = this;
        var objectData  = obj.objectData;
        if(objectData.dummyParent != null) objectData = objectData.dummyParent;        

        // if it has not more uses then one, or can get more used by undo (like empty berry bush with a new berry), then there is nothing to do
        if(objectData.numUses < 2 && objectData.undoLastUseObject == 0) return;

        if(obj.numberOfUses < 1)
        {
            if(objectData.lastUseObject != 0)
            {
                objectData = ObjectData.getObjectData(objectData.lastUseObject);
                //trace('DUMMY LASTUSE:  ${objectData.description}');

                obj.objectData = objectData;
                obj.numberOfUses = 1;

                return;
            }
            else
            {
                var message = 'TransformToDummy: WARNING: ${objectData.description}: obj.numberOfUses < 1: ${obj.numberOfUses}';
                trace(message);

                //throw new Exception(message);

                obj.numberOfUses = 1;
            }
        }

        // in case of an maxUses object changing like a well site numOfUses can be too big
        if(obj.numberOfUses > objectData.numUses || (obj.numberOfUses > 1 && objectData.undoLastUseObject != 0))
        {
            if(objectData.undoLastUseObject != 0)
            {
                objectData = ObjectData.getObjectData(objectData.undoLastUseObject);
                obj.numberOfUses = 1;

                //trace('DUMMY UNDO: ${objectData.description}');
            }
            else
            {
                obj.numberOfUses = objectData.numUses;
            }
        }

        if(obj.numberOfUses == objectData.numUses || objectData.undoLastUseObject != 0)
        {
            if(obj.objectData.dummy)
            {
                obj.objectData = obj.objectData.dummyParent;
            }
        }
        else
        {
            obj.objectData = objectData.dummyObjects[obj.numberOfUses-1];
            if(obj.objectData == null)
            {
                trace('DUMMY UNDO: numberOfUses: ${obj.numberOfUses} ${objectData.description}');
                throw new Exception('TransformToDummy: no object Data!');
            }

            //trace('dummy id: ${obj.objectData.id}');
        }
    }

    public function isLastUse() : Bool
    {
        return this.objectData.numUses > 1 && this.numberOfUses <= 1;
    }

    public function isHelperToBeDeleted() : Bool
    {
        var helper = this;
        // TODO why not use dummy instead? So no need for helper?
        var toDelete = (helper.numberOfUses == helper.objectData.numUses || helper.numberOfUses < 1);
        // TODO maybe dont use a helper for time transitions?
        toDelete = toDelete && helper.timeToChange == 0 && helper.containedObjects.length == 0 && helper.groundObject == null;
        //toDelete = toDelete && helper.livingOwners.length < 1;
        toDelete = toDelete && helper.isOwned() == false && helper.isFollowerOwned() == false && helper.isGrave() == false;

        return toDelete;
    }

    public function isContainable() : Bool
    {
        return this.objectData.containable;
    }
    
    public function isWound() : Bool
    {
        if(StringTools.contains(description, 'Snake Bite')) return true;
        if(StringTools.contains(description, 'Hog Cut')) return true;
        return StringTools.contains(description, 'Wound');
    }

    public function isArrowWound() : Bool
    {
        return StringTools.contains(description, 'Arrow Wound');
    }

    public function isDroppable() : Bool
    {
        return this.id != 0 && this.isWound() == false;
    }
    
    public function isGrave() : Bool
    {
        return StringTools.contains(description, 'origGrave');
    }

    public function isOwned() : Bool
    {
        return StringTools.contains(description, '+owned');
    }

    public function hasOwners() : Bool
    {
        return livingOwners.length > 0;
    }

    public function isFollowerOwned() : Bool
    {
        return StringTools.contains(description, '+followerOwned');
    }

    public function isOwnedByPlayer(player:PlayerInterface) : Bool
    {
        return isOwnedBy(player.getPlayerInstance().p_id);
    }

    public function isOwnedBy(playerId:Int) : Bool
    {
        return livingOwners.contains(playerId);
    }

    public function addOwner(player:GlobalPlayerInstance)
    {
        if(isOwnedByPlayer(player)) return;

        livingOwners.push(player.p_id);

        if(ownersByPlayerAccount.contains(player.account.id)) return;
        ownersByPlayerAccount.push(player.account.id);
    }

    public function removeOwner(player:GlobalPlayerInstance)
    {
        livingOwners.remove(player.p_id);
        ownersByPlayerAccount.remove(player.account.id);
    }

    // is called from TransitionHelper
    public static function DoOwnerShip(obj:ObjectHelper, player:GlobalPlayerInstance)
    {
        if(obj.objectData.isOwned == false) return;

        obj.livingOwners = new Array<Int>(); // clear all former owners
        obj.ownersByPlayerAccount = new Array<Int>(); // clear all former owners
        obj.addOwner(player);

        player.owning.push(obj);
    }

    public function createOwnerString() : String
    {
        var message = '';

        for(ownerId in livingOwners)
        {
            message += ' ${ownerId}';
        }

        return message;
    }    

    public function getOwnerAccount()
    {
        return PlayerAccount.AllPlayerAccountsById[this.ownersByPlayerAccount[0]];
    }

    public function isBoneGrave() : Bool
    {
        var grave:ObjectHelper = this;
        var objData = grave.objectData;

        if(objData.id == 87) return true; // Fresh Grave
        if(objData.id == 88) return true; // Grave
        if(objData.id == 89) return true; // Old Grave
        if(objData.id == 356) return true; // Basket of Bones
        if(objData.id == 357) return true; // Bone Pile

        if(objData.id == 1920) return true; // Baby Bones
        if(objData.id == 3051) return true; // Baby Bone Pile
        if(objData.id == 3052) return true; // Basket of Baby Bones

        if(objData.id == 3195) return true; // Defaced Bone Pile
        if(objData.id == 3196) return true; // Basket of Defaced Bones

        if(objData.id == 752) return true; // Murder Grave
        if(objData.id == 1011) return true; // Buried Grave

        return false;
    }

    public function isGraveWithGraveStone() : Bool
    {
        if(this.id == 1011) return false; //Buried Grave

        return isBoneGrave() == false;
    }

    public function isTimeToChangeReached() : Bool
    {
        var passedTime = TimeHelper.CalculateTimeSinceTicksInSec(this.creationTimeInTicks);
        var timeToChange = this.timeToChange;

        return (passedTime >= timeToChange);
    }
}