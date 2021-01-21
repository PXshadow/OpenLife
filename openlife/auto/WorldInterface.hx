package openlife.auto;

import openlife.data.transition.TransitionData;
import openlife.data.object.ObjectHelper;

interface WorldInterface
{
    public function getTrans(actor:ObjectHelper, target:ObjectHelper) : TransitionData;
    public function getTransition(actorId:Int, targetId:Int, lastUseActor:Bool = false, lastUseTarget:Bool = false, maxUseTarget:Bool=false) : TransitionData;

    public function getBiomeId(x:Int, y:Int):Int;           // since map is known no need for out of range
    public function isBiomeBlocking(x:Int, y:Int) : Bool;   // since map is known no need for out of range
    //** returns NULL of x,y is too far away from player **/
    public function getObjectId(x:Int, y:Int):Array<Int>; 
    //** returns NULL of x,y is too far away from player / allowNull means it wont create a object helper if there is none **/
    public function getObjectHelper(x:Int, y:Int, allowNull:Bool = false) : ObjectHelper;
    //** returns -1 of x,y is too far away from player **/
    public function getFloorId(x:Int, y:Int):Int; 

    

    /*
    getNearestPlayer()
    getPlayer(x,y)
    */
}