package;
import openlife.auto.Role;
import openlife.auto.Action;
import openlife.resources.ObjectBake;
import openlife.engine.EngineEvent;
import openlife.engine.Utility;
import openlife.auto.Automation;
import openlife.engine.Program;
import openlife.client.Client;
import openlife.engine.EngineHeader;
import openlife.engine.Engine;
import openlife.data.object.player.PlayerInstance;
import openlife.data.object.player.PlayerMove;
import openlife.data.map.MapInstance;
import openlife.data.map.MapChange;
import openlife.data.object.ObjectData;
import openlife.data.Target;
import haxe.ds.Map;
class Bot extends Engine implements EngineHeader
{
    public var currentAction:Action;
    public var target:openlife.data.Target;
    public var auto:Automation;
    public var resetFlag:Bool = false;
    public var names = new Map<Int,String>();
    var followingId:Int = -1;
    public var event:EngineEvent;
    private static var staticDelay:Float = 0;
    var reconnectBool:Bool = true;
    public function new(client:Client)
    {
        event = new EngineEvent();
        super(this,event,client);
        client.onClose = close;
        client.onReject = function()
        {
            reconnectBool = false;
        }
        client.onAccept = function()
        {
            reconnectBool = true;
        }
        this.setPlayer = function()
        {
            //new player set
            auto = new Automation(program,App.vector);
            #if script
            trace("EXECUTING SCRIPT");
            Script.main(this);
            #end
        }
    }
    private function close()
    {
        //reconnect
        Sys.sleep(2);
        var relay = client.relayIn != null ? true : false;
        player = null;
        clear();
        names.clear();
        players.clear();
        connect(reconnectBool,relay);
    }
    public function test()
    {
        
    }
    public function update()
    {
        client.update();
    }
    //events
    public function playerUpdate(instances:Array<PlayerInstance>)
    {
        
    } //PLAYER_UPDATE
    public function playerMoveStart(move:PlayerMove)
    {
        if (player.p_id == move.id) program.playerMainMove(player,move);
    } //PLAYER_MOVES_START

    public function playerOutOfRange(list:Array<Int>)
    {

    } //PLAYER_OUT_OF_RANGE
    public function playerName(id:Int,firstName:String,lastName:String)
    {
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
        program.say("I AM DYING!");
    } //DYING
    var found:Int = -1;
    public function says(id:Int,text:String,curse:Bool)
    {
        if (id == player.p_id) return;
        var words = text.split(" ");
        words.shift();
        var index:Int = 0;
        if (words.indexOf("YOU") > -1 && (index = words.indexOf("KNOW") + 1) > 0)
        {
            found = auto.interp.stringObject(words.slice(index,words.length));
            if (found == -1)
            {
                program.say("I DO NOT KNOW");
                return;
            }
            program.say('I KNOW! ${new ObjectData(found).description}');
        }
        if ((words.indexOf("FIND") > -1 || words.indexOf("GO") > -1) && found > -1)
        {
            var id = ObjectBake.dummies.get(found);
            id == null ? id = [found] : id.unshift(found);
            var pos = auto.select(id);
            if (pos == null)
            {
                program.say("I DID NOT FIND");
                return;
            }
            program.say("I GO THERE NOW");
            program.goto(pos.x,pos.y);
        }
        if ((index = words.indexOf("FOLLOW") + 1) > 0)
        {
            followingId = id;
            var p = players.get(followingId);
            program.goto(p.x,p.y);
        }
        if ((index = words.indexOf("HERE") + 1) > 0)
        {
            var p = players.get(id);
            program.goto(p.x,p.y);
        }
        if (words.indexOf("PICK") > -1 && words.indexOf("UP") > -1 || words.indexOf("USE") > -1)
        {
            program.use(player.x,player.y);
        }
        if (words.indexOf("SELF") > -1 || words.indexOf("EAT") > -1)
        {
            program.self();
        }
        if (words.indexOf("DROP") > -1)
        {
            program.drop(player.x,player.y);
        }
        if ((index = words.indexOf("STOP") + 1) > 0)
        {
            followingId = -1;
        }
        if (words.indexOf("PING") > -1)
        {
            program.say("PONG");
        }
        if (words.indexOf("MARCO") > -1)
        {
            program.say("POLO");
        }
    } //PLAYER_SAYS
    public function emot(id:Int,index:Int,sec:Int)
    {

    } //PLAYER_EMOT
    
    public function mapChunk(instance:MapInstance)
    {
        
    } //MAP_CHUNK
    public function mapChange(change:MapChange)
    {

    } //MAP_CHANGE
    public function foodChange(store:Int,capacity:Int,ateId:Int,fillMax:Int,speed:Float,responsible:Int)
    {
        if (store/capacity < 0.2) program.say("F");
    } //FOOD_CHANGE
    public function heatChange(heat:Float,foodTime:Float,indoorBonus:Float)
    {

    } //HEAT_CHANGE
    public function frame()
    {

    } //FRAME
    public function lineage(list:Array<Int>,eve:Int)
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
    public function flip(x:Int,y:Int)
    {

    } //FLIP
    public function craving(id:Int,bonus:Int)
    {
        
    } //CRAVING
    
    //Used for actions and roles
    public function moveTo(x:Int, y:Int){
        //if path not already calculated get one from the map
        //store the path in the bot
        //shift the path (remove the first element of the array) and store the node in a temp variable
        //check if the node is walkable - if not repath
        //if needed repath twice and give up
        //walk the node
        //wash and repeat every movement tick until at destination
    }
}
class Target{

}