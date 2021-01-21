package openlife.server;
import sys.thread.Mutex;
import openlife.settings.ServerSettings;
import openlife.data.map.MapData;
#if (target.threaded)
import openlife.client.ClientTag;
import sys.net.Socket;
import haxe.io.Bytes;

class Connection 
{
    private static var globalMutex = new Mutex();
    private static var connections:Array<Connection> = [];
    private static var ais:Array<ServerAi> = [];

    private var mutex = new Mutex();


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
        //server.map.mutex.acquire();
        //this.mutex.acquire();

        try
        {
            send(ACCEPTED);
                   
            this.player = GlobalPlayerInstance.CreateNew();
            
            var id = player.p_id;

            player.connection = this;

            //player.transformHeldObject(2710);
            
            trace("login: move_speed: " + player.move_speed);

            sendMapChunk(0,0);
            
            send(LINEAGE,['$id eve=$id']);
            send(TOOL_SLOTS,["0 1000"]);

            //trace('food_store_max: ${player.food_store_max}');

            //player.setHeldObject(ObjectHelper.readObjectHelper(player, [2098]));

            addToConnections();

            // send PU and FRAME also to the connection --> therefore make sure that addToConnections is called first 
            SendUpdateToAllClosePlayers(player); 
            SendToMeAllClosePlayers(player);
            player.sendFoodUpdate();

            send(FRAME, null, true);
        }
        catch(ex)
        {
            trace(ex.details);
        }

        //this.mutex.release();
        //server.map.mutex.release();
    }
    
    public static function getConnections() : Array<Connection>
    {
        return connections;
    }
    public static function getAis() : Array<ServerAi>
    {
        return ais;
    }
    
    public static function addAi(ai:ServerAi)
    {
        ais.push(ai);
    }

    public static function getPlayerAt(x:Int, y:Int, playerId:Int) : GlobalPlayerInstance
    {
        for(c in connections)
        {
            if(c.player.deleted) continue;

            if(c.player.p_id == playerId) return c.player;

            if(playerId <= 0)
            {
                if(c.player.x == x && c.player.y == y) return c.player;
            }
        }

        for(c in ais)
        {
            if(c.player.deleted) continue;

            if(c.player.p_id == playerId) return c.player;

            if(playerId <= 0)
            {
                if(c.player.x == x && c.player.y == y) return c.player;
            }
        }

        return null;
    }

    public static function SendUpdateToAllClosePlayers(player:GlobalPlayerInstance, isPlayerAction:Bool = true, sendFrame:Bool = true)
    {
        try
        {
            player.MakeSureHoldObjIdAndDummyIsSetRightAndNullObjUsed(); // TODO better change, since it can mess with other threads

            for (c in connections)
            {
                // since player has relative coordinates, transform them for player
                var targetX = player.tx() - c.player.gx;
                var targetY = player.ty() - c.player.gy;

                // update only close players
                if(c.player.isClose(targetX,targetY, ServerSettings.maxDistanceToBeConsideredAsClose) == false) continue;

                c.send(PLAYER_UPDATE,[player.toRelativeData(c.player)], isPlayerAction);
                if(sendFrame) c.send(FRAME, null, isPlayerAction);
            }

            for (ai in ais)
            {
                ai.playerUpdate(player);
            }
        }
        catch(ex) trace(ex);
    }

    public static function SendToMeAllClosePlayers(player:GlobalPlayerInstance, isPlayerAction:Bool = true)
        {
            try
            {
                player.MakeSureHoldObjIdAndDummyIsSetRightAndNullObjUsed(); // TODO better change, since it can mess with other threads
    
                for (c in connections)
                {
                    // since player has relative coordinates, transform them for player
                    var targetX = player.tx() - c.player.gx;
                    var targetY = player.ty() - c.player.gy;
    
                    // update only close players
                    if(c.player.isClose(targetX,targetY, ServerSettings.maxDistanceToBeConsideredAsClose) == false) continue;
    
                    player.connection.send(PLAYER_UPDATE,[c.player.toRelativeData(player)], isPlayerAction);
                }
                for (ai in ais)
                {
                    player.connection.send(PLAYER_UPDATE,[ai.player.toRelativeData(player)], true);
                }
                player.connection.send(FRAME, null, isPlayerAction);
            }
            catch(ex) trace(ex);
        }

    public static function SendTransitionUpdateToAllClosePlayers(player:GlobalPlayerInstance, tx:Int, ty:Int, newFloorId:Int, newTileObject:Array<Int>, doTransition:Bool, isPlayerAction:Bool = true)
    {
        try
        {
            for (c in connections) 
            {
                // since player has relative coordinates, transform them for player
                var targetX = tx - c.player.gx;
                var targetY = ty - c.player.gy;

                // update only close players
                if(c.player.isClose(targetX,targetY, ServerSettings.maxDistanceToBeConsideredAsClose) == false) continue;

                c.send(PLAYER_UPDATE,[player.toRelativeData(c.player)]);
                
                if(doTransition)
                {
                    c.sendMapUpdate(targetX, targetY, newFloorId, newTileObject, (-1) * player.p_id);
                }
                else
                {
                    c.sendMapUpdate(targetX, targetY, newFloorId, newTileObject, player.p_id);
                }
                
                c.send(FRAME);
            }

            for (ai in ais)
            {
                ai.mapUpdate(tx,ty);
            }
            
        } catch(ex) trace(ex);
    }

    public static function SendMapUpdateToAllClosePlayers(tx:Int, ty:Int, obj:Array<Int>)
    {    
        try
        {  
            var floorId = Server.server.map.getFloorId(tx,ty);

            for (c in connections)
            {
                // since player has relative coordinates, transform them for player
                var targetX = tx - c.player.gx;
                var targetY = ty - c.player.gy;

                // update only close players
                if(c.player.isClose(targetX,targetY, ServerSettings.maxDistanceToBeConsideredAsClose) == false) continue;
                
                try
                {
                    c.sendMapUpdate(targetX, targetY, floorId, obj, -1, false);
                    c.send(FRAME, null, false);
                }
                catch(ex)
                {
                    trace(ex);
                }
            }
        }
        catch(ex) trace(ex);
    }

    public static function SendAnimalMoveUpdateToAllClosePlayers(fromTx:Int, fromTy:Int, toTx:Int, toTy:Int, fromObj:Array<Int>, toObj:Array<Int>, speed:Float)
    {    
        try
        {  
            var floorIdTarget = Server.server.map.getFloorId(toTx, toTy);
            var floorIdFrom = Server.server.map.getFloorId(fromTx, fromTy);

            for (c in connections) 
            {            
                var player = c.player;
                
                // since player has relative coordinates, transform them for player
                var fromX = fromTx - player.gx;
                var fromY = fromTy - player.gy;
                var toX = toTx - player.gx;
                var toY = toTy - player.gy;

                // update only close players
                if(player.isClose(toX,toY, ServerSettings.maxDistanceToBeConsideredAsClose) == false && player.isClose(fromX,fromY, ServerSettings.maxDistanceToBeConsideredAsClose)) continue;

                c.mutex.acquire(); // do all in one frame

                c.sendMapUpdate(fromX, fromY, floorIdFrom, fromObj, -1, false);
                c.sendMapUpdateForMoving(toX, toY, floorIdTarget, toObj, -1, fromX, fromY, speed);
                c.send(FRAME, null, false);

                c.mutex.release();
            }

            for (ai in ais)
            {
                ai.mapUpdate(fromTx,fromTy,true);
            }
        }
        catch(ex) trace(ex);
    }

    public static function SendMoveUpdateToAllClosePlayers(player:GlobalPlayerInstance, totalMoveTime:Float, trunc:Int, moveString:String, isPlayerAction:Bool = true)
    {
        var eta = totalMoveTime; // TODO change???

        try
        {
            for (c in connections) 
            {
                // since player has relative coordinates, transform them for player
                var targetX = player.tx() - c.player.gx;
                var targetY = player.ty() - c.player.gy;

                // update only close players
                if(c.player.isClose(targetX,targetY, ServerSettings.maxDistanceToBeConsideredAsClose) == false) continue;

                c.send(PLAYER_MOVES_START,['${player.p_id} ${targetX} ${targetY} ${totalMoveTime} $eta ${trunc} ${moveString}']);
                
                c.send(FRAME);
            }
            for (ai in ais)
            {
                ai.playerMove(player,player.tx(),player.ty());
            }
        } 
        catch(ex) trace(ex);
    }

    private function addToConnections()
    {
        globalMutex.acquire();

        // it copies the connection array to be thread save 
        // other threads should meanwhile be able to iterate on connections. 
        var newConnections = [];

        newConnections.push(this);

        for(c in connections)
        {
            newConnections.push(c);
        }

        connections = newConnections; 

        globalMutex.release();
    }

    public function close()
    {
        this.mutex.acquire();

        globalMutex.acquire();

        try
        {
            // it copies the connection array to be thread save 
            // other threads should meanwhile be able to iterate on connections. replaces: //connections.remove(this);
            var newConnections = [];

            for(c in connections)
            {
                if(c == this) continue;

                newConnections.push(c);
            }

            connections = newConnections; 

            
            running = false;
            sock.close();
        }
        catch(ex)
        {
            trace(ex);
        }

        globalMutex.release();

        this.mutex.release();
    }

    // KA x y#
    public function keepAlive()
    {

    }

    // DIE x y#
    public function die()
    {
        this.close();
    }


    /**
        FL
        p_id face_left
        p_id face_left
        ...
        p_id face_left
        #

        Tells player about other players that have flipped.  face_left is true if facing
        left, false if facing right.  Only sent in response to stationary player flip
        requests (clients should still auto-flip players based on movement).
    **/
    public function flip(x:Int, y:Int)
    {
        for (c in connections) 
        {
            // since player has relative coordinates, transform them for player
            var targetX = player.tx() - c.player.gx;
            var targetY = player.ty() - c.player.gy;

            // update only close players
            if(c.player.isClose(targetX,targetY, ServerSettings.maxDistanceToBeConsideredAsClose) == false) continue;

            var face_left = x < player.x ? 'true' : 'false';
            c.send(FLIP,['${player.p_id} $face_left']);
        }
    }   

    public function sendMapChunk(x:Int,y:Int,width:Int = 32,height:Int = 30)
    {
        this.mutex.acquire();

        try
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
        catch(ex)
        {
            trace(ex);
        }

        this.mutex.release();
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
        this.mutex.acquire();

        try
        {
            send(MAP_CHANGE,['$x $y $newFloorId ${MapData.stringID(newObjectId)} $playerId'], isPlayerAction);        
        }
        catch(ex)
        {
            trace(ex);
        }

        this.mutex.release();
    }

    public function sendMapUpdateForMoving(toX:Int, toY:Int, newFloorId:Int, newObjectId:Array<Int>, playerId:Int, fromX:Int, fromY:Int, speed:Float)
    {
        this.mutex.acquire();

        try
        {
            send(MAP_CHANGE,['$toX $toY $newFloorId ${MapData.stringID(newObjectId)} $playerId $fromX $fromY $speed'], false);
        }
        catch(ex) trace(ex);

        this.mutex.release();
    }
    /**
        PE
        p_id emot_index ttl_sec
        p_id emot_index
        p_id emot_index ttl_sec
        ...
        p_id emot_index ttl_sec
        #
    **/
    public function emote(id:Int)
    {
        this.mutex.acquire();

        try
        {
            for (c in connections)
            {
                c.send(PLAYER_EMOT,['${player.p_id} $id']);
                c.send(FRAME);
            }
        }
        catch(ex) trace(ex);
        this.mutex.release();

        try
        {
            for (ai in ais)
            {
                ai.emote(player,id);
            }
        }catch(ex) trace(ex);
    }
    
    public function rlogin()
    {
        login(); // TODO reconnect
    }

    public function send(tag:ClientTag,data:Array<String>=null, isPlayerAction:Bool = true)
    {
        this.mutex.acquire();

        var string = "";

        //trace('send:  ${data}');

        try 
        {
            string = data != null ? '$tag\n${data.join("\n")}\n#' : '$tag\n#';
            sock.output.writeString(string);

            //if(ServerSettings.TraceSend && tag != MAP_CHANGE && tag != FRAME)
            if((ServerSettings.TraceSendPlayerActions && isPlayerAction) || (ServerSettings.TraceSendNonPlayerActions && isPlayerAction == false))
            {
                var tmpString = StringTools.replace(string, "\n", "\t");
                trace("Send: " + tmpString);
            }
        }
        catch(ex)
        {
            var tmpString = StringTools.replace(string, "\n", "\t");

            if('$ex' == 'Eof')
            {
                this.close();   
            }
            
            trace('WARNING Send: $tmpString ' + ex);
        }

        this.mutex.release();
    }

    public function sendPong(unique_id:String)
    {
        this.mutex.acquire();

        try
        {
            var tmpString = '$PONG\n$unique_id#';

            sock.output.writeString(tmpString);

            if(ServerSettings.TraceSendPlayerActions) trace("Send: " + tmpString);            
        }
        catch(ex)
        {
            trace(ex);
        }

        this.mutex.release();
    }

    public function sendGlobalMessage(message:String)
    {
        this.mutex.acquire();

        try
        {
            message  = StringTools.replace(message,' ', '_');
            send(ClientTag.GLOBAL_MESSAGE, [message]);
        }
        catch(ex)
        {
            trace(ex);
        }

        this.mutex.release();
    }
}
#end