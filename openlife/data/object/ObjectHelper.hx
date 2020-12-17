package openlife.data.object;

import openlife.server.WorldMap;
import openlife.data.transition.TransitionData;
import openlife.server.GlobalPlayerInstance;
import openlife.server.Server;

class ObjectHelper {
    public var objectData:ObjectData; 
    public var numberOfUses = 0;
    public var creationTimeInTicks:Int;

    // needed for time Transitions
    public var timeToChange = 0; // in sec 
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

    public function writeObjectHelper(ids:Array<Int>, isInSubcontainer:Bool = false) : Array<Int>
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

    public function new(creator:GlobalPlayerInstance, id:Int)
    {
        this.objectData = ObjectData.getObjectData(id); 
        //if(this.objectData == null) this.objectData = Server.objectDataMap[0];
        this.livingOwners[0] = creator;

        this.creationTimeInTicks = Server.server.tick;
        this.numberOfUses = objectData.numUses;
    }

    // TODO make look like variable
    public function id() : Int
    {
        return objectData.id;
    }

    public function setId(newID:Int)
    {
        if(this.id() == newID) return;

        objectData = ObjectData.getObjectData(newID);
    }
    
    
    // TODO make look like variable
    public function description() : String
    {
        return objectData.description;
    }

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

    public static function calculateTimeToChangeForObj(obj:ObjectHelper) : Int
    {
        var timeTransition = Server.transitionImporter.getTransition(-1, obj.id(), false, false);
        if(timeTransition == null) return 0;

        return calculateTimeToChange(timeTransition);
    }

    public static function calculateTimeToChange(timeTransition:TransitionData) : Int
    {
        // hours are negative
        var timeToChange = timeTransition.autoDecaySeconds < 0 ?  (-3600) * timeTransition.autoDecaySeconds : timeTransition.autoDecaySeconds;                 
        timeToChange = Math.ceil((Server.server.map.randomInt(timeToChange * 2) + timeToChange)/2);

        return timeToChange;
    }
}