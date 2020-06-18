package;

import haxe.ds.IntMap;
import openlife.data.object.player.PlayerInstance;
import openlife.engine.Program;
import openlife.data.map.MapInstance;
import openlife.engine.*;
import openlife.data.object.player.PlayerMove;
import openlife.data.map.MapChange;

class App extends Engine implements EngineHeader
{
    var player:PlayerInstance;
    var players = new IntMap<PlayerInstance>();
    var names = new IntMap<String>();
    var program:Program;
    public function new()
    {
        super(this);
        program = new Program(client);
        var bool:Bool = false;
        Config.run(client,cred());
        connect(false);
        while (true)
        {
            client.update();
            Sys.sleep(1/30);
        }
    }
    public function reborn()
    {
        players.clear();
        names.clear();
        player = null;
        Engine.data.clear();
        client.close();
        connect();
        trace("NEW CONNECT");
    }
    //events
    public function playerUpdate(instances:Array<PlayerInstance>)
    {
        var inst:PlayerInstance;
        for (instance in instances)
        {
            inst = players.get(instance.p_id);
            players.set(instance.p_id,instance);
            if (inst != null)
            {
                if (!instance.forced)
                {
                    instance.x = inst.x;
                    instance.y = inst.y;
                }
                instance.forced = false;
            }
            if (player != null && instance.p_id == player.p_id)
            {
                trace('my player ${player.age}');
            }
        }
        if (player == null)
        {
            player = instances.pop();
            //new player set
        }
    } //PLAYER_UPDATE
    public function playerMoveStart(move:PlayerMove)
    {

    } //PLAYER_MOVES_START

    public function playerOutOfRange(list:Array<Int>)
    {

    } //PLAYER_OUT_OF_RANGE
    public function playerName(id:Int,firstName:String,lastName:String)
    {
        trace("names " + firstName + " lastname " + lastName);
        names.set(id,firstName + " " + lastName);
    } //NAME

    public function apocalypse()
    {

    } //APOCALYPSE
    public function apocalypseDone()
    {

    } //APOCALYPSE_DONE

    public function posse(killer:Int,target:Int)
    {

    } //POSSE_JOIN

    public function following(follower:Int,leader:Int,color:Int)
    {

    } //FOLLOWING
    public function exiled(target:Int,id:Int)
    {

    } //EXILED
    public function cursed(id:Int,level:Int,word:String)
    {

    } //CURSED
    public function curseToken(count:Int)
    {

    } //CURSE_TOKEN_CHANGE
    public function curseScore(excess:Int)
    {

    } //CURSE_SCORE_CHANGE

    public function badBiomes(id:Int,name:String)
    {

    } //BAD_BIOMES

    public function vogUpdate()
    {

    } //VOG_UPDATE
    public function photo(x:Int,y:Int,signature:String)
    {

    } //PHOTO_SIGNATURE

    public function shutdown()
    {

    } //FORCED_SHUTDOWN

    public function global(text:String)
    {

    } //GLOBAL_MESSAGE
    public function war(a:Int,b:Int,status:String)
    {

    } //WAR_REPORT

    public function learnedTools(list:Array<Int>)
    {

    } //LEARNED_TOOL_REPORT
    public function toolExperts(list:Array<Int>)
    {

    } //TOOL_EXPERTS
    public function toolSlots(total:Int)
    {

    } //TOOL_SLOTS
    
    public function babyWiggle(list:Array<Int>)
    {

    } //BABY_WIGGLE
    public function saysLocation(x:Int,y:Int,text:String)
    {

    } //LOCATION_SAYS
    public function dying(id:Int,sick:Bool)
    {

    } //DYING
    public function says(id:Int,text:String,curse:Bool)
    {
        trace('id $id say $text');
        if (text.indexOf("HI") > -1 || text.indexOf("HELLO") > -1 || text.indexOf("HEY") > -1)
        {
            program.say("HI");
            return;
        }
        if (text.indexOf("FOLLOW") > -1)
        {
            //follow script of mother
            return;
        }
        if (text.indexOf("UP") > -1)
        {
            program.say("UP");
            program.step(player,0,1);
            return;
        }
        if (text.indexOf("DOWN") > -1)
        {
            program.say("DOWN");
            program.step(player,0,-1);
            return;
        }
        if (text.indexOf("LEFT") > -1)
        {
            program.say("LEFT");
            program.step(player,-1,0);
            return;
        }
        if (text.indexOf("RIGHT") > -1)
        {
            program.say("RIGHT");
            program.step(player,1,0);
            return;
        }
        if (text.indexOf("USE") > -1)
        {
            program.say("USE");
            program.use(player.x,player.y);
            return;
        }
        program.say("HELLO " + names.get(id));
    } //PLAYER_SAYS
    public function emot(id:Int,index:Int,sec:Int)
    {

    } //PLAYER_EMOT
    
    public function mapChunk(instance:MapInstance)
    {
        trace("instance " + instance.toString());
    } //MAP_CHUNK
    public function mapChange(change:MapChange)
    {

    } //MAP_CHANGE
    public function foodChange(store:Int,capacity:Int,ateId:Int,fillMax:Int,speed:Float,responsible:Int)
    {

    } //FOOD_CHANGE
    public function heatChange(heat:Float,foodTime:Float,indoorBonus:Float)
    {

    } //HEAT_CHANGE
    public function frame()
    {

    } //FRAME
    public function lineage(list:Array<String>)
    {

    } //LINEAGE
    public function healed(id:Int)
    {

    } //HEALED
    public function monument(x:Int,y:Int,id:Int)
    {

    } //MONUMENT_CALL
    public function grave(x:Int,y:Int,id:Int)
    {

    } //GRAVE
    public function graveOld(x:Int,y:Int,pid:Int,poid:Int,age:Float,name:String,lineage:Array<String>)
    {

    } //GRAVE_OLD
    public function graveMove(xs:Int,ys:Int,xd:Int,yd:Int,swapDest:Bool)
    {

    } //GRAVE_MOVE
    public function ownerList(x:Int,y:Int,list:Array<Int>)
    {

    } //OWNER_LIST
    public function valley(spacing:Int,offset:Int)
    {

    } //VALLEY_SPACING
    public function flight(id:Int,x:Int,y:Int)
    {

    } //FLIGHT_DEST
    public function homeland(x:Int,y:Int,name:String)
    {

    } //HOMELAND
}