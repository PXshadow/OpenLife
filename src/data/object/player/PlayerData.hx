package data.object.player;

class PlayerData
{
    /**
     * id -> PlayerType
     */
    public var key:Map<Int,PlayerType> = new Map<Int,PlayerType>();
    /**
     * array of player types
     */
    public var array:Array<PlayerType> = [];
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