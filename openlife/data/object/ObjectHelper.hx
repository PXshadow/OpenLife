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

    /*
    public static function createNewObjectHelper(objectData:ObjectData){
        return new ObjectHelper(objectData);
    }
    */

    public function new(objectData:ObjectData, creator:GlobalPlayerInstance)
    {
        this.objectData = objectData;
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