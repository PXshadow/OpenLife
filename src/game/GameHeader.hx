package game;
import data.map.MapInstance;
import data.object.player.PlayerInstance;
import data.object.player.PlayerMove;
import data.map.MapChange;
class GameHeader #if openfl extends openfl.display.Sprite #elseif heaps hxd.App #end
{
    //Headers not included:
    //COMPRESSED_MESSAGE

    public function playerUpdate(instances:Array<PlayerInstance>) {}; //PLAYER_UPDATE
    public function playerMoveStart(move:PlayerMove) {}; //PLAYER_MOVES_START
    public function playerOutOfRange() {}; //PLAYER_OUT_OF_RANGE
    public function playerName() {}; //NAME

    public function apocalypse() {}; //APOCALYPSE
    public function apocalypseDone() {}; //APOCALYPSE_DONE

    public function posse() {}; //POSSE_JOIN

    public function following() {}; //FOLLOWING
    public function exiled() {}; //EXILED
    public function cursed() {}; //CURSED
    public function curseToken() {}; //CURSE_TOKEN_CHANGE
    public function curseScore() {}; //CURSE_SCORE_CHANGE

    public function badBiomes() {}; //BAD_BIOMES

    public function vogUpdate() {}; //VOG_UPDATE
    public function photo() {}; //PHOTO_SIGNATURE

    public function shutdown() {}; //FORCED_SHUTDOWN

    public function global() {}; //GLOBAL_MESSAGE
    public function war() {}; //WAR_REPORT

    public function learnedTools() {}; //LEARNED_TOOL_REPORT
    public function toolExperts() {}; //TOOL_EXPERTS
    public function toolSlots() {}; //TOOL_SLOTS

    public function babyWiggle() {}; //BABY_WIGGLE
    public function location() {}; //LOCATION_SAYS
    public function dying() {}; //DYING
    public function says(id:Int,text:String,curse:Bool) {}; //PLAYER_SAYS
    public function emot(p_id:Int,emot_index:Int,ttl_sec:Int) {}; //PLAYER_EMOT
    
    public function mapChunk(instance:MapInstance) {}; //MAP_CHUNK
    public function mapChange(change:MapChange) {}; //MAP_CHANGE

    public function foodChange() {}; //FOOD_CHANGE
    public function heatChange(current:Float,total:Float) {}; //HEAT_CHANGE
    public function frame() {}; //FRAME
    public function lineage() {}; //LINEAGE
    public function healed() {}; //HEALED
    public function monument() {}; //MONUMENT_CALL
    public function grave() {}; //GRAVE
    public function graveOld() {}; //GRAVE_OLD
    public function graveMove() {}; //GRAVE_MOVE
    public function ownerList() {}; //OWNER_LIST
    public function valley() {}; //VALLEY_SPACING
    public function flight() {}; //FLIGHT_DEST
}