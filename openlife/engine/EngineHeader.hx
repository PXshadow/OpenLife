package openlife.engine;
import openlife.data.map.MapInstance;
import openlife.data.object.player.PlayerInstance;
import openlife.data.object.player.PlayerMove;
import openlife.data.map.MapChange;
//#if nativeGen @:nativeGen #end
interface EngineHeader
{
    //Headers not included:
    //COMPRESSED_MESSAGE

    public function playerUpdate(instances:Array<PlayerInstance>):Void; //PLAYER_UPDATE
    public function playerMoveStart(move:PlayerMove):Void; //PLAYER_MOVES_START

    public function playerOutOfRange(list:Array<Int>):Void; //PLAYER_OUT_OF_RANGE
    public function playerName(id:Int,firstName:String,lastName:String):Void; //NAME

    public function apocalypse():Void; //APOCALYPSE
    public function apocalypseDone():Void; //APOCALYPSE_DONE

    public function posse(killer:Int,target:Int):Void; //POSSE_JOIN

    public function following(follower:Int,leader:Int,color:Int):Void; //FOLLOWING
    public function exiled(target:Int,id:Int):Void; //EXILED
    public function cursed(id:Int,level:Int,word:String):Void; //CURSED
    public function curseToken(count:Int):Void; //CURSE_TOKEN_CHANGE
    public function curseScore(excess:Int):Void; //CURSE_SCORE_CHANGE

    public function badBiomes(id:Int,name:String):Void; //BAD_BIOMES

    public function vogUpdate():Void; //VOG_UPDATE
    public function photo(x:Int,y:Int,signature:String):Void; //PHOTO_SIGNATURE

    public function shutdown():Void; //FORCED_SHUTDOWN

    public function global(text:String):Void; //GLOBAL_MESSAGE
    public function war(a:Int,b:Int,status:String):Void; //WAR_REPORT

    public function learnedTools(list:Array<Int>):Void; //LEARNED_TOOL_REPORT
    public function toolExperts(list:Array<Int>):Void; //TOOL_EXPERTS
    public function toolSlots(total:Int):Void; //TOOL_SLOTS
    
    public function babyWiggle(list:Array<Int>):Void; //BABY_WIGGLE
    public function saysLocation(x:Int,y:Int,text:String):Void; //LOCATION_SAYS
    public function dying(id:Int,sick:Bool):Void; //DYING
    public function says(id:Int,text:String,curse:Bool):Void; //PLAYER_SAYS
    public function emot(id:Int,index:Int,sec:Int):Void; //PLAYER_EMOT
    
    public function mapChunk(instance:MapInstance):Void; //MAP_CHUNK
    public function mapChange(change:MapChange):Void; //MAP_CHANGE
    public function foodChange(store:Int,capacity:Int,ateId:Int,fillMax:Int,speed:Float,responsible:Int):Void; //FOOD_CHANGE
    public function heatChange(heat:Float,foodTime:Float,indoorBonus:Float):Void; //HEAT_CHANGE
    public function frame():Void; //FRAME
    public function lineage(list:Array<String>):Void; //LINEAGE
    public function healed(id:Int):Void; //HEALED
    public function monument(x:Int,y:Int,id:Int):Void; //MONUMENT_CALL
    public function grave(x:Int,y:Int,id:Int):Void; //GRAVE
    public function graveOld(x:Int,y:Int,pid:Int,poid:Int,age:Float,name:String,lineage:Array<String>):Void; //GRAVE_OLD
    public function graveMove(xs:Int,ys:Int,xd:Int,yd:Int,swapDest:Bool):Void; //GRAVE_MOVE
    public function ownerList(x:Int,y:Int,list:Array<Int>):Void; //OWNER_LIST
    public function valley(spacing:Int,offset:Int):Void; //VALLEY_SPACING
    public function flight(id:Int,x:Int,y:Int):Void; //FLIGHT_DEST
    public function homeland(x:Int,y:Int,name:String):Void; //HOMELAND
    public function craving(id:Int,bonus:Int):Void; //CRAVING
    public function flip(x:Int,y:Int):Void; //FLIP
}