package openlife.data.object;

import openlife.server.GlobalPlayerInstance;
import openlife.server.Server;

class ObjectHelper {
    public var objectData:ObjectData; 
    public var numberOfUses = 0;
    public var creationTimeInTicks:Int;

    // first one is always creator
    public var livingOwners:Array<GlobalPlayerInstance> = [];

    // to store contained objects in case object is a container
    public var containedObjects:Array<ObjectHelper> = [];
    
    public static function readObjectHelper(creator:GlobalPlayerInstance, ids:Array<Int>, i:Int = 0) : ObjectHelper
    {

        var id = ids[i];
        var isFirst = (i == 0);
        var isInSubcontainer = false;

        // negative values are used for subcontained items
        if(id < 0){
            isInSubcontainer = true;
            id *= -1;
        }

        var helper = new ObjectHelper(creator, id);

        if(isInSubcontainer) return helper;

        i++;

        // read container items
        while(i < ids.length){
            if(isFirst)
            {
                // negative values are used for subcontained items
                if(ids[i] < 0) continue;
            }
            else
            {
                // in subcontainer contained items must be negative, so return if there is no negative item
                if(ids[i] >= 0) return helper;
            }

            var item = readObjectHelper(creator, ids, i);
            helper.containedObjects.push(item);
            i++;
        }

        return helper;

        // Or iterativ?
        /*
        var first = null;
        var container = null;
        var isInSubcontainer = false;
        var helper = null;

        for(id in ids){
            // negative values are used for subcontained items
            if(id < 0){
                if(isInSubcontainer == false) {
                    container = helper;        
                }

                isInSubcontainer = true;
                id *= -1;
                
            }
            // if it is not in subcontainer, it must be in first object
            else
            {
                isInSubcontainer = false;
                container = first;
            }

            var objectData = Server.objectDataMap[id];
            helper = new ObjectHelper(creator, objectData);

            // the object is either the first one, or its in a container
            if(first == null)
            {
                first = helper;
                
            } else
            {
                container.containedObjects.push(helper);
            }
        }

        return first;
        */
    }

    public function writeObjectHelper(ids:Array<Int>, isInSubcontainer:Bool = false) : Array<Int>
    {
        var first = (ids.length > 0);
        //var isInSubcontainer = 

        // negative values are used for subcontained items
        if(isInSubcontainer){
            ids.push(this.objectData.id * (-1));
            return ids;
        }
        
        ids.push(this.objectData.id);

        for(item in containedObjects){
            if(first) writeObjectHelper(ids);
            else writeObjectHelper(ids, true);
        }

        return ids;
    }

    public function new(creator:GlobalPlayerInstance, id:Int)
    {
        this.objectData = Server.objectDataMap[id];
        this.livingOwners[0] = creator;

        this.creationTimeInTicks = Server.server.tick;
        this.numberOfUses = objectData.numUses;

        
        //objectData.numSlots > 0;
        //public var useChance:Float = 0;
    }

    public function getCreator() : GlobalPlayerInstance
    {
        return this.livingOwners[0];
    }

}