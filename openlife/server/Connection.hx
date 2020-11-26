package openlife.server;
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
        var id = server.index++;
        player.p_id = id;
        player.gx = 360;
        player.gy = 600 - 400; // server map is saved y inverse 
        player.move_speed = server.map.getBiomeSpeed(player.gx, player.gy) * PlayerInstance.initial_move_speed;

        Server.server.map.setObjectId(player.gx, player.gy, [33]);
        Server.server.map.setObjectId(player.gx+1, player.gy, [32]);
        Server.server.map.setObjectId(player.gx+2, player.gy, [486]);
        Server.server.map.setObjectId(player.gx+3, player.gy, [486]);
        Server.server.map.setObjectId(player.gx+4, player.gy, [677]);
        Server.server.map.setObjectId(player.gx+5, player.gy, [684]);
        Server.server.map.setObjectId(player.gx+6, player.gy, [677]);

        // add some clothing for testing
        Server.server.map.setObjectId(player.gx, player.gy+1, [2916]);
        Server.server.map.setObjectId(player.gx+1, player.gy+1, [2456]);
        Server.server.map.setObjectId(player.gx+2, player.gy+1, [766]);
        Server.server.map.setObjectId(player.gx+3, player.gy+1, [2919]);
        Server.server.map.setObjectId(player.gx+4, player.gy+1, [198]);
        Server.server.map.setObjectId(player.gx+5, player.gy+1, [2886]);
        Server.server.map.setObjectId(player.gx+6, player.gy+1, [586]);
        Server.server.map.setObjectId(player.gx+7, player.gy+1, [2951]);

        // test time / decay transitions
        Server.server.map.setObjectId(player.gx - 4,player.gy + 5,[248]);
        Server.server.map.setObjectId(player.gx - 5,player.gy + 5,[82]);
        Server.server.map.setObjectId(player.gx - 6,player.gy + 5,[418]);

        //test transitions of numUses + decay
        Server.server.map.setObjectId(player.gx,player.gy + 10,[238]);
        Server.server.map.setObjectId(player.gx,player.gy + 11,[1599]);

        //containers testing SREMV
        Server.server.map.setObjectId(player.gx - 4,player.gy + 10,[434]);
        Server.server.map.setObjectId(player.gx - 5,player.gy + 10,[292,2143,2143,2143]);
        Server.server.map.setObjectId(player.gx - 6,player.gy + 10,[292,2143,2143,2143]);
        Server.server.map.setObjectId(player.gx - 7,player.gy + 10,[292,33,2143,33]);
        Server.server.map.setObjectId(player.gx - 8,player.gy + 10,[2143,2143,2143]);
        Server.server.map.setObjectId(player.gx - 7,player.gy + 10,[3371,33,2143,33]);
        


        
        server.map.mutex.release();
        
        trace("move_speed: " + player.move_speed);

        sendMapChunk(0,0);

        var data:Array<String> = [];
        for (c in server.connections)
        {
            data.push(c.player.toData());
            if (c != this)
            {
                c.send(PLAYER_UPDATE,[player.toData()]);
                c.send(FRAME);
            }
        }
        send(PLAYER_UPDATE,data);
        send(FRAME);
        send(LINEAGE,['$id eve=$id']);

        
    }

    /*
    public function update()
    {
        player.handleUpdate();


        //if(server.tick % 20 == 0){
            //trace("Ticks: " + server.tick);
            // TODO needs to calculate the player position first
            // player.sendSpeedUpdate(this);
            //this.send(FRAME);
        //}
    }
    */

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
    public function sendMapUpdate(x:Int, y:Int, newFloorId:Int, newObjectId:Array<Int>, playerId:Int)
    {
        send(MAP_CHANGE,['$x $y $newFloorId ${MapData.stringID(newObjectId)} $playerId']);
    }

    public function sendMapUpdateForMoving(toX:Int, toY:Int, newFloorId:Int, newObjectId:Array<Int>, playerId:Int, fromX:Int, fromY:Int, speed:Float)
    {
        send(MAP_CHANGE,['$toX $toY $newFloorId ${MapData.stringID(newObjectId)} $playerId $fromX $fromY $speed']);
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

    public function send(tag:ClientTag,data:Array<String>=null)
    {
        var string = data != null ? '$tag\n${data.join("\n")}\n#' : '$tag\n#';
        //trace("S: " + string);
        sock.output.writeString(string);
    }
}
#end