package data.object.player;

class PlayerData
{
    /**
     * id -> PlayerType
     */
    public var key:Map<Int,PlayerInstance> = new Map<Int,PlayerInstance>();
    /**
     * array of player types
     */
    public var array:Array<PlayerInstance> = [];
    /**
     * main player id
     */
    public var primary:Int = -1;
    /**
     * update player data
     */
    public var update:Void->Void;
    public function new()
    {
        
    }
}