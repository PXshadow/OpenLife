package openlife.engine;
import openlife.data.map.MapInstance;
import openlife.data.object.player.PlayerInstance;
import openlife.data.object.player.PlayerMove;
import openlife.data.map.MapChange;
@:expose
class EngineEvent
{
    public static function create():EngineEvent
    {
        return new EngineEvent();
    }
    public var playerUpdate:(instances:Array<PlayerInstance>)->Void; //PLAYER_UPDATE
    public var playerMoveStart:(move:PlayerMove)->Void; //PLAYER_MOVES_START

    public var playerOutOfRange:(list:Array<Int>)->Void; //PLAYER_OUT_OF_RANGE
    public var playerName:(id:Int,firstName:String,lastName:String)->Void; //NAME

    public var apocalypse:()->Void; //APOCALYPSE
    public var apocalypseDone:()->Void; //APOCALYPSE_DONE

    public var posse:(killer:Int,target:Int)->Void; //POSSE_JOIN

    public var following:(follower:Int,leader:Int,color:Int)->Void; //FOLLOWING
    public var exiled:(target:Int,id:Int)->Void; //EXILED
    public var cursed:(id:Int,level:Int,word:String)->Void; //CURSED
    public var curseToken:(count:Int)->Void; //CURSE_TOKEN_CHANGE
    public var curseScore:(excess:Int)->Void; //CURSE_SCORE_CHANGE

    public var badBiomes:(id:Int,name:String)->Void; //BAD_BIOMES

    public var vogUpdate:()->Void; //VOG_UPDATE
    public var photo:(x:Int,y:Int,signature:String)->Void; //PHOTO_SIGNATURE

    public var shutdown:()->Void; //FORCED_SHUTDOWN

    public var global:(text:String)->Void; //GLOBAL_MESSAGE
    public var war:(a:Int,b:Int,status:String)->Void; //WAR_REPORT

    public var learnedTools:(list:Array<Int>)->Void; //LEARNED_TOOL_REPORT
    public var toolExperts:(list:Array<Int>)->Void; //TOOL_EXPERTS
    public var toolSlots:(total:Int)->Void; //TOOL_SLOTS
    
    public var babyWiggle:(list:Array<Int>)->Void; //BABY_WIGGLE
    public var saysLocation:(x:Int,y:Int,text:String)->Void; //LOCATION_SAYS
    public var dying:(id:Int,sick:Bool)->Void; //DYING
    public var says:(id:Int,text:String,curse:Bool)->Void; //PLAYER_SAYS
    public var emot:(id:Int,index:Int,sec:Int)->Void; //PLAYER_EMOT
    
    public var mapChunk:(instance:MapInstance)->Void; //MAP_CHUNK
    public var mapChange:(change:MapChange)->Void; //MAP_CHANGE
    public var foodChange:(store:Int,capacity:Int,ateId:Int,fillMax:Int,speed:Float,responsible:Int)->Void; //FOOD_CHANGE
    public var heatChange:(heat:Float,foodTime:Float,indoorBonus:Float)->Void; //HEAT_CHANGE
    public var frame:()->Void; //FRAME
    public var lineage:(list:Array<Int>,eve:Int)->Void; //LINEAGE
    public var healed:(id:Int)->Void; //HEALED
    public var monument:(x:Int,y:Int,id:Int)->Void; //MONUMENT_CALL
    public var grave:(x:Int,y:Int,id:Int)->Void; //GRAVE
    public var graveOld:(x:Int,y:Int,pid:Int,poid:Int,age:Float,name:String,lineage:Array<String>)->Void; //GRAVE_OLD
    public var graveMove:(xs:Int,ys:Int,xd:Int,yd:Int,swapDest:Bool)->Void; //GRAVE_MOVE
    public var ownerList:(x:Int,y:Int,list:Array<Int>)->Void; //OWNER_LIST
    public var valley:(spacing:Int,offset:Int)->Void; //VALLEY_SPACING
    public var flight:(id:Int,x:Int,y:Int)->Void; //FLIGHT_DEST
    public var homeland:(x:Int,y:Int,name:String)->Void; //HOMELAND
    public var craving:(id:Int,bonus:Int)->Void; //CRAVING
    public var flip:(x:Int,y:Int)->Void; //FLIP

    public function new()
    {
        
    }
}