package openlife.data.object;

import openlife.data.transition.TransitionImporter;
import haxe.macro.Type.TVar;
import haxe.Exception;
import openlife.server.TimeHelper;
import openlife.server.WorldMap;
import openlife.data.transition.TransitionData;
import openlife.server.GlobalPlayerInstance;
import openlife.server.Server;

class ObjectHelper {
    public var objectData:ObjectData; 
    public var numberOfUses = 0;
    public var creationTimeInTicks:Float;

    /**Time to next change in seconds / needed for time Transitions**/
    public var timeToChange = 0; 
    public var tx:Int = 0;
    public var ty:Int = 0;

    // public var preferredBiome:Int; // used for movement
    // needed to store ground object in case something moves on top
    public var groundObject:ObjectHelper;

    // first one is always creator
    public var livingOwners:Array<GlobalPlayerInstance> = [];

    // to store contained objects in case object is a container
    public var containedObjects:Array<ObjectHelper> = [];
    
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

        this.livingOwners[0] = creator;

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
    
    public var description(get, null):String;

    public function get_description()
    {
        return objectData.description;
    }

    
    // TODO make look like variable
    //public function description() : String
    //{
    //    return objectData.description;
    //}

    // TODO make look like variable
    public function blocksWalking() : Bool
    {
        return objectData.blocksWalking;
    }

    public function getCreator() : GlobalPlayerInstance
    {
        return this.livingOwners[0];
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

    public static function CalculateTimeToChangeForObj(obj:ObjectHelper) : Int
    {
        var timeTransition = TransitionImporter.GetTransition(-1, obj.id, false, false);
        if(timeTransition == null) return 0;

        //trace('TIME: has time transition: ${transition.newTargetID} ${newTargetObjectData.description} time: ${timeTransition.autoDecaySeconds}');

        return CalculateTimeToChange(timeTransition);
    }

    public static function CalculateTimeToChange(timeTransition:TransitionData) : Int
    {
        // hours are negative
        var timeToChange = timeTransition.autoDecaySeconds < 0 ?  (-3600) * timeTransition.autoDecaySeconds : timeTransition.autoDecaySeconds;                 
        timeToChange = Math.ceil((WorldMap.calculateRandomInt(timeToChange * 2) + timeToChange)/2);

        return timeToChange;
    }

    public function TransformToDummy()
    {
        var obj:ObjectHelper = this;
        var objectData  = obj.objectData;
        if(objectData.dummyParent != null) objectData = objectData.dummyParent;        

        if(objectData.numUses < 2) return;

        if(obj.numberOfUses < 1)
        {
            var message = 'TransformToDummy: WARNING: ${objectData.description}: obj.numberOfUses < 1: ${obj.numberOfUses}';
            trace(message);

            //throw new Exception(message);

            obj.numberOfUses = 1;
        }

        // in case of an maxUses object changing like a well site numOfUses can be too big
        if(obj.numberOfUses > objectData.numUses)
        {
            obj.numberOfUses = objectData.numUses;
        }

        if(obj.numberOfUses == objectData.numUses)
        {
            if(obj.objectData.dummy)
            {
                obj.objectData = obj.objectData.dummyParent;
            }
        }
        else
        {
            obj.objectData = objectData.dummyObjects[obj.numberOfUses-1];
            if(obj.objectData == null) throw new Exception('TransformToDummy: no object Data!');

            //trace('dummy id: ${obj.objectData.id}');
        }
    }

    public function isLastUse() : Bool
    {
        return this.objectData.numUses > 1 && this.numberOfUses <= 1;
    }
}