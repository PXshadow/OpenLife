package openlife.engine;
import haxe.ds.IntMap;
import openlife.engine.EngineEvent;
import openlife.data.map.MapData;
import openlife.data.object.player.PlayerMove;
import openlife.data.Pos;
import openlife.data.object.player.PlayerInstance;
import openlife.data.map.MapChange;
import openlife.data.map.MapInstance;
import openlife.client.Client;
import openlife.settings.Settings;
import haxe.io.Path;
import openlife.client.ClientTag;
//
class Engine 
{
    /**
     * static game data
     */
    public var map:MapData;
    public var program:Program;
    public var players:IntMap<PlayerInstance>;
    public var client:Client;
    /**
     * Used for string tool functions
     */
    var string:String;
    public static var dir:String;
    var _mapInstance:MapInstance;
    var _header:EngineHeader;
    var _event:EngineEvent;
    var _eventBool:Bool;
    public var relayPort:Int = 8005;
    public function new(header:EngineHeader,event:EngineEvent=null,dir:String=null)
    {
        if (dir != null) Engine.dir = dir;
        this._header = header;
        _event = event;
        _eventBool = _event != null;
        map = new MapData();
        client = new Client();
        program = new Program(client);
        players = new IntMap<PlayerInstance>();
    }
    public function connect(reconnect:Bool=false,setRelayCallback:Bool=false)
    {
        client.accept = function()
        {
            client.message = message;
            client.accept = null;
        }
        client.reject = function()
        {
            client.reject = null;
        }
        client.message = !setRelayCallback ? client.login : function (tag:ClientTag,_)
        {
            switch (tag)
            {
                case ACCEPTED:
                client.accept();
                case REJECTED:
                client.reject();
                default:
            }
        };
        if (setRelayCallback) client.relay(relayPort);
        client.connect(reconnect);
    }
    private function message(tag:ClientTag,input:Array<String>) 
    {
        switch(tag)
        {
            case COMPRESSED_MESSAGE:
            var array = input[0].split(" ");
            client.compress(Std.parseInt(array[0]),Std.parseInt(array[1]));
            case PLAYER_EMOT:
            var index:Int = 0;
            var index2:Int = 0;
            var secs:Int = 0;
            for (line in input)
            {
                index = line.indexOf(" ");
                index2 = line.indexOf(" ",index + 1);
                if (index2 == -1)
                {
                    //no ttl_sec
                    secs = 10;
                    index2 = line.length;
                }else{
                    //ttl_sec exists
                    secs = Std.parseInt(line.substr(index2 + 1));
                }
                _emot(Std.parseInt(line.substring(0,index)),Std.parseInt(line.substring(index + 1,index2)),secs);
            }
            //p_id emot_index ttl_sec
            //ttl_sec is optional, and specifies how long the emote should be shown
            //-1 is permanent, -2 is permanent but not new so should be skipped
            case PLAYER_UPDATE:
            var list:Array<PlayerInstance> = [];
            var temp:PlayerInstance;
            var player:PlayerInstance;
            for (data in input) 
            {
                temp = new PlayerInstance(data.split(" "));
                player = players.get(temp.p_id);
                if (temp.action == 1)
                {
                    map.object.set(temp.action_target_x,temp.action_target_y,player.o_id);
                }
                trace("action " + temp.action);
                list.push(player);
            }
            _playerUpdate(list);
            case PLAYER_MOVES_START:
            var a:Array<String> = [];
            for (string in input)
            {
                a = string.split(" ");
                if (a.length < 8 || a.length % 2 != 0) continue;
                _playerMoveStart(new PlayerMove(a));
            }
            case MAP_CHUNK:
            if (_mapInstance == null)
            {
                var instance = input[0].split(" ");
                var compress = input[1].split(" ");
                _mapInstance = new MapInstance();
                _mapInstance.width = Std.parseInt(instance[0]);
                _mapInstance.height = Std.parseInt(instance[1]);
                _mapInstance.x = Std.parseInt(instance[2]);
                _mapInstance.y = Std.parseInt(instance[3]);
                client.compress(Std.parseInt(compress[0]),Std.parseInt(compress[1]));
            }else{
                map.setRect(_mapInstance,input[0]);
                _mapChunk(_mapInstance);
                _mapInstance = null;
            }
            case MAP_CHANGE:
            var change:MapChange;
            for (i in 0...input.length - 1)
            {
                change = new MapChange(input[i].split(" "));
                if (change.floor == 0) 
                {
                    map.object.set(change.oldX,change.oldY,[0]);
                    map.object.set(change.x,change.y,change.id);
                }else{
                    map.floor.set(change.x,change.y,change.id[0]);
                }
                _mapChange(change);
            }
            case HEAT_CHANGE:
            //heat food_time indoor_bonus
            var array = input[0].split(" ");
            _heatChange(Std.parseFloat(array[0]),Std.parseFloat(array[1]),Std.parseFloat(array[2]));
            case FOOD_CHANGE:
            var array = input[0].split(" ");
            //food_store food_capacity last_ate_id last_ate_fill_max move_speed responsible_id
            _foodChange(
                Std.parseInt(array[0]),
                Std.parseInt(array[1]),
                Std.parseInt(array[2]),
                Std.parseInt(array[3]),
                Std.parseFloat(array[4]),
                Std.parseInt(array[5])
            );
            case FRAME:
            _frame();
            case PLAYER_SAYS:
            var index:Int = 0;
            for (line in input)
            {
                index = line.indexOf("/");
                _says(Std.parseInt(line.substring(0,index)),line.substr(index + 2),line.substr(index + 1,1) == "1");
            }
            case LOCATION_SAYS:
            var array:Array<String> = [];
            for (line in input)
            {
                array = line.split(" ");
                _saysLocation(Std.parseInt(array[0]),Std.parseInt(array[1]),array[2]);
            }
            case BAD_BIOMES:
            var index:Int = 0;
            for (line in input)
            {
                index = line.indexOf(" ");
                _badBiomes(Std.parseInt(line.substring(0,index)),line.substr(index + 1));
            }
            case PLAYER_OUT_OF_RANGE:
            //player is out of range
            var list:Array<Int> = [];
            for (string in input) list.push(Std.parseInt(string));
            _playerOutOfRange(list);
            case BABY_WIGGLE:
            var list:Array<Int> = [];
            for (string in input) list.push(Std.parseInt(string));
            _babyWiggle(list);
            case LINEAGE:
            //p_id mother_id grandmother_id great_grandmother_id ... eve_id eve=eve_id
            //included at the end with the eve= tag in front of it.
            var array:Array<String> = [];
            for (line in input)
            {
                var array = line.split(" ");
                var list:Array<Int> = [];
                for (i in 0...array.length - 1)
                {
                    list.push(Std.parseInt(array[i]));
                }
                var eveString = array[array.length - 1];
                var eve = Std.parseInt(eveString.substring(eveString.indexOf("=") + 1));
                _lineage(list,eve);
            }
            case NAME:
            //p_id first_name last_name last_name may be ommitted.
            var array:Array<String> = [];
            var lastName:String = "";
            for (line in input)
            {
                array = line.split(" ");
                if (array.length > 2)
                {
                    //last name
                    lastName = array[2];
                }else{
                    //no last name
                    lastName = "";
                }
                _playerName(Std.parseInt(array[0]),array[1],lastName);
            }
            case APOCALYPSE:
            //Indicates that an apocalypse is pending.  Gives client time to show a visual effect.
            _apocalypse();
            case APOCALYPSE_DONE:
            //Indicates that an apocalypse is now over.  Client should go back to displaying world.
            case DYING:
            var index:Int = 0;
            var sick:Bool;
            for (line in input)
            {
               index = line.indexOf(" ");
               if (index == -1)
               {
                   index = line.length;
                   sick = false;
               }else{
                   sick = true;
               }
               _dying(Std.parseInt(line.substring(0,index)),sick);
            }
            _apocalypseDone();
            case HEALED:
            //p_id player healed no longer dying.
            for (line in input)
            {
                _healed(Std.parseInt(line));
            }
            case POSSE_JOIN: //FINISH tommrow
            //Indicates that killer joined posse of target.
            //If target = 0, killer has left the posse.
            var array = input[0].split(" ");
            _posse(Std.parseInt(array[0]),Std.parseInt(array[1]));
            case MONUMENT_CALL:
            //MN x y o_id monument call has happened at location x,y with the creation object id
            var array = input[0].split(" ");
            _monument(Std.parseInt(array[0]),Std.parseInt(array[1]),Std.parseInt(array[2]));
            case GRAVE:
            //x y p_id
            var x:Int = Std.parseInt(input[0]);
            var y:Int = Std.parseInt(input[1]);
            var id:Int = Std.parseInt(input[2]);
            case GRAVE_MOVE:
            //xs ys xd yd swap_dest optional swap_dest parameter is 1, it means that some other grave at  destination is in mid-air.  If 0, not

            case GRAVE_OLD:
            //x y p_id po_id death_age underscored_name mother_id grandmother_id great_grandmother_id ... eve_id eve=eve_id
            //Provides info about an old grave that wasn't created during your lifetime.
            //underscored_name is name with spaces replaced by _ If player has no name, this will be ~ character instead.
            
            case OWNER_LIST:
            //x y p_id p_id p_id ... p_id

            case VALLEY_SPACING:
            //y_spacing y_offset Offset is from client's birth position (0,0) of first valley.
            //data.map.valleySpacing = Std.parseInt(input[0]);
            //data.map.valleyOffsetY = Std.parseInt(input[1]);
            
            case FLIGHT_DEST:
            //p_id dest_x dest_y
            trace("FLIGHT FLIGHT FLIGHT " + input);
            case PONG:
            //client.ping = Std.int(UnitTest.stamp() * 100);
            //trace("ping: " + client.ping);
            case HOMELAND:
            var array = input[0].split(" ");
            _homeland(Std.parseInt(array[0]),Std.parseInt(array[1]),array[2]);
            case CRAVING:
            _craving(Std.parseInt(input[0]),Std.parseInt(input[1]));
            case FLIP:
            _flip(Std.parseInt(input[0]),Std.parseInt(input[1]));
            default:
        }
    }
    //functions
    private inline function _playerUpdate(instances:Array<PlayerInstance>)
    {
        _header.playerUpdate(instances);
        if (_eventBool && _event.playerUpdate != null) _event.playerUpdate(instances);
    } //PLAYER_UPDATE
    private inline function _playerMoveStart(move:PlayerMove)
    {
        _header.playerMoveStart(move);
        if (_eventBool && _event.playerMoveStart != null) _event.playerMoveStart(move);
    } //PLAYER_MOVES_START

    private inline function _playerOutOfRange(list:Array<Int>)
    {
        _header.playerOutOfRange(list);
        if (_eventBool && _event.playerOutOfRange != null) _event.playerOutOfRange(list);
    } //PLAYER_OUT_OF_RANGE
    private inline function _playerName(id:Int,firstName:String,lastName:String)
    {
        _header.playerName(id,firstName,lastName);
        if (_eventBool && _event.playerName != null) _event.playerName(id,firstName,lastName);
    } //NAME

    private inline function _apocalypse()
    {
        _header.apocalypse();
        if (_eventBool && _event.apocalypse != null) _event.apocalypse();
    } //APOCALYPSE
    private inline function _apocalypseDone()
    {
        _header.apocalypseDone();
        if (_eventBool && _event.apocalypseDone != null) _event.apocalypseDone();
    } //APOCALYPSE_DONE

    private inline function _posse(killer:Int,target:Int)
    {
        _header.posse(killer,target);
        if (_eventBool && _event.posse != null) _event.posse(killer,target);
    } //POSSE_JOIN

    private inline function _following(follower:Int,leader:Int,color:Int)
    {
        _header.following(follower,leader,color);
        if (_eventBool && _event.following != null) _event.following(follower,leader,color);
    } //FOLLOWING
    private inline function _exiled(target:Int,id:Int)
    {
        _header.exiled(target,id);
        if (_eventBool && _event.exiled != null) _event.exiled(target,id);
    } //EXILED
    private inline function _cursed(id:Int,level:Int,word:String)
    {
        _header.cursed(id,level,word);
        if (_eventBool && _event.cursed != null) _event.cursed(id,level,word);
    } //CURSED
    private inline function _curseToken(count:Int)
    {
        _header.curseToken(count);
        if (_eventBool && _event.curseToken != null) _event.curseToken(count);
    } //CURSE_TOKEN_CHANGE
    private inline function _curseScore(excess:Int)
    {
        _header.curseScore(excess);
        if (_eventBool && _event.curseScore != null) _event.curseScore(excess);
    } //CURSE_SCORE_CHANGE

    private inline function _badBiomes(id:Int,name:String)
    {
        _header.badBiomes(id,name);
        if (_eventBool && _event.badBiomes != null) _event.badBiomes(id,name);
    } //BAD_BIOMES

    private inline function _vogUpdate()
    {
        _header.vogUpdate();
        if (_eventBool && _event.vogUpdate != null) _event.vogUpdate();
    } //VOG_UPDATE
    private inline function _photo(x:Int,y:Int,signature:String)
    {
        _header.photo(x,y,signature);
        if (_eventBool && _event.photo != null) _event.photo(x,y,signature);
    } //PHOTO_SIGNATURE

    private inline function _shutdown()
    {
        _header.shutdown();
        if (_eventBool && _event.shutdown != null) _event.shutdown();
    } //FORCED_SHUTDOWN

    private inline function _global(text:String)
    {
        _header.global(text);
        if (_eventBool && _event.global != null) _event.global(text);
    } //GLOBAL_MESSAGE
    private inline function _war(a:Int,b:Int,status:String)
    {
        _header.war(a,b,status);
        if (_eventBool && _event.war != null) _event.war(a,b,status);
    } //WAR_REPORT

    private inline function _learnedTools(list:Array<Int>)
    {
        _header.learnedTools(list);
        if (_eventBool && _event.learnedTools != null) _event.learnedTools(list);
    } //LEARNED_TOOL_REPORT
    private inline function _toolExperts(list:Array<Int>)
    {
        _header.toolExperts(list);
        if (_eventBool && _event.toolExperts != null) _event.toolExperts(list);
    } //TOOL_EXPERTS
    private inline function _toolSlots(total:Int)
    {
        _header.toolSlots(total);
        if (_eventBool && _event.toolSlots != null) _event.toolSlots(total);
    } //TOOL_SLOTS
    
    private inline function _babyWiggle(list:Array<Int>)
    {
        _header.babyWiggle(list);
        if (_eventBool && _event.babyWiggle != null) _event.babyWiggle(list);
    } //BABY_WIGGLE
    private inline function _saysLocation(x:Int,y:Int,text:String)
    {
        _header.saysLocation(x,y,text);
        if (_eventBool && _event.saysLocation != null) _event.saysLocation(x,y,text);
    } //LOCATION_SAYS
    private inline function _dying(id:Int,sick:Bool)
    {
        _header.dying(id,sick);
        if (_eventBool && _event.dying != null) _event.dying(id,sick);
    } //DYING
    private inline function _says(id:Int,text:String,curse:Bool)
    {
        _header.says(id,text,curse);
        if (_eventBool && _event.says != null) _event.says(id,text,curse);
    } //PLAYER_SAYS
    private inline function _emot(id:Int,index:Int,sec:Int)
    {
        _header.emot(id,index,sec);
        if (_eventBool && _event.emot != null) _event.emot(id,index,sec);
    } //PLAYER_EMOT
    
    private inline function _mapChunk(instance:MapInstance)
    {
        _header.mapChunk(instance);
        if (_eventBool && _event.mapChunk != null) _event.mapChunk(instance);
    } //MAP_CHUNK
    private inline function _mapChange(change:MapChange)
    {
        _header.mapChange(change);
        if (_eventBool && _event.mapChange != null) _event.mapChange(change);
    } //MAP_CHANGE
    private inline function _foodChange(store:Int,capacity:Int,ateId:Int,fillMax:Int,speed:Float,responsible:Int)
    {
        _header.foodChange(store,capacity,ateId,fillMax,speed,responsible);
        if (_eventBool && _event.foodChange != null) _event.foodChange(store,capacity,ateId,fillMax,speed,responsible);
    } //FOOD_CHANGE
    private inline function _heatChange(heat:Float,foodTime:Float,indoorBonus:Float)
    {
        _header.heatChange(heat,foodTime,indoorBonus);
        if (_eventBool && _event.heatChange != null) _event.heatChange(heat,foodTime,indoorBonus);
    } //HEAT_CHANGE
    private inline function _frame()
    {
        _header.frame();
        if (_eventBool && _event.frame != null) _event.frame();
    } //FRAME
    private inline function _lineage(list:Array<Int>,eve:Int)
    {
        _header.lineage(list,eve);
        if (_eventBool && _event.lineage != null) _event.lineage(list,eve);
    } //LINEAGE
    private inline function _healed(id:Int)
    {
        _header.healed(id);
        if (_eventBool && _event.healed != null) _event.healed(id);
    } //HEALED
    private inline function _monument(x:Int,y:Int,id:Int)
    {
        _header.monument(x,y,id);
        if (_eventBool && _event.monument != null) _event.monument(x,y,id);
    } //MONUMENT_CALL
    private inline function _grave(x:Int,y:Int,id:Int)
    {
        _header.grave(x,y,id);
        if (_eventBool && _event.grave != null) _event.grave(x,y,id);
    } //GRAVE
    private inline function _graveOld(x:Int,y:Int,pid:Int,poid:Int,age:Float,name:String,lineage:Array<String>)
    {
        _header.graveOld(x,y,pid,poid,age,name,lineage);
        if (_eventBool && _event.graveOld != null) _event.graveOld(x,y,pid,poid,age,name,lineage);
    } //GRAVE_OLD
    private inline function _graveMove(xs:Int,ys:Int,xd:Int,yd:Int,swapDest:Bool)
    {
        _header.graveMove(xs,ys,xd,yd,swapDest);
        if (_eventBool && _event.graveMove != null) _event.graveMove(xs,ys,xd,yd,swapDest);
    } //GRAVE_MOVE
    private inline function _ownerList(x:Int,y:Int,list:Array<Int>)
    {
        _header.ownerList(x,y,list);
        if (_eventBool && _event.ownerList != null) _event.ownerList(x,y,list);
    } //OWNER_LIST
    private inline function _valley(spacing:Int,offset:Int)
    {
        _header.valley(spacing,offset);
        if (_eventBool && _event.valley != null) _event.valley(spacing,offset);
    } //VALLEY_SPACING
    private inline function _flight(id:Int,x:Int,y:Int)
    {
        _header.flight(id,x,y);
        if (_eventBool && _event.flight != null) _event.flight(id,x,y);
    } //FLIGHT_DEST
    private inline function _homeland(x:Int,y:Int,name:String)
    {
        _header.homeland(x,y,name);
        if (_eventBool && _event.homeland != null) _event.homeland(x,y,name);
    } //HOMELAND
    private inline function _craving(id:Int,bonus:Int)
    {
        _header.craving(id,bonus);
        if (_eventBool && _event.craving != null) _event.craving(id,bonus);
    } //CRAVING
    private inline function _flip(x:Int,y:Int)
    {
        _header.flip(x,y);
        if (_eventBool && _event.flip != null) _event.flip(x,y);
    } //FLIP
}