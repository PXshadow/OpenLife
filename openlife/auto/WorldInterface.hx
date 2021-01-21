package openlife.auto;

interface WorldInterface
{
    public function getBiomeId(x:Int, y:Int):Int;           // since map is known no need for out of range
    public function isBiomeBlocking(x:Int, y:Int) : Bool;   // since map is known no need for out of range
    //** returns NULL of x,y is too far away from player **/
    public function getObjectId(x:Int, y:Int):Array<Int>; 
    //** returns -1 of x,y is too far away from player **/
    public function getFloorId(x:Int, y:Int):Int; 

    /*
    getNearestPlayer()
    getPlayer(x,y)
    */
}