package states.game;

import data.MapData;
import data.MapData.MapInstance;
import data.PlayerData.PlayerInstance;
import data.PlayerData.PlayerMove;
import data.GameData;
import client.MessageTag;
import haxe.io.Bytes;

class Game extends states.State
{
    var dialog:Dialog;
    var ground:Ground;
    var objects:Objects;
    var player:Player;
    var playerInstance:PlayerInstance;
    var mapInstance:MapInstance;
    var index:Int = 0;
    public var data:GameData;
    public function new()
    {
        super();
        data = new GameData();
        ground = new Ground(this);
        objects = new Objects(this);
        dialog = new Dialog(this);
        addChild(ground);
        addChild(objects);
        addChild(dialog);
    }
    override function update() {
        super.update();
    }
    override function message(input:String, tag:MessageTag) {
        super.message(input, tag);
        switch(tag)
        {
            case PLAYER_UPDATE:
            var array = input.split(" ");
            playerInstance = new PlayerInstance(input.split(" "));
            case PLAYER_MOVES_START:
            var playerMove = new PlayerMove(input.split(" "));
            if (data.playerMap.exists(playerMove.id))
            {
                
            }
            //p_id xs ys total_sec eta_sec trunc xdelt0 ydelt0
            //264 0 -1 0.503 0.503 0 1 1
            case MAP_CHUNK:
            var array = input.split(" ");
            //trace("map chunk array " + array);
            for(value in array)
            {
                switch(index++)
                {
                    case 0:
                    mapInstance = new MapInstance();
                    mapInstance.sizeX = Std.parseInt(value);
                    case 1:
                    mapInstance.sizeY = Std.parseInt(value);
                    case 2:
                    mapInstance.x = Std.parseInt(value);
                    case 3:
                    mapInstance.y = Std.parseInt(value);
                    case 4:
                    mapInstance.rawSize = Std.parseInt(value);
                    case 5:
                    mapInstance.compressedSize = Std.parseInt(value);
                    mapInstance.bytes = Bytes.alloc(mapInstance.compressedSize);
                    data.map.setX = mapInstance.x;
                    data.map.setY = mapInstance.y;
                    data.map.setWidth = mapInstance.sizeX;
                    data.map.setHeight = mapInstance.sizeY;
                    trace("map chunk " + mapInstance.toString());
                    index = 0;
                    Main.client.compress = mapInstance.compressedSize;
                }
            }
            case MAP_CHANGE:
            //x y new_floor_id new_id p_id optional oldX oldY speed
            var array = input.split(" ");
            var mapChange = new MapChange();
            mapChange.x = Std.parseInt(array[0]);
            mapChange.y = Std.parseInt(array[1]);
            var string = mapChange.x + "." + mapChange.y;
            //floor
            mapChange.floor = Std.parseInt(array[2]);
            data.map.floor.set(string,mapChange.floor);
            
            //object
            mapChange.id = Std.parseInt(array[3]);
            data.map.object.set(string,Std.string(mapChange.id));
            
            //p_id 4
            /*mapChange.pid = Std.parseInt(array[4]);
            if(mapChange.pid == -1)
            {

                //change no triggered by player
                var object = Main.display.objectMap.get(string);
                if(object != null)
                {
                    trace("object " + object);
                    for (child in object.children)
                    {
                        Main.display.removeTile(child);
                    }
                    Main.display.objectMap.remove(string);
                    object = null;
                }
            }else{
                trace("trigger");
                //triggered by player
                if(mapChange.pid < -1)
                {
                    //object was not dropped
                }else{
                    //object was dropped
                    trace("drop by " + mapChange.pid);
                    var player = Player.active.get(mapChange.pid);
                    //player.pid
                }
            }*/
            //optional speed
            if(array.length > 4)
            {
                var old = array[5] + "." + array[6];
                var speed = array[7];
            }
            case HEAT_CHANGE:
            //trace("heat " + input);

            case FOOD_CHANGE:
            //trace("food change " + input);
            //also need to set new movement move_speed: is floating point speed in grid square widths per second.
            case FRAME:
            tag = "";
            case PLAYER_SAYS:
            trace("player say " + input);
            dialog.say(input);
            case PLAYER_OUT_OF_RANGE:
            //player is out of range

            case LINEAGE:
            //p_id mother_id grandmother_id great_grandmother_id ... eve_id eve=eve_id

            case NAME:
            //p_id first_name last_name last_name may be ommitted.

            case DYING:
            //p_id isSick isSick is optional 1 flag to indicate that player is sick (client shouldn't show blood UI overlay for sick players)

            case HEALED:
            //p_id player healed no longer dying.

            case MONUMENT_CALL:
            //MN x y o_id monument call has happened at location x,y with the creation object id

            case GRAVE:
            //x y p_id

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

            case FLIGHT_DEST:
            //p_id dest_x dest_y
            trace("FLIGHT FLIGHT FLIGHT " + input.split(" "));
            default:
        }
    }
}