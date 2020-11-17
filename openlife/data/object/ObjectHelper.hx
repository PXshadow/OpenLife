package openlife.data.object;

import openlife.server.Server;

class ObjectHelper {
    public var objectData:ObjectData; 
    public var numberOfUses = 0;
    public var creationTimeInTicks:Int;

    // to store contained objects in case object is a container
    public var containedObjects:Array<ObjectHelper> = [];

    /*
    public static function createNewObjectHelper(objectData:ObjectData){
        return new ObjectHelper(objectData);
    }
    */

    public function new(objectData:ObjectData)
    {
        this.objectData = objectData;
        this.creationTimeInTicks = Server.server.tick;
        this.numberOfUses = objectData.numUses;
        //objectData.numSlots > 0;
        

        //public var useChance:Float = 0;
    }

}