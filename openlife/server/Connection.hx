package openlife.server;
import openlife.data.object.ObjectData;
import haxe.macro.Expr.Catch;
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

    public var serverAi:ServerAi; // null if connected to a client


    public var running:Bool = true;
    var sock:Socket;
    var server:Server;
    var tag:ServerTag;
    public var player:GlobalPlayerInstance;

    //public function get_player():GlobalPlayerInstance{return player;}

    // if it is an AI sock = null
    public function new(sock:Socket,server:Server)
    {
        this.sock = sock;
        this.server = server;
        var challenge = "dsdjsiojdiasjiodsjiosd";
        var version = ObjectData.dataVersionNumber;

        // if it is an AI there is no sock
        if(sock != null) send(SERVER_INFO,["0/0",challenge,'$version']);
    }

    public function login()
    {
        server.map.mutex.acquire();
        this.mutex.acquire();

        try
        {
            send(ACCEPTED);
                   
            this.player = new GlobalPlayerInstance();
            
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
            SendToMeAllClosePlayers(player, true); 
            sendToMeAllPlayerNames();
            sendToMeAllLineages();
            sendToMeAllFollowings();
            
            player.sendFoodUpdate();

            //this.send(ClientTag.LOCATION_SAYS,['0 100 ! 30']);

            send(FRAME, null, true);
        }
        catch(ex)
        {
            trace(ex.details);
        }

        this.mutex.release();
        server.map.mutex.release();
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

                // update only close players except if player is deleted (death)
                if(player.deleted == false && c.player.isClose(targetX,targetY, ServerSettings.maxDistanceToBeConsideredAsClose) == false) continue;

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

    public function sendToMeAllClosePlayers(sendMovingPlayer:Bool = false, isPlayerAction:Bool = true)
    {
        return SendToMeAllClosePlayers(this.player, sendMovingPlayer, isPlayerAction);
    }

    public static function SendToMeAllClosePlayers(player:GlobalPlayerInstance, sendMovingPlayer:Bool = false, isPlayerAction:Bool = true)
    {
        try
        {
            var connection = player.connection;

            //player.MakeSureHoldObjIdAndDummyIsSetRightAndNullObjUsed(); // TODO better change, since it can mess with other threads. A good idea maybe to use a global player mutex for all players

            for (c in connections)
            {
                connection.sendToMePlayerInfo(c.player, sendMovingPlayer, isPlayerAction);
            }

            for (ai in ais)
            {
                connection.sendToMePlayerInfo(ai.player, sendMovingPlayer, isPlayerAction);
            }

            player.connection.send(FRAME, null, isPlayerAction);
        }
        catch(ex) trace(ex);
    }

    private function sendToMePlayerInfo(playerToSend:GlobalPlayerInstance, sendMovingPlayer:Bool = false, isPlayerAction:Bool = true)
    {
        if(playerToSend.deleted) return;

        var player = this.player;

        // since player has relative coordinates, transform them for player
        var targetX = playerToSend.tx() - player.gx;
        var targetY = playerToSend.ty() - player.gy;

        // update only close players
        if(player.isClose(targetX,targetY, ServerSettings.maxDistanceToBeConsideredAsClose) == false)
        {
            player.connection.send(PLAYER_OUT_OF_RANGE,['${playerToSend.p_id}'], isPlayerAction);
            return;
        }
        
        if(playerToSend.isMoving())
        {        
            if(sendMovingPlayer)
            {
                player.connection.send(PLAYER_UPDATE,[playerToSend.toRelativeData(player)], isPlayerAction);
                var moveString = playerToSend.moveHelper.generateRelativeMoveUpdateString(player);
                player.connection.send(PLAYER_MOVES_START,[moveString]);
            }

            // sending moving player again creates otherwise a display bug
        }
        else
        {
            player.connection.send(PLAYER_UPDATE,[playerToSend.toRelativeData(player)], isPlayerAction);
        }
    }

    public function sendToMeAllPlayerNames()
    {
        for(c in Connection.getConnections())
        {
            var player = c.player;
            this.send(ClientTag.NAME,['${player.p_id} ${player.name} ${player.familyName}']);
        }

        for(c in ais)
        {
            var player = c.player;
            this.send(ClientTag.NAME,['${player.p_id} ${player.name} ${player.familyName}']);
        }
    }

    // fathers are not supported by client 
    public function sendToMeAllLineages()
    {
        for(c in Connection.getConnections())
        {
            var lineageString = c.player.lineage.createLineageString();
            this.send(ClientTag.LINEAGE,[lineageString]);
        }

        for(c in ais)
        {
            var lineageString = c.player.lineage.createLineageString();
            this.send(ClientTag.LINEAGE,[lineageString]);
        }
    }

    /* FOLLOWING (FW): follower_id leader_id leader_color_index
    Provides list of people following other people.
    If leader is -1, that person follows no one
    Leader color index specifies leader's badge color from a fixed color list*/
    public function sendFollowing(player:GlobalPlayerInstance)
    {
        var leader = player.getTopLeader();
        //var leaderId = leader == null ? -1 : leader.p_id; // TODO not sure if client wants top leader or next leader
        var leaderId = player.followPlayer == null ? -1 : player.followPlayer.p_id;
        var leaderBadgeColor = leader == null ? player.leaderBadgeColor : leader.leaderBadgeColor;

        send(FOLLOWING,['${player.p_id} $leaderId $leaderBadgeColor']);
    }

    public static function SendFollowingToAll(player:GlobalPlayerInstance)
    {
        for(c in Connection.getConnections())
        {
            c.sendFollowing(player);
        }
    }

    public function sendToMeAllFollowings()
    {
        for(c in Connection.getConnections())
        {
            sendFollowing(c.player);
        }

        for(c in ais)
        {
            sendFollowing(c.player);
        }
    }
    /**
                        EX
                        exile_target_id exiler_id
                        exile_target_id exiler_id
                        exile_target_id exiler_id
                        ...
                        exile_target_id exiler_id
                        #

                        Provides list of people exiled by another person.

                        If someone's exile list has changed (who's exiling them), then their whole
                        exile list is sent.

                        Each target's list is prefaced by this line:

                        exile_target_id -1

                        This indicates that the target's complete list is coming, and the client-side
                        list should be cleared in preparation for this.


                        If a target is newly exiled by no one, they show up as:

                        exile_target_id -1
**/
    public static function SendExileToAll(exiler:GlobalPlayerInstance, target:GlobalPlayerInstance)
    {
        for(c in Connection.getConnections())
        {
            c.send(EXILED,['${target.p_id} ${exiler.p_id}']);
        }
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

            for (c in ais)
            {
                // since player has relative coordinates, transform them for player
                var targetX = tx - c.player.gx;
                var targetY = ty - c.player.gy;

                // update only close players
                if(c.player.isClose(targetX,targetY, ServerSettings.maxDistanceToBeConsideredAsClose) == false) continue;

                c.mapUpdate(tx,ty);
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

            for (c in ais)
            {
                var player = c.player;
                
                // since player has relative coordinates, transform them for player
                var fromX = fromTx - player.gx;
                var fromY = fromTy - player.gy;
                var toX = toTx - player.gx;
                var toY = toTy - player.gy;

                // update only close players
                if(player.isClose(toX,toY, ServerSettings.maxDistanceToBeConsideredAsClose) == false && player.isClose(fromX,fromY, ServerSettings.maxDistanceToBeConsideredAsClose)) continue;

                c.mapUpdate(fromTx,fromTy,true);
            }
        }
        catch(ex) trace(ex);
    }

    public static function SendMoveUpdateToAllClosePlayers(player:GlobalPlayerInstance, isPlayerAction:Bool = true)
    {
        try
        {
            for (c in connections) 
            {
                // since player has relative coordinates, transform them for player
                var targetX = player.tx() - c.player.gx;
                var targetY = player.ty() - c.player.gy;

                // update only close players
                if(c.player.isClose(targetX,targetY, ServerSettings.maxDistanceToBeConsideredAsClose) == false) continue;

                var moveString = player.moveHelper.generateRelativeMoveUpdateString(c.player);

                c.send(PLAYER_MOVES_START,[moveString]);

                //c.send(PLAYER_MOVES_START,['${player.p_id} ${targetX} ${targetY} ${totalMoveTime} $eta ${trunc} ${moveString}']);
                
                c.send(FRAME);
            }

            for (c in ais)
            {
                // since player has relative coordinates, transform them for player
                var targetX = player.tx() - c.player.gx;
                var targetY = player.ty() - c.player.gy;

                // update only close players
                if(c.player.isClose(targetX,targetY, ServerSettings.maxDistanceToBeConsideredAsClose) == false) continue;

                c.playerMove(player,player.tx(),player.ty());
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
            // set all stuff null so that nothing is hanging around
            this.player.delete();

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
        if(serverAi != null) return;

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
        if(serverAi != null) return;

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
        if(serverAi != null) return;

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
                ttl_sec is optional, and specifies how long the emote should be shown, in
                seconds, client-side.  If it is omitted, the client should display the emotion
                for the standard amount of time.  If ttl is -1, this emot is permanent and
                should layer with other permanent and non-permanent emots.

                If ttl is -2, the emot is permanent but not new, so sound shoudl be skipped.
    **/
    public function emote(id:Int, seconds:Int = -10)
    {
        this.mutex.acquire();

        try
        {
            for (c in connections)
            {

                // since player has relative coordinates, transform them for player
                var targetX = player.tx() - c.player.gx;
                var targetY = player.ty() - c.player.gy;

                // update only close players
                if(c.player.isClose(targetX,targetY, ServerSettings.maxDistanceToBeConsideredAsClose) == false) continue;

                if(seconds < 3)c.send(PLAYER_EMOT,['${player.p_id} $id']);
                else c.send(PLAYER_EMOT,['${player.p_id} $id $seconds']);
                c.send(FRAME);
            }
        }
        catch(ex) trace(ex);

        this.mutex.release();

        try
        {
            for (c in ais)
            {
                // since player has relative coordinates, transform them for player
                var targetX = player.tx() - c.player.gx;
                var targetY = player.ty() - c.player.gy;

                // update only close players
                if(c.player.isClose(targetX,targetY, ServerSettings.maxDistanceToBeConsideredAsClose) == false) continue;

                c.emote(player,id);
            }
        }
        catch(ex) trace(ex);
    }
    
    public function rlogin()
    {
        login(); // TODO reconnect
    }

    public function send(tag:ClientTag,data:Array<String>=null, isPlayerAction:Bool = true)
    {
        this.mutex.acquire();

        if(serverAi != null)
        {
            try{
                // TODO call serverAi.
            } catch(ex)
            {
                trace(ex);
            }

            this.mutex.release();

            return;
        } 

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

    /**
    BABY_WIGGLE (BW): p_id
    A list of player IDs that are babies who just started wiggling.
    **/
    public function sendWiggle(player:GlobalPlayerInstance)
    {
        send(BABY_WIGGLE,['${player.p_id}'], true);
    } 

    /**( 
                                        "PS\n"
                                        "%d/0 OUTSIDER %s IS MY NEW FOLLOWER "
                                        "*visitor %d *map %d %d\n#",
                                        otherToFollow->id,
                                        name,
                                        nextPlayer->id,
                                        nextPlayer->xs - 
                                        otherToFollow->birthPos.x,
                                        nextPlayer->ys - 
                                        otherToFollow->birthPos.y );
    **/

    public function sendMapLocation(toPlayer:GlobalPlayerInstance, text1:String, text2:String)
    {
        var player = this.player;
        var message = '${player.p_id}/0 $text1 *$text2 ${toPlayer.p_id} *map ${toPlayer.tx() - player.gx} ${toPlayer.ty() - player.gy}';

        //trace('MAPSAY: $message');

        this.send(ClientTag.PLAYER_SAYS, [message], true);
    }

    public function sendLeader()
    {
        var player = this.player;
        var leader = player.getTopLeader();
        
        this.sendMapLocation(leader, "LEADER", "leader");
    }
}
#end