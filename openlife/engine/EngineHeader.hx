package openlife.engine;
import openlife.data.map.MapInstance;
import openlife.data.object.player.PlayerInstance;
import openlife.data.object.player.PlayerMove;
import openlife.data.map.MapChange;
//#if nativeGen @:nativeGen #end
class EngineHeader
{
    //Headers not included:
    //COMPRESSED_MESSAGE

    public function playerUpdate(instances:Array<PlayerInstance>) {}; //PLAYER_UPDATE
    public function playerMoveStart(move:PlayerMove) {}; //PLAYER_MOVES_START
    /**
     * [Description]
     * @param list player ids
     */
    public function playerOutOfRange(list:Array<Int>) {}; //PLAYER_OUT_OF_RANGE
    public function playerName(id:Int,firstName:String,lastName:String) {}; //NAME

    public function apocalypse() {}; //APOCALYPSE
    public function apocalypseDone() {}; //APOCALYPSE_DONE

    public function posse(killer:Int,target:Int) {}; //POSSE_JOIN

    public function following(follower:Int,leader:Int,color:Int) {}; //FOLLOWING
    public function exiled(target:Int,id:Int) {}; //EXILED
    public function cursed(id:Int,level:Int,word:String) {}; //CURSED
    public function curseToken(count:Int) {}; //CURSE_TOKEN_CHANGE
    public function curseScore(excess:Int) {}; //CURSE_SCORE_CHANGE

    public function badBiomes(id:Int,name:String) {}; //BAD_BIOMES

    public function vogUpdate() {}; //VOG_UPDATE
    public function photo(x:Int,y:Int,signature:String) {}; //PHOTO_SIGNATURE

    public function shutdown() {}; //FORCED_SHUTDOWN

    public function global(text:String) {}; //GLOBAL_MESSAGE
    public function war(a:Int,b:Int,status:String) {}; //WAR_REPORT

    public function learnedTools(list:Array<Int>) {}; //LEARNED_TOOL_REPORT
    public function toolExperts(list:Array<Int>) {}; //TOOL_EXPERTS
    public function toolSlots(total:Int) {}; //TOOL_SLOTS
    /**
     * [Description]
     * @param list player ids
     */
    public function babyWiggle(list:Array<Int>) {}; //BABY_WIGGLE
    public function saysLocation(x:Int,y:Int,text:String) {}; //LOCATION_SAYS
    public function dying(id:Int,sick:Bool) {}; //DYING
    public function says(id:Int,text:String,curse:Bool) {}; //PLAYER_SAYS
    public function emot(id:Int,index:Int,sec:Int) {}; //PLAYER_EMOT
    
    public function mapChunk(instance:MapInstance) {}; //MAP_CHUNK
    public function mapChange(change:MapChange) {}; //MAP_CHANGE
    public function foodChange(store:Int,capacity:Int,ateId:Int,fillMax:Int,speed:Float,responsible:Int) {}; //FOOD_CHANGE
    public function heatChange(heat:Float,foodTime:Float,indoorBonus:Float) {}; //HEAT_CHANGE
    public function frame() {}; //FRAME
    public function lineage(list:Array<String>) {}; //LINEAGE
    public function healed(id:Int) {}; //HEALED
    public function monument(x:Int,y:Int,id:Int) {}; //MONUMENT_CALL
    public function grave(x:Int,y:Int,id:Int) {}; //GRAVE
    public function graveOld(x:Int,y:Int,pid:Int,poid:Int,age:Float,name:String,lineage:Array<String>) {}; //GRAVE_OLD
    public function graveMove(xs:Int,ys:Int,xd:Int,yd:Int,swapDest:Bool) {}; //GRAVE_MOVE
    public function ownerList(x:Int,y:Int,list:Array<Int>) {}; //OWNER_LIST
    public function valley(spacing:Int,offset:Int) {}; //VALLEY_SPACING
    public function flight(id:Int,x:Int,y:Int) {}; //FLIGHT_DEST
    public function homeland(x:Int,y:Int,name:String) {}; //HOMELAND
}