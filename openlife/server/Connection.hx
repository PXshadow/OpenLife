package openlife.server;
import openlife.settings.ServerSettings;
import openlife.data.map.MapData;
import format.swf.Data.PlaceObject;
#if (target.threaded)
import openlife.data.Pos;
import openlife.data.object.player.PlayerInstance;
import openlife.data.object.player.PlayerMove;
import openlife.client.ClientTag;
import sys.net.Socket;
import haxe.io.Bytes;

class Connection implements ServerHeader
{
    public var running:Bool = true;
    var sock:Socket;
    var server:Server;
    var tag:ServerTag;
    public var player:GlobalPlayerInstance;

    //public function get_player():GlobalPlayerInstance{return player;}

    public function new(sock:Socket,server:Server)
    {
        this.sock = sock;
        this.server = server;
        var challenge = "dsdjsiojdiasjiodsjiosd";
        var version = server.dataVersionNumber;
        send(SERVER_INFO,["0/0",challenge,'$version']);
    }

    // TODO Arcurus>> add birth logic - suggestion:
    // select mother or Admam / Eve
    // if no mother 50% born as Adam 50 % born as Eve
    // First companion of Adam is Eve, of Eve it is Adam

    // TODO Arcurus>> "curses" function through dead bodies that are not properly burried
    // bone pile an normal grave blocks 200 Tiles nearby
    // bone pile dos not decay
    // grave with at least a grave stone block for 15 min
    // additional if you are blocked, you are shown "cursed" to others of you go near
    // for "cursed" your name is consantly shown in "cursed" color
    // "cursed" lowers your speed to 80% and pickup of Age 3 items (you can still use if you have one)
    // "cursed" hinders you to engage with your own dead body 
    // if you are blocked everywhere you may be born as "lowborn"

    // TODO Arcurus>> birth logic if you are not blocked
    // mothers on horses / cars cannot have children
    // mothers who where not close to a male in last 9 months cannot have a child 
    // mother must be at least 14 and max 40
    // X2 times chance for each grave with at least a gravestone nearby (100 Tiles)
    // X1/2 chance for each living child a mother has
    // X (score this life) / (average this live score of living players) (score is connected to YUM plus extra)
    
    // TODO Arcurus>> nobles and low born
    // If you are top 20% score of currently playing players (min 5 player) you are born as "noble"
    // If you are lowest 20% score of currently playing players (min 5 player) you are born as "low born"
    // as noble / low born first noble / low born mothers are considered
    // (new players have a 50% change of noble birth in their first 5 lifes)
    // nobels follow by default the leader
    // by default you follow your mother or / and??? father 50%
    // if your mother / father dies, you follow the noble of the mother / father
    // people in a village are distributed as followers among the nobles if a nobles dies
    
    // TODO Arcurus>> prince
    // if you have the highest score in this village (not counting the leader score) you are born as prince / princess to the leader
    // the eldest prince / princess becomes the crown prince
    // if there is no prince the noble with the highest score in this village becomes Cancelor
    // exiles / commands from crown prince / cancelor are valid for all followers if not overriden by the leader
    // giving a crown from the leader to a noble or prince makes them the new Cancelor / crown prince as long as he keeps the crown. 
    // A cancelor with a crown will get the new leader in case of the leaders death

    public function login()
    {
        send(ACCEPTED);
        // TODO choose better mutex
        server.map.mutex.acquire();
        
        server.connections.push(this);

        player = new GlobalPlayerInstance([]);
        
        player.connection = this;
        var id = server.playerIndex++;
        player.p_id = id;
        player.gx = ServerSettings.startingGx;
        player.gy = ServerSettings.startingGy;

        player.move_speed = MoveExtender.calculateSpeed(player, player.gx, player.gy);
        player.food_store_max = TimeHelper.CalculateFoodStoreMax(player);
        player.food_store = player.food_store_max / 2;
        
        trace("move_speed: " + player.move_speed);

        sendMapChunk(0,0);

        SendUpdateToAllClosePlayers(player);
        
        send(LINEAGE,['$id eve=$id']);
        send(TOOL_SLOTS,["0 1000"]);
        player.sendFoodUpdate();
        send(FRAME);
        server.map.mutex.release();
    }

    public static function SendUpdateToAllClosePlayers(player:GlobalPlayerInstance, isPlayerAction:Bool = true)
    {
        for (c in Server.server.connections)
        {
            // since player has relative coordinates, transform them for player
            var targetX = player.tx() - c.player.gx;
            var targetY = player.ty() - c.player.gy;

            // update only close players
            if(c.player.isClose(targetX,targetY, ServerSettings.maxDistanceToBeConsideredAsClose) == false) continue;

            c.send(PLAYER_UPDATE,[player.toRelativeData(c.player)], isPlayerAction);
            c.send(FRAME, null, isPlayerAction);
        }
    }

    public function close()
    {
        running = false;
        sock.close();
        server.connections.remove(this);
    }
    
    public function keepAlive()
    {

    }

    public function die()
    {
        server.connections.remove(this);
        sock.close();
    }

    public function say(text:String)
    {
        var curse = 0;
        var id = player.p_id;
        for (c in server.connections)
        {
            // TODO why send movement ???
            c.send(PLAYER_MOVES_START,[
                "p_id xs ys total_sec eta_sec trunc xdelt0 ydelt0 ... xdeltN ydeltN",
                "p_id xs ys total_sec eta_sec trunc xdelt0 ydelt0 ... xdeltN ydeltN",
            ]);
            c.send(PLAYER_SAYS,['$id/$curse $text']);
            c.send(FRAME);
        }
    }

    public function flip()
    {
        
    }   

    public function sendMapChunk(x:Int,y:Int,width:Int = 32,height:Int = 30)
    {
        x -= Std.int(width / 2);
        y -= Std.int(height / 2);
              
        var map = server.map.getChunk(x + player.gx, y + player.gy, width, height).toString();
        var uncompressed = Bytes.ofString(map);
        var bytes = haxe.zip.Compress.run(uncompressed,-1);
        
        send(MAP_CHUNK,['$width $height $x $y','${uncompressed.length} ${bytes.length}']);
        sock.output.write(bytes);
        //send(VALLEY_SPACING,["40 40"]); // TODO what is this for?
        //send(FRAME);
    }

    /*
    MX
    x y new_floor_id new_id p_id
    #

    Or 

    MX
    x y new_floor_id new_id p_id old_x old_y speed
    #
    */
    public function sendMapUpdate(x:Int, y:Int, newFloorId:Int, newObjectId:Array<Int>, playerId:Int, isPlayerAction:Bool = true)
    {
        send(MAP_CHANGE,['$x $y $newFloorId ${MapData.stringID(newObjectId)} $playerId'], isPlayerAction);
    }

    public function sendMapUpdateForMoving(toX:Int, toY:Int, newFloorId:Int, newObjectId:Array<Int>, playerId:Int, fromX:Int, fromY:Int, speed:Float)
    {
        send(MAP_CHANGE,['$toX $toY $newFloorId ${MapData.stringID(newObjectId)} $playerId $fromX $fromY $speed'], false);
    }
    
    public function emote(id:Int)
    {
        for (c in server.connections)
        {
            c.send(FRAME);
            c.send(PLAYER_EMOT,['${player.p_id} $id']);
        }
    }
    
    public function rlogin()
    {
        login();
    }

    public function send(tag:ClientTag,data:Array<String>=null, isPlayerAction:Bool = true)
    {
        var string = data != null ? '$tag\n${data.join("\n")}\n#' : '$tag\n#';
        sock.output.writeString(string);

        //if(ServerSettings.TraceSend && tag != MAP_CHANGE && tag != FRAME)
        if((ServerSettings.TraceSendPlayerActions && isPlayerAction) || (ServerSettings.TraceSendNonPlayerActions && isPlayerAction == false))
        {
            var tmpString = StringTools.replace(string, "\n", "\t");
            trace("Send: " + tmpString);
        }
    }

    public function sendPong(unique_id:String)
    {
        var tmpString = '$PONG\n$unique_id#';

        sock.output.writeString(tmpString);

        if(ServerSettings.TraceSendPlayerActions) trace("Send: " + tmpString);
    }
}
#end